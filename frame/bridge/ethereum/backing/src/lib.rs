// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for cross chain assets backing.

// TODO: https://github.com/darwinia-network/darwinia-common/issues/372
#![allow(unused)]
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

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type RingBalance<T> = <<T as Trait>::RingCurrency as Currency<AccountId<T>>>::Balance;
	pub type KtonBalance<T> = <<T as Trait>::KtonCurrency as Currency<AccountId<T>>>::Balance;

	pub type EthereumReceiptProofThing<T> = <<T as Trait>::EthereumRelay as EthereumReceipt<
		AccountId<T>,
		RingBalance<T>,
	>>::EthereumReceiptProofThing;

	pub type EcdsaSignature = [u8; 65];
	pub type EcdsaMessage = [u8; 32];
}

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethabi::{Event as EthEvent, EventParam as EthEventParam, ParamType, RawLog};
// --- substrate ---
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement::KeepAlive, Get},
	weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use sp_io::{crypto, hashing};
use sp_runtime::{
	traits::{AccountIdConversion, SaturatedConversion, Saturating, Zero},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{convert::TryFrom, prelude::*};
// --- darwinia ---
use array_bytes::array_unchecked;
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::{
	balance::lock::*,
	traits::{EthereumReceipt, OnDepositRedeem},
};
use ethereum_primitives::{
	receipt::{EthereumTransactionIndex, LogEntry},
	EthereumAddress, U256,
};
use types::*;

pub trait Trait: frame_system::Trait {
	/// The ethereum backing module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;
	type FeeModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type RedeemAccountId: From<[u8; 32]> + Into<Self::AccountId>;
	type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;
	type OnDepositRedeem: OnDepositRedeem<Self::AccountId, RingBalance<Self>>;

	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	type KtonCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type RingLockLimit: Get<RingBalance<Self>>;
	type KtonLockLimit: Get<KtonBalance<Self>>;
	type AdvancedFee: Get<RingBalance<Self>>;
	type SyncReward: Get<RingBalance<Self>>;
	type EcdsaAuthorities: RelayAuthorityProtocol<Self::BlockNumber, Signer = EthereumAddress>;

	/// Weight information for the extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
		RingBalance = RingBalance<T>,
		KtonBalance = KtonBalance<T>,
	{
		/// Someone redeem some *RING*. [account, amount, transaction index]
		RedeemRing(AccountId, Balance, EthereumTransactionIndex),
		/// Someone redeem some *KTON*. [account, amount, transaction index]
		RedeemKton(AccountId, Balance, EthereumTransactionIndex),
		/// Someone redeem a deposit. [account, deposit id, amount, transaction index]
		RedeemDeposit(AccountId, DepositId, RingBalance, EthereumTransactionIndex),
		/// Someone lock some *RING*. [account, ethereum account, asset address, amount]
		LockRing(AccountId, EthereumAddress, EthereumAddress, RingBalance),
		/// Someone lock some *KTON*. [account, ethereum account, asset address, amount]
		LockKton(AccountId, EthereumAddress, EthereumAddress, KtonBalance),
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
		/// Array - CONVERSION FAILED
		ArrayCF,
		/// Address - CONVERSION FAILED
		AddressCF,
		/// Asset - ALREADY REDEEMED
		AssetAR,
		/// Authorities Change - ALREADY SYNCED
		AuthoritiesChangeAR,
		/// EthereumReceipt Proof - INVALID
		ReceiptProofInv,
		/// Eth Log - PARSING FAILED
		EthLogPF,
		/// *KTON* Locked - NO SUFFICIENT BACKING ASSETS
		KtonLockedNSBA,
		/// *RING* Locked - NO SUFFICIENT BACKING ASSETS
		RingLockedNSBA,
		/// Log Entry - NOT EXISTED
		LogEntryNE,
		// TODO: remove fee?
		// /// Usable Balance for Paying Redeem Fee - INSUFFICIENT
		// FeeIns,
		/// Redeem - DISABLED
		RedeemDis,
		/// Ring Lock - LIMITED
		RingLockLim,
		/// Kton Lock - LIMITED
		KtonLockLim,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthereumBacking {
		pub VerifiedProof
			get(fn verified_proof)
			: map hasher(blake2_128_concat) EthereumTransactionIndex => bool = false;

		pub TokenRedeemAddress get(fn token_redeem_address) config(): EthereumAddress;
		pub DepositRedeemAddress get(fn deposit_redeem_address) config(): EthereumAddress;
		pub SetAuthoritiesAddress get(fn set_authorities_address) config(): EthereumAddress;

		pub RingTokenAddress get(fn ring_token_address) config(): EthereumAddress;
		pub KtonTokenAddress get(fn kton_token_address) config(): EthereumAddress;

		pub RedeemStatus get(fn redeem_status): bool = true;

		pub LockAssetEvents
			get(fn lock_asset_events)
			: Vec<<T as frame_system::Trait>::Event>;
	}
	add_extra_genesis {
		config(ring_locked): RingBalance<T>;
		config(kton_locked): KtonBalance<T>;
		build(|config: &GenesisConfig<T>| {
			// Create Backing account
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance() + config.ring_locked,
			);
			let _ = T::KtonCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::KtonCurrency::minimum_balance() + config.kton_locked,
			);
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::fee_account_id(),
				T::RingCurrency::minimum_balance(),
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

		/// The ethereum backing module id, used for deriving its sovereign account ID.
		const ModuleId: ModuleId = T::ModuleId::get();

		const FeeModuleId: ModuleId = T::FeeModuleId::get();

		const AdvancedFee: RingBalance<T> = T::AdvancedFee::get();

		const SyncReward: RingBalance<T> = T::SyncReward::get();

		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			<LockAssetEvents<T>>::kill();

			0
		}

		/// Redeem balances
		///
		/// # <weight>
		/// - `O(1)`
		/// # </weight>
		#[weight = 10_000_000]
		pub fn redeem(origin, act: RedeemFor, proof: EthereumReceiptProofThing<T>) {
			let redeemer = ensure_signed(origin)?;

			if RedeemStatus::get() {
				match act {
					RedeemFor::Token => Self::redeem_token(&redeemer, &proof)?,
					RedeemFor::Deposit => Self::redeem_deposit(&redeemer, &proof)?,
				}
			} else {
				Err(<Error<T>>::RedeemDis)?;
			}
		}

		/// Lock some balances into the module account
		/// which very similar to lock some assets into the contract on ethereum side
		#[weight = 10_000_000]
		pub fn lock(
			origin,
			#[compact] ring_value: RingBalance<T>,
			#[compact] kton_value: KtonBalance<T>,
			ethereum_account: EthereumAddress,
		) {
			let user = ensure_signed(origin)?;
			let fee_account = Self::fee_account_id();

			// 50 Ring for fee
			// https://github.com/darwinia-network/darwinia-common/pull/377#issuecomment-730369387
			T::RingCurrency::transfer(&user, &fee_account, T::AdvancedFee::get(), KeepAlive)?;

			let mut locked = false;

			if !ring_value.is_zero() {
				let ring_to_lock = ring_value.min(T::RingCurrency::usable_balance(&user));

				ensure!(ring_to_lock < T::RingLockLimit::get(), <Error<T>>::RingLockLim);

				T::RingCurrency::transfer(&user, &Self::account_id(), ring_to_lock, KeepAlive)?;

				let raw_event = RawEvent::LockRing(
					user.clone(),
					ethereum_account.clone(),
					RingTokenAddress::get(),
					ring_to_lock
				);
				let module_event: <T as Trait>::Event = raw_event.clone().into();
				let system_event: <T as frame_system::Trait>::Event = module_event.into();

				locked = true;

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(raw_event);
			}
			if !kton_value.is_zero() {
				let kton_to_lock = kton_value.min(T::KtonCurrency::usable_balance(&user));

				ensure!(kton_to_lock < T::KtonLockLimit::get(), <Error<T>>::KtonLockLim);

				T::KtonCurrency::transfer(&user, &Self::account_id(), kton_to_lock, KeepAlive)?;

				let raw_event = RawEvent::LockKton(
					user,
					ethereum_account,
					KtonTokenAddress::get(),
					kton_to_lock
				);
				let module_event: <T as Trait>::Event = raw_event.clone().into();
				let system_event: <T as frame_system::Trait>::Event = module_event.into();

				locked = true;

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(raw_event);
			}

			if locked {
				T::EcdsaAuthorities::schedule_mmr_root((
					<frame_system::Module<T>>::block_number().saturated_into()
						/ 10 * 10 + 10
				).saturated_into());
			}
		}

		#[weight = 10_000_000]
		fn sync_authorities_change(origin, proof: EthereumReceiptProofThing<T>) {
			let bridger = ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);

			ensure!(!VerifiedProof::contains_key(tx_index), <Error<T>>::AuthoritiesChangeAR);

			let (term, authorities, beneficiary) = Self::parse_authorities_set_proof(&proof)?;

			T::EcdsaAuthorities::check_authorities_change_to_sync(term, authorities)?;

			let fee_account = Self::fee_account_id();
			let sync_reward = T::SyncReward::get().min(
				T::RingCurrency::usable_balance(&fee_account)
					.saturating_sub(T::RingCurrency::minimum_balance())
			);

			if !sync_reward.is_zero() {
				T::RingCurrency::transfer(
					&fee_account,
					&beneficiary,
					sync_reward,
					KeepAlive
				)?;
			}

			T::EcdsaAuthorities::sync_authorities_change()?;

			VerifiedProof::insert(tx_index, true);
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

		#[weight = 10_000_000]
		pub fn set_redeem_status(origin, status: bool) {
			ensure_root(origin)?;

			RedeemStatus::put(status);
		}
	}
}

impl<T: Trait> Module<T> {
	/// The account ID of the backing pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn fee_account_id() -> T::AccountId {
		T::FeeModuleId::get().into_account()
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
			.map_err(|_| <Error<T>>::ReceiptProofInv)?;
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
					x.address == TokenRedeemAddress::get() && x.topics[0] == eth_event.signature()
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
				token_address == RingTokenAddress::get()
					|| token_address == KtonTokenAddress::get(),
				<Error<T>>::AssetAR
			);

			token_address == RingTokenAddress::get()
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
			let raw_account_id = result.params[3]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;
			debug::trace!(target: "ethereum-backing", "[ethereum-backing] Raw Account: {:?}", raw_account_id);

			Self::account_id_try_from_bytes(&raw_account_id)?
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
			.map_err(|_| <Error<T>>::ReceiptProofInv)?;
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
					x.address == DepositRedeemAddress::get() && x.topics[0] == eth_event.signature()
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
		let months = {
			let months = result.params[2]
				.value
				.clone()
				.to_uint()
				.ok_or(<Error<T>>::IntCF)?;

			months.saturated_into()
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

			<RingBalance<T>>::saturated_from(redeemed_ring.saturated_into::<u128>())
		};
		let darwinia_account = {
			let raw_account_id = result.params[6]
				.value
				.clone()
				.to_bytes()
				.ok_or(<Error<T>>::BytesCF)?;
			debug::trace!(target: "ethereum-backing", "[ethereum-backing] Raw Account: {:?}", raw_account_id);

			Self::account_id_try_from_bytes(&raw_account_id)?
		};
		debug::trace!(target: "ethereum-backing", "[ethereum-backing] Darwinia Account: {:?}", darwinia_account);

		Ok((
			deposit_id,
			darwinia_account,
			redeemed_ring,
			start_at,
			months,
			fee,
		))
	}

	// event SetAuthritiesEvent(uint32 nonce, address[] authorities, bytes32 benifit);
	// https://github.com/darwinia-network/darwinia-bridge-on-ethereum/blob/51839e614c0575e431eabfd5c70b84f6aa37826a/contracts/Relay.sol#L22
	// https://ropsten.etherscan.io/tx/0x652528b9421ecb495610a734a4ab70d054b5510dbbf3a9d5c7879c43c7dde4e9#eventlog
	fn parse_authorities_set_proof(
		proof_record: &EthereumReceiptProofThing<T>,
	) -> Result<(Term, Vec<EthereumAddress>, AccountId<T>), DispatchError> {
		let log = {
			let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)
				.map_err(|_| <Error<T>>::ReceiptProofInv)?;
			let eth_event = EthEvent {
				name: "SetAuthoritiesEvent".into(),
				inputs: vec![
					EthEventParam {
						name: "nonce".into(),
						kind: ParamType::Uint(32),
						indexed: false,
					},
					EthEventParam {
						name: "authorities".into(),
						kind: ParamType::Array(Box::new(ParamType::Address)),
						indexed: false,
					},
					EthEventParam {
						name: "beneficiary".into(),
						kind: ParamType::FixedBytes(32),
						indexed: false,
					},
				],
				anonymous: false,
			};
			let LogEntry { topics, data, .. } = verified_receipt
				.logs
				.into_iter()
				.find(|x| {
					x.address == SetAuthoritiesAddress::get()
						&& x.topics[0] == eth_event.signature()
				})
				.ok_or(<Error<T>>::LogEntryNE)?;

			eth_event
				.parse_log(RawLog {
					topics: vec![topics[0]],
					data,
				})
				.map_err(|_| <Error<T>>::EthLogPF)?
		};
		let term = log.params[0]
			.value
			.clone()
			.to_uint()
			.ok_or(<Error<T>>::BytesCF)?
			.saturated_into();
		let authorities = {
			let mut authorities = vec![];

			for token in log.params[1]
				.value
				.clone()
				.to_array()
				.ok_or(<Error<T>>::ArrayCF)?
			{
				authorities.push(token.to_address().ok_or(<Error<T>>::AddressCF)?);
			}

			authorities
		};
		let beneficiary = {
			let raw_account_id = log.params[2]
				.value
				.clone()
				.to_fixed_bytes()
				.ok_or(<Error<T>>::BytesCF)?;

			debug::trace!(target: "ethereum-backing", "[ethereum-backing] Raw Account: {:?}", raw_account_id);

			Self::account_id_try_from_bytes(&raw_account_id)?
		};

		Ok((term, authorities, beneficiary))
	}

	fn redeem_token(
		redeemer: &T::AccountId,
		proof: &EthereumReceiptProofThing<T>,
	) -> DispatchResult {
		let tx_index = T::EthereumRelay::gen_receipt_index(proof);

		ensure!(!VerifiedProof::contains_key(tx_index), <Error<T>>::AssetAR);

		// TODO: remove fee?
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
		// // Checking redeemer have enough of balance to pay fee, make sure follow up transfer will success.
		// ensure!(
		// 	T::RingCurrency::usable_balance(redeemer) >= fee,
		// 	<Error<T>>::FeeIns
		// );

		C::transfer(
			&Self::account_id(),
			&darwinia_account,
			redeem_amount,
			KeepAlive,
		)?;
		// // Transfer the fee from redeemer.
		// T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee, KeepAlive)?;

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

		// TODO: remove fee?
		let (deposit_id, darwinia_account, redeemed_ring, start_at, months, fee) =
			Self::parse_deposit_redeem_proof(&proof)?;

		ensure!(
			Self::pot::<T::RingCurrency>() >= redeemed_ring,
			<Error<T>>::RingLockedNSBA
		);
		// // Checking redeemer have enough of balance to pay fee, make sure follow up fee transfer will success.
		// ensure!(
		// 	T::RingCurrency::usable_balance(redeemer) >= fee,
		// 	<Error<T>>::FeeIns
		// );

		T::OnDepositRedeem::on_deposit_redeem(
			&Self::account_id(),
			&darwinia_account,
			redeemed_ring,
			start_at,
			months,
		)?;
		// // Transfer the fee from redeemer.
		// T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee, KeepAlive)?;

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

impl<T: Trait> Sign<BlockNumber<T>> for Module<T> {
	type Signature = EcdsaSignature;
	type Message = EcdsaMessage;
	type Signer = EthereumAddress;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message {
		hashing::keccak_256(raw_message.as_ref())
	}

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool {
		fn eth_signable_message(message: &[u8]) -> Vec<u8> {
			let mut l = message.len();
			let mut rev = Vec::new();

			while l > 0 {
				rev.push(b'0' + (l % 10) as u8);
				l /= 10;
			}

			let mut v = b"\x19Ethereum Signed Message:\n".to_vec();

			v.extend(rev.into_iter().rev());
			v.extend_from_slice(message);

			v
		}

		let message = hashing::keccak_256(&eth_signable_message(message));

		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature, &message) {
			hashing::keccak_256(&public_key)[12..] == signer.0
		} else {
			false
		}
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RedeemFor {
	Token,
	Deposit,
}
