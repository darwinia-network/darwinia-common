//! Prototype module for cross chain assets backing.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test_with_linear_relay;
#[cfg(test)]
mod test_with_relay;

mod types {
	use crate::*;

	pub type Balance = u128;
	pub type DepositId = U256;

	pub type RingBalance<T> =
		<<T as Trait>::RingCurrency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
	#[cfg(feature = "std")]
	pub type KtonBalance<T> =
		<<T as Trait>::KtonCurrency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

	pub type EthereumReceiptProofThing<T> = <<T as Trait>::EthereumRelay as EthereumReceipt<
		<T as frame_system::Trait>::AccountId,
		RingBalance<T>,
	>>::EthereumReceiptProofThing;
}

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethabi::{Event as EthEvent, EventParam as EthEventParam, ParamType, RawLog};
// --- substrate ---
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement::KeepAlive, Get},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
	traits::{AccountIdConversion, SaturatedConversion, Saturating},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{convert::TryFrom, vec};
// --- darwinia ---
use array_bytes::array_unchecked;
use darwinia_support::{
	balance::lock::*,
	traits::{EthereumReceipt, OnDepositRedeem},
};
use ethereum_primitives::{receipt::EthereumTransactionIndex, EthereumAddress, U256};
use types::*;

pub trait Trait: frame_system::Trait {
	/// The backing's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type RedeemAccountId: From<[u8; 32]> + Into<Self::AccountId>;

	type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;

	type OnDepositRedeem: OnDepositRedeem<Self::AccountId, RingBalance<Self>>;

	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type KtonCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	/// Weight information for the extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
		RingBalance = RingBalance<T>,
	{
		/// Some one redeem some *RING*. [account, amount, transaction index]
		RedeemRing(AccountId, Balance, EthereumTransactionIndex),
		/// Some one redeem some *KTON*. [account, amount, transaction index]
		RedeemKton(AccountId, Balance, EthereumTransactionIndex),
		/// Some one redeem a deposit. [account, deposit id, amount, transaction index]
		RedeemDeposit(AccountId, DepositId, RingBalance, EthereumTransactionIndex),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Address Length - MISMATCHED
		AddrLenMis,
		/// Pubkey Prefix - MISMATCHED
		PubkeyPrefixMis,

		/// Bytes - CONVERSION FAILED
		BytesCF,
		/// Int - CONVERSION FAILED
		IntCF,

		/// Address - CONVERSION FAILED
		AddressCF,

		/// Asset - ALREADY REDEEMED
		AssetAR,

		/// EthereumReceipt Proof - INVALID
		ReceiptProofI,

		/// Eth Log - PARSING FAILED
		EthLogPF,

		/// *KTON* Locked - NO SUFFICIENT BACKING ASSETS
		KtonLockedNSBA,
		/// *RING* Locked - NO SUFFICIENT BACKING ASSETS
		RingLockedNSBA,

		/// Log Entry - NOT EXISTED
		LogEntryNE,

		/// Usable Balance for Paying Redeem Fee - NOT ENOUGH
		FeeNE,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthereumBacking {
		pub VerifiedProof
			get(fn verified_proof)
			: map hasher(blake2_128_concat) EthereumTransactionIndex => Option<bool>;

		pub TokenRedeemAddress get(fn token_redeem_address) config(): EthereumAddress;
		pub DepositRedeemAddress get(fn deposit_redeem_address) config(): EthereumAddress;

		pub RingTokenAddress get(fn ring_token_address) config(): EthereumAddress;
		pub KtonTokenAddress get(fn kton_token_address) config(): EthereumAddress;
	}
	add_extra_genesis {
		config(ring_locked): RingBalance<T>;
		config(kton_locked): KtonBalance<T>;
		build(|config: &GenesisConfig<T>| {
			// Create Backing account
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance().max(config.ring_locked),
			);

			let _ = T::KtonCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::KtonCurrency::minimum_balance().max(config.kton_locked),
			);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		/// The treasury's module id, used for deriving its sovereign account ID.
		const ModuleId: ModuleId = T::ModuleId::get();

		fn deposit_event() = default;

		/// Redeem balances
		///
		/// # <weight>
		/// - `O(1)`
		/// # </weight>
		#[weight = 10_000_000]
		pub fn redeem(origin, act: RedeemFor, proof: EthereumReceiptProofThing<T>) {
			let redeemer = ensure_signed(origin)?;

			match act {
				RedeemFor::Token => Self::redeem_token(&redeemer, &proof)?,
				RedeemFor::Deposit => Self::redeem_deposit(&redeemer, &proof)?,
			}
		}

		// --- Root Call ---

		/// Set a new ring redeem address.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// - `new`: The new ring redeem address.
		///
		/// # <weight>
		/// - `O(1)`.
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_token_redeem_address(origin, new: EthereumAddress) {
			ensure_root(origin)?;
			TokenRedeemAddress::put(new);
		}

		/// Set a new deposit redeem address.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// - `new`: The new deposit redeem address.
		///
		/// # <weight>
		/// - `O(1)`.
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_deposit_redeem_address(origin, new: EthereumAddress) {
			ensure_root(origin)?;
			DepositRedeemAddress::put(new);
		}
	}
}

impl<T: Trait> Module<T> {
	// --- Immutable ---

	/// The account ID of the backing pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn account_id_try_from_bytes(bytes: &[u8]) -> Result<T::AccountId, DispatchError> {
		ensure!(bytes.len() == 32, <Error<T>>::AddrLenMis);

		let redeem_account_id: T::RedeemAccountId = array_unchecked!(bytes, 0, 32).into();

		Ok(redeem_account_id.into())
	}

	/// Return the amount of money in the pot.
	// The existential deposit is not part of the pot so backing account never gets deleted.
	fn pot<C: LockableCurrency<T::AccountId>>() -> C::Balance {
		C::usable_balance(&Self::account_id())
			// Must never be less than 0 but better be safe.
			.saturating_sub(C::minimum_balance())
	}

	// event BurnAndRedeem(address indexed token, address indexed from, uint256 amount, bytes receiver);
	// Redeem RING https://ropsten.etherscan.io/tx/0x1d3ef601b9fa4a7f1d6259c658d0a10c77940fa5db9e10ab55397eb0ce88807d
	// Redeem KTON https://ropsten.etherscan.io/tx/0x2878ae39a9e0db95e61164528bb1ec8684be194bdcc236848ff14d3fe5ba335d
	fn parse_token_redeem_proof(
		proof_record: &EthereumReceiptProofThing<T>,
	) -> Result<(T::AccountId, (bool, Balance), RingBalance<T>), DispatchError> {
		let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)
			.map_err(|_| <Error<T>>::ReceiptProofI)?;
		let fee = T::EthereumRelay::receipt_verify_fee();
		let result = {
			let eth_event = EthEvent {
				name: "BurnAndRedeem".to_owned(),
				inputs: vec![
					EthEventParam {
						name: "token".to_owned(),
						kind: ParamType::Address,
						indexed: true,
					},
					EthEventParam {
						name: "from".to_owned(),
						kind: ParamType::Address,
						indexed: true,
					},
					EthEventParam {
						name: "amount".to_owned(),
						kind: ParamType::Uint(256),
						indexed: false,
					},
					EthEventParam {
						name: "receiver".to_owned(),
						kind: ParamType::Bytes,
						indexed: false,
					},
				],
				anonymous: false,
			};
			let log_entry = verified_receipt
				.logs
				.into_iter()
				.find(|x| {
					x.address == Self::token_redeem_address()
						&& x.topics[0] == eth_event.signature()
				})
				.ok_or(<Error<T>>::LogEntryNE)?;
			let log = RawLog {
				topics: vec![
					log_entry.topics[0],
					log_entry.topics[1],
					log_entry.topics[2],
				],
				data: log_entry.data.clone(),
			};

			eth_event.parse_log(log).map_err(|_| <Error<T>>::EthLogPF)?
		};
		let is_ring = {
			let token_address = result.params[0]
				.value
				.clone()
				.to_address()
				.ok_or(<Error<T>>::AddressCF)?;

			ensure!(
				token_address == Self::ring_token_address()
					|| token_address == Self::kton_token_address(),
				<Error<T>>::AssetAR
			);

			token_address == Self::ring_token_address()
		};

		let redeemed_amount = {
			// TODO: div 10**18 and mul 10**9
			let amount = result.params[2]
				.value
				.clone()
				.to_uint()
				.map(|x| x / U256::from(1_000_000_000u64))
				.ok_or(<Error<T>>::IntCF)?;

			Balance::try_from(amount)?
		};
		let darwinia_account = {
			let raw_subkey = result.params[3]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;
			debug::trace!(target: "ethereum-backing", "[ethereum-backing] Raw Subkey: {:?}", raw_subkey);

			Self::account_id_try_from_bytes(&raw_subkey)?
		};
		debug::trace!(target: "ethereum-backing", "[ethereum-backing] Darwinia Account: {:?}", darwinia_account);

		Ok((darwinia_account, (is_ring, redeemed_amount), fee))
	}

	// event BurnAndRedeem(uint256 indexed _depositID,  address _depositor, uint48 _months, uint48 _startAt, uint64 _unitInterest, uint128 _value, bytes _data);
	// Redeem Deposit https://ropsten.etherscan.io/tx/0x5a7004126466ce763501c89bcbb98d14f3c328c4b310b1976a38be1183d91919
	fn parse_deposit_redeem_proof(
		proof_record: &EthereumReceiptProofThing<T>,
	) -> Result<
		(
			DepositId,
			T::AccountId,
			RingBalance<T>,
			u64,
			u8,
			RingBalance<T>,
		),
		DispatchError,
	> {
		let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)
			.map_err(|_| <Error<T>>::ReceiptProofI)?;
		let fee = T::EthereumRelay::receipt_verify_fee();
		let result = {
			let eth_event = EthEvent {
				name: "BurnAndRedeem".to_owned(),
				inputs: vec![
					EthEventParam {
						name: "_depositID".to_owned(),
						kind: ParamType::Uint(256),
						indexed: true,
					},
					EthEventParam {
						name: "_depositor".to_owned(),
						kind: ParamType::Address,
						indexed: false,
					},
					EthEventParam {
						name: "_months".to_owned(),
						kind: ParamType::Uint(48),
						indexed: false,
					},
					EthEventParam {
						name: "_startAt".to_owned(),
						kind: ParamType::Uint(48),
						indexed: false,
					},
					EthEventParam {
						name: "_unitInterest".to_owned(),
						kind: ParamType::Uint(64),
						indexed: false,
					},
					EthEventParam {
						name: "_value".to_owned(),
						kind: ParamType::Uint(128),
						indexed: false,
					},
					EthEventParam {
						name: "_data".to_owned(),
						kind: ParamType::Bytes,
						indexed: false,
					},
				],
				anonymous: false,
			};
			let log_entry = verified_receipt
				.logs
				.iter()
				.find(|&x| {
					x.address == Self::deposit_redeem_address()
						&& x.topics[0] == eth_event.signature()
				})
				.ok_or(<Error<T>>::LogEntryNE)?;
			let log = RawLog {
				topics: vec![log_entry.topics[0], log_entry.topics[1]],
				data: log_entry.data.clone(),
			};

			eth_event.parse_log(log).map_err(|_| <Error<T>>::EthLogPF)?
		};
		let deposit_id = result.params[0]
			.value
			.clone()
			.to_uint()
			.ok_or(<Error<T>>::IntCF)?;
		let month = {
			let month = result.params[2]
				.value
				.clone()
				.to_uint()
				.ok_or(<Error<T>>::IntCF)?;

			month.saturated_into()
		};
		// The start_at here is in seconds, will be converted to milliseconds later in on_deposit_redeem
		let start_at = {
			let start_at = result.params[3]
				.value
				.clone()
				.to_uint()
				.ok_or(<Error<T>>::IntCF)?;

			start_at.saturated_into()
		};
		let redeemed_ring = {
			// The decimal in Ethereum is 10**18, and the decimal in Darwinia is 10**9,
			// div 10**18 and mul 10**9
			let redeemed_ring = result.params[5]
				.value
				.clone()
				.to_uint()
				.map(|x| x / U256::from(1_000_000_000u64))
				.ok_or(<Error<T>>::IntCF)?;

			<RingBalance<T>>::saturated_from(redeemed_ring.saturated_into())
		};
		let darwinia_account = {
			let raw_subkey = result.params[6]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;
			debug::trace!(target: "ethereum-backing", "[ethereum-backing] Raw Subkey: {:?}", raw_subkey);

			Self::account_id_try_from_bytes(&raw_subkey)?
		};
		debug::trace!(target: "ethereum-backing", "[ethereum-backing] Darwinia Account: {:?}", darwinia_account);

		Ok((
			deposit_id,
			darwinia_account,
			redeemed_ring,
			start_at,
			month,
			fee,
		))
	}

	// --- Mutable ---

	fn redeem_token(
		redeemer: &T::AccountId,
		proof: &EthereumReceiptProofThing<T>,
	) -> DispatchResult {
		let tx_index = T::EthereumRelay::gen_receipt_index(proof);

		ensure!(!VerifiedProof::contains_key(tx_index), <Error<T>>::AssetAR);

		let (darwinia_account, (is_ring, redeem_amount), fee) =
			Self::parse_token_redeem_proof(&proof)?;

		if is_ring {
			Self::redeem_token_cast::<T::RingCurrency>(
				redeemer,
				darwinia_account,
				tx_index,
				true,
				redeem_amount,
				fee,
			)?;
		} else {
			Self::redeem_token_cast::<T::KtonCurrency>(
				redeemer,
				darwinia_account,
				tx_index,
				false,
				redeem_amount,
				fee,
			)?;
		}

		Ok(())
	}

	fn redeem_token_cast<C: LockableCurrency<T::AccountId>>(
		redeemer: &T::AccountId,
		darwinia_account: T::AccountId,
		tx_index: EthereumTransactionIndex,
		is_ring: bool,
		redeem_amount: Balance,
		fee: RingBalance<T>,
	) -> DispatchResult {
		let raw_amount = redeem_amount;
		let redeem_amount: C::Balance = redeem_amount.saturated_into();

		ensure!(
			Self::pot::<C>() >= redeem_amount,
			if is_ring {
				<Error<T>>::RingLockedNSBA
			} else {
				<Error<T>>::KtonLockedNSBA
			}
		);
		// Checking redeemer have enough of balance to pay fee, make sure follow up transfer will success.
		ensure!(
			T::RingCurrency::usable_balance(redeemer) >= fee,
			<Error<T>>::FeeNE
		);

		C::transfer(
			&Self::account_id(),
			&darwinia_account,
			redeem_amount,
			KeepAlive,
		)?;
		// Transfer the fee from redeemer.
		T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee, KeepAlive)?;

		VerifiedProof::insert(tx_index, true);

		if is_ring {
			Self::deposit_event(RawEvent::RedeemRing(darwinia_account, raw_amount, tx_index));
		} else {
			Self::deposit_event(RawEvent::RedeemKton(darwinia_account, raw_amount, tx_index));
		}

		Ok(())
	}

	fn redeem_deposit(
		redeemer: &T::AccountId,
		proof: &EthereumReceiptProofThing<T>,
	) -> DispatchResult {
		let tx_index = T::EthereumRelay::gen_receipt_index(proof);

		ensure!(!VerifiedProof::contains_key(tx_index), <Error<T>>::AssetAR);

		let (deposit_id, darwinia_account, redeemed_ring, start_at, month, fee) =
			Self::parse_deposit_redeem_proof(&proof)?;

		ensure!(
			Self::pot::<T::RingCurrency>() >= redeemed_ring,
			<Error<T>>::RingLockedNSBA
		);
		// Checking redeemer have enough of balance to pay fee, make sure follow up fee transfer will success.
		ensure!(
			T::RingCurrency::usable_balance(redeemer) >= fee,
			<Error<T>>::FeeNE
		);

		T::OnDepositRedeem::on_deposit_redeem(
			&Self::account_id(),
			start_at,
			month,
			redeemed_ring,
			&darwinia_account,
		)?;
		// Transfer the fee from redeemer.
		T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee, KeepAlive)?;

		// TODO: check deposit_id duplication
		// TODO: Ignore Unit Interest for now
		VerifiedProof::insert(tx_index, true);

		<Module<T>>::deposit_event(RawEvent::RedeemDeposit(
			darwinia_account,
			deposit_id,
			redeemed_ring,
			tx_index,
		));

		Ok(())
	}
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RedeemFor {
	Token,
	Deposit,
}
