//! Prototype module for cross chain assets backing.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(all(feature = "std", test))]
mod mock;
#[cfg(all(feature = "std", test))]
mod tests;

mod types {
	use crate::*;

	/// Balance of an account.
	pub type Balance = u128;
	pub type DepositId = U256;

	pub type MomentT<T> = <TimeT<T> as Time>::Moment;

	pub type RingBalance<T> =
		<<T as Trait>::Ring as Currency<<T as system::Trait>::AccountId>>::Balance;
	pub type RingPositiveImbalance<T> =
		<<T as Trait>::Ring as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;

	pub type KtonBalance<T> =
		<<T as Trait>::Kton as Currency<<T as system::Trait>::AccountId>>::Balance;
	pub type KtonPositiveImbalance<T> =
		<<T as Trait>::Kton as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;

	pub type EthTransactionIndex = (H256, u64);

	type TimeT<T> = <T as Trait>::Time;
}

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethabi::{Event as EthEvent, EventParam as EthEventParam, ParamType, RawLog};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, Get, OnUnbalanced, Time},
	weights::SimpleDispatchInfo,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
	traits::{CheckedSub, SaturatedConversion},
	DispatchError, DispatchResult, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{convert::TryFrom, marker::PhantomData, vec};
// --- darwinia ---
use darwinia_eth_relay::{EthReceiptProof, VerifyEthReceipts};
use darwinia_support::traits::{LockableCurrency, OnDepositRedeem};
use eth_primitives::{EthAddress, H256, U256};
use types::*;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	type Time: Time;

	type DetermineAccountId: AccountIdFor<Self::AccountId>;

	type EthRelay: VerifyEthReceipts;

	type OnDepositRedeem: OnDepositRedeem<
		Self::AccountId,
		Balance = RingBalance<Self>,
		Moment = MomentT<Self>,
	>;

	type Ring: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	type RingReward: OnUnbalanced<RingPositiveImbalance<Self>>;

	type Kton: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	type KtonReward: OnUnbalanced<KtonPositiveImbalance<Self>>;

	type SubKeyPrefix: Get<u8>;
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RedeemFor {
	Ring(EthReceiptProof),
	Kton(EthReceiptProof),
	Deposit(EthReceiptProof),
}

decl_storage! {
	trait Store for Module<T: Trait> as EthBacking {
		pub RingLocked get(fn ring_locked) config(): RingBalance<T>;
		pub RingProofVerified
			get(fn ring_proof_verfied)
			: map hasher(blake2_128_concat) EthTransactionIndex => Option<EthReceiptProof>;
		pub RingRedeemAddress get(fn ring_redeem_address) config(): EthAddress;

		pub KtonLocked get(fn kton_locked) config(): KtonBalance<T>;
		pub KtonProofVerified
			get(fn kton_proof_verfied)
			: map hasher(blake2_128_concat) EthTransactionIndex => Option<EthReceiptProof>;
		pub KtonRedeemAddress get(fn kton_redeem_address) config(): EthAddress;

		pub DepositProofVerified
			 get(fn deposit_proof_verfied)
			 : map hasher(blake2_128_concat) EthTransactionIndex => Option<EthReceiptProof>;
		pub DepositRedeemAddress get(fn deposit_redeem_address) config(): EthAddress;
	}
}

decl_event! {
	pub enum Event<T>
	where
		<T as system::Trait>::AccountId,
		RingBalance = RingBalance<T>,
		KtonBalance = KtonBalance<T>,
	{
		RedeemRing(AccountId, RingBalance, EthTransactionIndex),
		RedeemKton(AccountId, KtonBalance, EthTransactionIndex),
		RedeemDeposit(AccountId, DepositId, RingBalance, EthTransactionIndex),
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

		/// Deposit - ALREADY REDEEMED
		DepositAR,
		/// *KTON* - ALREADY REDEEMED
		KtonAR,
		/// *RING* - ALREADY REDEEMED
		RingAR,

		/// Eth Log - PARSING FAILED
		EthLogPF,

		/// *KTON* Locked - NO SUFFICIENT BACKING ASSETS
		KtonLockedNSBA,
		/// *RING* Locked - NO SUFFICIENT BACKING ASSETS
		RingLockedNSBA,

		/// Log Entry - NOT EXISTED
		LogEntryNE,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Redeem balances
		///
		/// # <weight>
		/// - `O(1)`
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn redeem(origin, r#for: RedeemFor) {
			let _relayer = ensure_signed(origin)?;

			match r#for {
				RedeemFor::Ring(proof_record) => Self::redeem_ring(proof_record)?,
				RedeemFor::Kton(proof_record) => Self::redeem_kton(proof_record)?,
				RedeemFor::Deposit(proof_record) => Self::redeem_deposit(proof_record)?,
			}
		}

		/// Set a new ring redeem address.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// - `new`: The new ring redeem address.
		///
		/// # <weight>
		/// - `O(1)`.
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn set_ring_redeem_address(origin, new: EthAddress) {
			ensure_root(origin)?;
			RingRedeemAddress::put(new);
		}

		/// Set a new kton redeem address.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// - `new`: The new kton redeem address.
		///
		/// # <weight>
		/// - `O(1)`.
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn set_kton_redeem_address(origin, new: EthAddress) {
			ensure_root(origin)?;
			KtonRedeemAddress::put(new);
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
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn set_deposit_redeem_address(origin, new: EthAddress) {
			ensure_root(origin)?;
			DepositRedeemAddress::put(new);
		}
	}
}

impl<T: Trait> Module<T> {
	// --- immutable ---

	fn parse_token_redeem_proof(
		proof_record: &EthReceiptProof,
		event_name: &str,
	) -> Result<(Balance, T::AccountId), DispatchError> {
		let result = {
			let verified_receipt = T::EthRelay::verify_receipt(proof_record)?;
			let eth_event = EthEvent {
				name: event_name.to_owned(),
				inputs: vec![
					EthEventParam {
						name: "token".to_owned(),
						kind: ParamType::Address,
						indexed: true,
					},
					EthEventParam {
						name: "owner".to_owned(),
						kind: ParamType::Address,
						indexed: true,
					},
					EthEventParam {
						name: "amount".to_owned(),
						kind: ParamType::Uint(256),
						indexed: false,
					},
					EthEventParam {
						name: "data".to_owned(),
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
					x.address == Self::ring_redeem_address() && x.topics[0] == eth_event.signature()
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
			let raw_sub_key = result.params[3]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;

			//			let decoded_sub_key = hex::decode(&raw_sub_key).map_err(|_| "Decode Address - FAILED")?;

			T::DetermineAccountId::account_id_for(&raw_sub_key)?
		};

		Ok((redeemed_amount, darwinia_account))
	}

	fn parse_deposit_redeem_proof(
		proof_record: &EthReceiptProof,
	) -> Result<
		(
			DepositId,
			MomentT<T>,
			MomentT<T>,
			RingBalance<T>,
			T::AccountId,
		),
		DispatchError,
	> {
		let result = {
			let verified_receipt = T::EthRelay::verify_receipt(proof_record)?;
			let eth_event = EthEvent {
				name: "Burndrop".to_owned(),
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

			<MomentT<T>>::saturated_from(month.saturated_into())
		};
		// https://github.com/evolutionlandorg/bank/blob/master/contracts/GringottsBankV2.sol#L178
		// The start_at here is in seconds, will be converted to milliseconds later in on_deposit_redeem
		let start_at = {
			let start_at = result.params[3]
				.value
				.clone()
				.to_uint()
				.ok_or(<Error<T>>::IntCF)?;

			<MomentT<T>>::saturated_from(start_at.saturated_into())
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
			let raw_sub_key = result.params[6]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;
			//				let decoded_sub_key = hex::decode(&raw_sub_key).map_err(|_| "Decode Address - FAILED")?;

			T::DetermineAccountId::account_id_for(&raw_sub_key)?
		};

		Ok((deposit_id, month, start_at, redeemed_ring, darwinia_account))
	}

	// --- mutable ---

	// event RingBurndropTokens(address indexed token, address indexed owner, uint amount, bytes data)
	// https://ropsten.etherscan.io/tx/0x81f699c93b00ab0b7db701f87b6f6045c1e0692862fcaaf8f06755abb0536800
	fn redeem_ring(proof_record: EthReceiptProof) -> DispatchResult {
		ensure!(
			!RingProofVerified::contains_key((proof_record.header_hash, proof_record.index)),
			<Error<T>>::RingAR,
		);

		let (redeemed_ring, darwinia_account) =
			Self::parse_token_redeem_proof(&proof_record, "RingBurndropTokens")?;
		let redeemed_ring = redeemed_ring.saturated_into();
		let new_ring_locked = Self::ring_locked()
			.checked_sub(&redeemed_ring)
			.ok_or(<Error<T>>::RingLockedNSBA)?;
		let redeemed_positive_imbalance_ring =
			T::Ring::deposit_into_existing(&darwinia_account, redeemed_ring)?;

		// RingReward should be set to (), cause nothing special need to be done for on_unbalanced
		T::RingReward::on_unbalanced(redeemed_positive_imbalance_ring);
		RingProofVerified::insert(
			(proof_record.header_hash, proof_record.index),
			&proof_record,
		);
		<RingLocked<T>>::put(new_ring_locked);
		<Module<T>>::deposit_event(RawEvent::RedeemRing(
			darwinia_account,
			redeemed_ring,
			(proof_record.header_hash, proof_record.index),
		));

		Ok(())
	}

	// event KtonBurndropTokens(address indexed token, address indexed owner, uint amount, bytes data)
	// https://ropsten.etherscan.io/tx/0xc878562085dd8b68ad81adf0820aa0380f1f81b0ea7c012be122937b74020f96
	fn redeem_kton(proof_record: EthReceiptProof) -> DispatchResult {
		ensure!(
			!KtonProofVerified::contains_key((proof_record.header_hash, proof_record.index)),
			<Error<T>>::KtonAR,
		);

		let (redeemed_kton, darwinia_account) =
			Self::parse_token_redeem_proof(&proof_record, "KtonBurndropTokens")?;
		let redeemed_kton = redeemed_kton.saturated_into();
		let new_kton_locked = Self::kton_locked()
			.checked_sub(&redeemed_kton)
			.ok_or(<Error<T>>::KtonLockedNSBA)?;
		let redeemed_positive_imbalance_kton =
			T::Kton::deposit_into_existing(&darwinia_account, redeemed_kton)?;

		// KtonReward should be set to (), cause nothing special need to be done for on_unbalanced
		T::KtonReward::on_unbalanced(redeemed_positive_imbalance_kton);
		KtonProofVerified::insert(
			(proof_record.header_hash, proof_record.index),
			&proof_record,
		);
		<KtonLocked<T>>::put(new_kton_locked);
		<Module<T>>::deposit_event(RawEvent::RedeemKton(
			darwinia_account,
			redeemed_kton,
			(proof_record.header_hash, proof_record.index),
		));

		Ok(())
	}

	// event Burndrop(uint256 indexed _depositID,  address _depositor, uint48 _months, uint48 _startAt, uint64 _unitInterest, uint128 _value, bytes _data)
	// https://ropsten.etherscan.io/tx/0xfd2cac791bb0c0bee7c5711f17ef93401061d314f4eb84e1bc91f32b73134ca1
	fn redeem_deposit(proof_record: EthReceiptProof) -> DispatchResult {
		ensure!(
			!DepositProofVerified::contains_key((proof_record.header_hash, proof_record.index)),
			<Error<T>>::DepositAR,
		);

		let (deposit_id, month, start_at, redeemed_ring, darwinia_account) =
			Self::parse_deposit_redeem_proof(&proof_record)?;
		let new_ring_locked = Self::ring_locked()
			.checked_sub(&redeemed_ring)
			.ok_or(<Error<T>>::RingLockedNSBA)?;

		T::OnDepositRedeem::on_deposit_redeem(start_at, month, redeemed_ring, &darwinia_account)?;
		// TODO: check deposit_id duplication
		// TODO: Ignore Unit Interest for now
		DepositProofVerified::insert(
			(proof_record.header_hash, proof_record.index),
			&proof_record,
		);
		<RingLocked<T>>::put(new_ring_locked);
		<Module<T>>::deposit_event(RawEvent::RedeemDeposit(
			darwinia_account,
			deposit_id,
			redeemed_ring,
			(proof_record.header_hash, proof_record.index),
		));

		Ok(())
	}
}

pub trait AccountIdFor<AccountId> {
	fn account_id_for(decoded_sub_key: &[u8]) -> Result<AccountId, DispatchError>;
}

pub struct AccountIdDeterminator<T: Trait>(PhantomData<T>);

impl<T: Trait> AccountIdFor<T::AccountId> for AccountIdDeterminator<T>
where
	T::AccountId: sp_std::convert::From<[u8; 32]> + AsRef<[u8]>,
{
	fn account_id_for(decoded_sub_key: &[u8]) -> Result<T::AccountId, DispatchError> {
		ensure!(decoded_sub_key.len() == 33, <Error<T>>::AddrLenMis);
		ensure!(
			decoded_sub_key[0] == T::SubKeyPrefix::get(),
			<Error<T>>::PubkeyPrefixMis
		);

		let mut raw_account = [0u8; 32];
		raw_account.copy_from_slice(&decoded_sub_key[1..]);

		Ok(raw_account.into())
	}
}

impl<T: Trait> Module<T> {
	pub fn adjust_deposit_value() {
		unimplemented!()
	}
}
