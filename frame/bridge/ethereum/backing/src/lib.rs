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
// along with Darwinia.  If not, see <https://www.gnu.org/licenses/>.

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
	pub type RingBalance<T> = <<T as Trait>::RingCurrency as Currency<AccountId<T>>>::Balance;
	pub type KtonBalance<T> = <<T as Trait>::KtonCurrency as Currency<AccountId<T>>>::Balance;

	pub type EthereumReceiptProofThing<T> = <<T as Trait>::EthereumRelay as EthereumReceipt<
		AccountId<T>,
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
	traits::{AccountIdConversion, SaturatedConversion, Saturating, Zero},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{convert::TryFrom, prelude::*};
// --- darwinia ---
use array_bytes::array_unchecked;
use darwinia_support::{
	balance::lock::*,
	traits::{EthereumReceipt, OnDepositRedeem},
};
use ethereum_primitives::{receipt::EthereumTransactionIndex, EthereumAddress, U256};
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

	type AdvancedFee: Get<RingBalance<Self>>;

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
		/// Someone lock some *RING*. [account, amount]
		LockRing(AccountId, RingBalance),
		/// Someone lock some *KTON*. [account, amount]
		LockKton(AccountId, KtonBalance),
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

		fn deposit_event() = default;

		fn on_initialize() {
			<LockAssetEvents<T>>::kill();
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::fee_account_id(),
				T::RingCurrency::minimum_balance(),
			);

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

			if Self::redeem_status() {
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
		) {
			let user = ensure_signed(origin)?;
			let fee_account = Self::fee_account_id();

			// 50 Ring for fee
			// https://github.com/darwinia-network/darwinia-common/pull/377#issuecomment-730369387
			T::RingCurrency::transfer(&user, &fee_account, T::AdvancedFee::get(), KeepAlive)?;

			if !ring_value.is_zero() {
				let ring_to_lock = ring_value.min(T::RingCurrency::usable_balance(&user));

				T::RingCurrency::transfer(&user, &fee_account, ring_to_lock, KeepAlive)?;

				let raw_event = RawEvent::LockRing(user.clone(), ring_to_lock);
				let module_event: <T as Trait>::Event = raw_event.clone().into();
				let system_event: <T as frame_system::Trait>::Event = module_event.into();

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(raw_event);
			}
			if !kton_value.is_zero() {
				let kton_to_lock = kton_value.min(T::KtonCurrency::usable_balance(&user));

				T::KtonCurrency::transfer(&user, &fee_account, kton_to_lock, KeepAlive)?;

				let raw_event = RawEvent::LockKton(user, kton_to_lock);
				let module_event: <T as Trait>::Event = raw_event.clone().into();
				let system_event: <T as frame_system::Trait>::Event = module_event.into();

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(raw_event);
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

		#[weight = 10_000_000]
		pub fn set_redeem_status(origin, status: bool) {
			ensure_root(origin)?;

			RedeemStatus::put(status);
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
			months,
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

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RedeemFor {
	Token,
	Deposit,
}
