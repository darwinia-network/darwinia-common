// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for cross chain assets backing.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		// --- darwinia-network ---
		use crate::pallet::*;

		// Simple type
		pub type Balance = u128;
		pub type DepositId = U256;
		pub type EcdsaSignature = [u8; 65];
		pub type EcdsaMessage = [u8; 32];
		// Generic type
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
		pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
		pub type KtonBalance<T> = <<T as Config>::KtonCurrency as Currency<AccountId<T>>>::Balance;
		pub type EthereumReceiptProofThing<T> = <<T as Config>::EthereumRelay as EthereumReceipt<
			AccountId<T>,
			RingBalance<T>,
		>>::EthereumReceiptProofThing;
	}
	pub use types::*;

	// --- crates.io ---
	use ethabi::{Event as EthEvent, EventParam as EthEventParam, ParamType, RawLog};
	use scale_info::TypeInfo;
	// --- paritytech ---
	use frame_support::{
		log,
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, LockableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_io::{crypto, hashing};
	use sp_runtime::traits::{AccountIdConversion, SaturatedConversion, Saturating, Zero};
	#[cfg(not(feature = "std"))]
	use sp_std::borrow::ToOwned;
	use sp_std::{convert::TryFrom, prelude::*};
	// --- darwinia-network ---
	use crate::weights::WeightInfo;
	use darwinia_relay_primitives::relay_authorities::*;
	use darwinia_support::traits::{EthereumReceipt, OnDepositRedeem};
	use ethereum_primitives::{
		log_entry::LogEntry, receipt::EthereumTransactionIndex, EthereumAddress, U256,
	};

	// TODO
	// macro_rules! set_address_call {
	// 	($call_name:ident, $address:ty) => {
	// 		#[pallet::weight(10_000_000)]
	// 		pub fn $call_name(
	// 			origin: OriginFor<T>,
	// 			new: EthereumAddress,
	// 		) -> DispatchResultWithPostInfo {
	// 			ensure_root(origin)?;

	// 			$address::put(new);

	// 			Ok(().into())
	// 		}
	// 	}
	// }

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// --- paritytech ---
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		// --- darwinia-network ---
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type FeePalletId: Get<PalletId>;
		type RingCurrency: LockableCurrency<Self::AccountId>;
		type KtonCurrency: LockableCurrency<Self::AccountId>;
		type RedeemAccountId: From<[u8; 32]> + Into<Self::AccountId>;
		type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;
		type OnDepositRedeem: OnDepositRedeem<Self::AccountId, RingBalance<Self>>;
		#[pallet::constant]
		type RingLockLimit: Get<RingBalance<Self>>;
		#[pallet::constant]
		type KtonLockLimit: Get<KtonBalance<Self>>;
		#[pallet::constant]
		type AdvancedFee: Get<RingBalance<Self>>;
		#[pallet::constant]
		type SyncReward: Get<RingBalance<Self>>;
		type EcdsaAuthorities: RelayAuthorityProtocol<Self::BlockNumber, Signer = EthereumAddress>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Someone redeem some *RING*. \[account, amount, transaction index\]
		RedeemRing(AccountId<T>, Balance, EthereumTransactionIndex),
		/// Someone redeem some *KTON*. \[account, amount, transaction index\]
		RedeemKton(AccountId<T>, Balance, EthereumTransactionIndex),
		/// Someone redeem a deposit. \[account, deposit id, amount, transaction index\]
		RedeemDeposit(AccountId<T>, DepositId, RingBalance<T>, EthereumTransactionIndex),
		/// Someone lock some *RING*. \[account, ethereum account, asset address, amount\]
		LockRing(AccountId<T>, EthereumAddress, EthereumAddress, RingBalance<T>),
		/// Someone lock some *KTON*. \[account, ethereum account, asset address, amount\]
		LockKton(AccountId<T>, EthereumAddress, EthereumAddress, KtonBalance<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
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

	#[pallet::storage]
	#[pallet::getter(fn verified_proof)]
	pub type VerifiedProof<T> = StorageMap<
		_,
		Blake2_128Concat,
		EthereumTransactionIndex,
		bool,
		ValueQuery,
		DefaultForVerifiedProof,
	>;
	#[pallet::type_value]
	pub fn DefaultForVerifiedProof() -> bool {
		false
	}

	#[pallet::storage]
	#[pallet::getter(fn token_redeem_address)]
	pub type TokenRedeemAddress<T> = StorageValue<_, EthereumAddress, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn deposit_redeem_address)]
	pub type DepositRedeemAddress<T> = StorageValue<_, EthereumAddress, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn set_authorities_address)]
	pub type SetAuthoritiesAddress<T> = StorageValue<_, EthereumAddress, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn ring_token_address)]
	pub type RingTokenAddress<T> = StorageValue<_, EthereumAddress, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn kton_token_address)]
	pub type KtonTokenAddress<T> = StorageValue<_, EthereumAddress, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn redeem_status)]
	pub type RedeemStatus<T> = StorageValue<_, bool, ValueQuery, DefaultForRedeemStatus>;
	#[pallet::type_value]
	pub fn DefaultForRedeemStatus() -> bool {
		true
	}

	#[pallet::storage]
	#[pallet::getter(fn lock_asset_events)]
	pub type LockAssetEvents<T> =
		StorageValue<_, Vec<<T as frame_system::Config>::Event>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub token_redeem_address: EthereumAddress,
		pub deposit_redeem_address: EthereumAddress,
		pub set_authorities_address: EthereumAddress,
		pub ring_token_address: EthereumAddress,
		pub kton_token_address: EthereumAddress,
		pub backed_ring: RingBalance<T>,
		pub backed_kton: KtonBalance<T>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				token_redeem_address: Default::default(),
				deposit_redeem_address: Default::default(),
				set_authorities_address: Default::default(),
				ring_token_address: Default::default(),
				kton_token_address: Default::default(),
				backed_ring: Default::default(),
				backed_kton: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<TokenRedeemAddress<T>>::put(self.token_redeem_address);
			<DepositRedeemAddress<T>>::put(self.deposit_redeem_address);
			<SetAuthoritiesAddress<T>>::put(self.set_authorities_address);
			<RingTokenAddress<T>>::put(self.ring_token_address);
			<KtonTokenAddress<T>>::put(self.kton_token_address);

			let _ = T::RingCurrency::make_free_balance_be(
				&<Pallet<T>>::account_id(),
				T::RingCurrency::minimum_balance() + self.backed_ring,
			);
			let _ =
				T::KtonCurrency::make_free_balance_be(&<Pallet<T>>::account_id(), self.backed_kton);
			let _ = T::RingCurrency::make_free_balance_be(
				&<Pallet<T>>::fee_account_id(),
				T::RingCurrency::minimum_balance(),
			);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			<LockAssetEvents<T>>::kill();

			T::DbWeight::get().writes(1)
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Redeem balances
		///
		/// # <weight>
		/// - `O(1)`
		/// # </weight>
		#[pallet::weight(10_000_000)]
		pub fn redeem(
			origin: OriginFor<T>,
			act: RedeemFor,
			proof: EthereumReceiptProofThing<T>,
		) -> DispatchResultWithPostInfo {
			let redeemer = ensure_signed(origin)?;

			if <RedeemStatus<T>>::get() {
				match act {
					RedeemFor::Token => Self::redeem_token(&redeemer, &proof)?,
					RedeemFor::Deposit => Self::redeem_deposit(&redeemer, &proof)?,
				}
			} else {
				Err(<Error<T>>::RedeemDis)?;
			}

			Ok(().into())
		}

		/// Lock some balances into the module account
		/// which very similar to lock some assets into the contract on ethereum side
		///
		/// This might kill the account just like `balances::transfer`
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn lock(
			origin: OriginFor<T>,
			#[pallet::compact] ring_to_lock: RingBalance<T>,
			#[pallet::compact] kton_to_lock: KtonBalance<T>,
			ethereum_account: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			let fee_account = Self::fee_account_id();

			// 50 Ring for fee
			// https://github.com/darwinia-network/darwinia-common/pull/377#issuecomment-730369387
			T::RingCurrency::transfer(
				&user,
				&fee_account,
				T::AdvancedFee::get(),
				ExistenceRequirement::KeepAlive,
			)?;

			let mut locked = false;

			if !ring_to_lock.is_zero() {
				ensure!(ring_to_lock < T::RingLockLimit::get(), <Error<T>>::RingLockLim);

				T::RingCurrency::transfer(
					&user,
					&Self::account_id(),
					ring_to_lock,
					ExistenceRequirement::AllowDeath,
				)?;

				let event = Event::LockRing(
					user.clone(),
					ethereum_account.clone(),
					<RingTokenAddress<T>>::get(),
					ring_to_lock,
				);
				let module_event: <T as Config>::Event = event.clone().into();
				let system_event: <T as frame_system::Config>::Event = module_event.into();

				locked = true;

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(event);
			}
			if !kton_to_lock.is_zero() {
				ensure!(kton_to_lock < T::KtonLockLimit::get(), <Error<T>>::KtonLockLim);

				T::KtonCurrency::transfer(
					&user,
					&Self::account_id(),
					kton_to_lock,
					ExistenceRequirement::AllowDeath,
				)?;

				let event = Event::LockKton(
					user,
					ethereum_account,
					<KtonTokenAddress<T>>::get(),
					kton_to_lock,
				);
				let module_event: <T as Config>::Event = event.clone().into();
				let system_event: <T as frame_system::Config>::Event = module_event.into();

				locked = true;

				<LockAssetEvents<T>>::append(system_event);
				Self::deposit_event(event);
			}

			if locked {
				T::EcdsaAuthorities::schedule_mmr_root(
					(<frame_system::Pallet<T>>::block_number().saturated_into::<u32>() / 10 * 10
						+ 10)
						.saturated_into(),
				)?;
			}

			Ok(().into())
		}

		// Transfer should always return ok
		// Even it failed, still finish the syncing
		//
		// But should not dispatch the reward if the syncing failed
		#[pallet::weight(10_000_000)]
		pub fn sync_authorities_change(
			origin: OriginFor<T>,
			proof: EthereumReceiptProofThing<T>,
		) -> DispatchResultWithPostInfo {
			let _bridger = ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);

			ensure!(!<VerifiedProof<T>>::contains_key(tx_index), <Error<T>>::AuthoritiesChangeAR);

			let (term, authorities, beneficiary) = Self::parse_authorities_set_proof(&proof)?;

			T::EcdsaAuthorities::check_authorities_change_to_sync(term, authorities)?;
			T::EcdsaAuthorities::sync_authorities_change()?;

			<VerifiedProof<T>>::insert(tx_index, true);

			T::RingCurrency::transfer(
				&Self::fee_account_id(),
				&beneficiary,
				T::SyncReward::get(),
				ExistenceRequirement::KeepAlive,
			)?;

			Ok(().into())
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
		#[pallet::weight(10_000_000)]
		pub fn set_token_redeem_address(
			origin: OriginFor<T>,
			new: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<TokenRedeemAddress<T>>::put(new);

			Ok(().into())
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
		#[pallet::weight(10_000_000)]
		pub fn set_deposit_redeem_address(
			origin: OriginFor<T>,
			new: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<DepositRedeemAddress<T>>::put(new);

			Ok(().into())
		}

		#[pallet::weight(10_000_000)]
		pub fn set_set_authorities_address(
			origin: OriginFor<T>,
			new: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<SetAuthoritiesAddress<T>>::put(new);

			Ok(().into())
		}

		#[pallet::weight(10_000_000)]
		pub fn set_ring_token_address(
			origin: OriginFor<T>,
			new: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<RingTokenAddress<T>>::put(new);

			Ok(().into())
		}

		#[pallet::weight(10_000_000)]
		pub fn set_kton_token_address(
			origin: OriginFor<T>,
			new: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<KtonTokenAddress<T>>::put(new);

			Ok(().into())
		}

		#[pallet::weight(10_000_000)]
		pub fn set_redeem_status(origin: OriginFor<T>, status: bool) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<RedeemStatus<T>>::put(status);

			Ok(().into())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		pub fn fee_account_id() -> T::AccountId {
			T::FeePalletId::get().into_account()
		}

		pub fn account_id_try_from_bytes(bytes: &[u8]) -> Result<T::AccountId, DispatchError> {
			ensure!(bytes.len() == 32, <Error<T>>::AddrLenMis);

			let redeem_account_id: T::RedeemAccountId = array_bytes::dyn_into!(bytes, 32);

			Ok(redeem_account_id.into())
		}

		/// Return the amount of money in the pot.
		// The existential deposit is not part of the pot so backing account never gets deleted.
		pub fn pot<C: LockableCurrency<T::AccountId>>() -> C::Balance {
			// No other locks on this account.
			C::free_balance(&Self::account_id())
				// Must never be less than 0 but better be safe.
				.saturating_sub(C::minimum_balance())
		}

		fn redeem_token(
			redeemer: &T::AccountId,
			proof: &EthereumReceiptProofThing<T>,
		) -> DispatchResult {
			let tx_index = T::EthereumRelay::gen_receipt_index(proof);

			ensure!(!<VerifiedProof<T>>::contains_key(tx_index), <Error<T>>::AssetAR);

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
			_redeemer: &T::AccountId,
			darwinia_account: T::AccountId,
			tx_index: EthereumTransactionIndex,
			is_ring: bool,
			redeem_amount: Balance,
			_fee: RingBalance<T>,
		) -> DispatchResult {
			let raw_amount = redeem_amount;
			let redeem_amount: C::Balance = redeem_amount.saturated_into();

			ensure!(
				Self::pot::<C>() >= redeem_amount,
				if is_ring { <Error<T>>::RingLockedNSBA } else { <Error<T>>::KtonLockedNSBA }
			);
			// // Checking redeemer have enough of balance to pay fee, make sure follow up transfer
			// will success. ensure!(
			// 	T::RingCurrency::usable_balance(redeemer) >= fee,
			// 	<Error<T>>::FeeIns
			// );

			C::transfer(
				&Self::account_id(),
				&darwinia_account,
				redeem_amount,
				ExistenceRequirement::KeepAlive,
			)?;
			// // Transfer the fee from redeemer.
			// T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee,
			// KeepAlive)?;

			<VerifiedProof<T>>::insert(tx_index, true);

			if is_ring {
				Self::deposit_event(Event::RedeemRing(darwinia_account, raw_amount, tx_index));
			} else {
				Self::deposit_event(Event::RedeemKton(darwinia_account, raw_amount, tx_index));
			}

			Ok(())
		}

		// event BurnAndRedeem(address indexed token, address indexed from, uint256 amount, bytes
		// receiver); Redeem RING https://ropsten.etherscan.io/tx/0x1d3ef601b9fa4a7f1d6259c658d0a10c77940fa5db9e10ab55397eb0ce88807d
		// Redeem KTON https://ropsten.etherscan.io/tx/0x2878ae39a9e0db95e61164528bb1ec8684be194bdcc236848ff14d3fe5ba335d
		pub(super) fn parse_token_redeem_proof(
			proof_record: &EthereumReceiptProofThing<T>,
		) -> Result<(T::AccountId, (bool, Balance), RingBalance<T>), DispatchError> {
			let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)?;
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
					.to_legacy_receipt()
					.logs
					.into_iter()
					.find(|x| {
						x.address == <TokenRedeemAddress<T>>::get()
							&& x.topics[0] == eth_event.signature()
					})
					.ok_or(<Error<T>>::LogEntryNE)?;
				let log = RawLog {
					topics: vec![log_entry.topics[0], log_entry.topics[1], log_entry.topics[2]],
					data: log_entry.data.clone(),
				};

				eth_event.parse_log(log).map_err(|_| <Error<T>>::EthLogPF)?
			};
			let is_ring = {
				let token_address =
					result.params[0].value.clone().into_address().ok_or(<Error<T>>::AddressCF)?;

				ensure!(
					token_address == <RingTokenAddress<T>>::get()
						|| token_address == <KtonTokenAddress<T>>::get(),
					<Error<T>>::AssetAR
				);

				token_address == <RingTokenAddress<T>>::get()
			};

			let redeemed_amount = {
				// TODO: div 10**18 and mul 10**9
				let amount = result.params[2]
					.value
					.clone()
					.into_uint()
					.map(|x| x / U256::from(1_000_000_000u64))
					.ok_or(<Error<T>>::IntCF)?;

				Balance::try_from(amount)?
			};
			let darwinia_account = {
				let raw_account_id =
					result.params[3].value.clone().into_bytes().ok_or(<Error<T>>::BytesCF)?;
				log::trace!("[ethereum-backing] Raw Account: {:?}", raw_account_id);

				Self::account_id_try_from_bytes(&raw_account_id)?
			};
			log::trace!("[ethereum-backing] Darwinia Account: {:?}", darwinia_account);

			Ok((darwinia_account, (is_ring, redeemed_amount), fee))
		}

		fn redeem_deposit(
			_redeemer: &T::AccountId,
			proof: &EthereumReceiptProofThing<T>,
		) -> DispatchResult {
			let tx_index = T::EthereumRelay::gen_receipt_index(proof);

			ensure!(!<VerifiedProof<T>>::contains_key(tx_index), <Error<T>>::AssetAR);

			// TODO: remove fee?
			let (deposit_id, darwinia_account, redeemed_ring, start_at, months, _fee) =
				Self::parse_deposit_redeem_proof(&proof)?;

			ensure!(Self::pot::<T::RingCurrency>() >= redeemed_ring, <Error<T>>::RingLockedNSBA);
			// // Checking redeemer have enough of balance to pay fee, make sure follow up fee
			// transfer will success. ensure!(
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
			// T::RingCurrency::transfer(redeemer, &T::EthereumRelay::account_id(), fee,
			// KeepAlive)?;

			// TODO: check deposit_id duplication
			// TODO: Ignore Unit Interest for now
			<VerifiedProof<T>>::insert(tx_index, true);

			Self::deposit_event(Event::RedeemDeposit(
				darwinia_account,
				deposit_id,
				redeemed_ring,
				tx_index,
			));

			Ok(())
		}

		// event BurnAndRedeem(uint256 indexed _depositID,  address _depositor, uint48 _months,
		// uint48 _startAt, uint64 _unitInterest, uint128 _value, bytes _data); Redeem Deposit https://ropsten.etherscan.io/tx/0x5a7004126466ce763501c89bcbb98d14f3c328c4b310b1976a38be1183d91919
		pub(super) fn parse_deposit_redeem_proof(
			proof_record: &EthereumReceiptProofThing<T>,
		) -> Result<(DepositId, T::AccountId, RingBalance<T>, u64, u8, RingBalance<T>), DispatchError>
		{
			let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)?;
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
					.to_legacy_receipt()
					.logs
					.into_iter()
					.find(|x| {
						x.address == <DepositRedeemAddress<T>>::get()
							&& x.topics[0] == eth_event.signature()
					})
					.ok_or(<Error<T>>::LogEntryNE)?;
				let log = RawLog {
					topics: vec![log_entry.topics[0], log_entry.topics[1]],
					data: log_entry.data.clone(),
				};

				eth_event.parse_log(log).map_err(|_| <Error<T>>::EthLogPF)?
			};
			let deposit_id = result.params[0].value.clone().into_uint().ok_or(<Error<T>>::IntCF)?;
			let months = {
				let months = result.params[2].value.clone().into_uint().ok_or(<Error<T>>::IntCF)?;

				months.saturated_into()
			};
			// The start_at here is in seconds, will be converted to milliseconds later in
			// on_deposit_redeem
			let start_at = {
				let start_at =
					result.params[3].value.clone().into_uint().ok_or(<Error<T>>::IntCF)?;

				start_at.saturated_into()
			};
			let redeemed_ring = {
				// The decimal in Ethereum is 10**18, and the decimal in Darwinia is 10**9,
				// div 10**18 and mul 10**9
				let redeemed_ring = result.params[5]
					.value
					.clone()
					.into_uint()
					.map(|x| x / U256::from(1_000_000_000u64))
					.ok_or(<Error<T>>::IntCF)?;

				<RingBalance<T>>::saturated_from(redeemed_ring.saturated_into::<u128>())
			};
			let darwinia_account = {
				let raw_account_id =
					result.params[6].value.clone().into_bytes().ok_or(<Error<T>>::BytesCF)?;
				log::trace!("[ethereum-backing] Raw Account: {:?}", raw_account_id);

				Self::account_id_try_from_bytes(&raw_account_id)?
			};
			log::trace!("[ethereum-backing] Darwinia Account: {:?}", darwinia_account);

			Ok((deposit_id, darwinia_account, redeemed_ring, start_at, months, fee))
		}

		// event SetAuthoritiesEvent(uint32 nonce, address[] authorities, bytes32 benefit);
		// https://github.com/darwinia-network/darwinia-bridge-on-ethereum/blob/51839e614c0575e431eabfd5c70b84f6aa37826a/contracts/Relay.sol#L22
		// https://ropsten.etherscan.io/tx/0x652528b9421ecb495610a734a4ab70d054b5510dbbf3a9d5c7879c43c7dde4e9#eventlog
		fn parse_authorities_set_proof(
			proof_record: &EthereumReceiptProofThing<T>,
		) -> Result<(Term, Vec<EthereumAddress>, AccountId<T>), DispatchError> {
			let log = {
				let verified_receipt = T::EthereumRelay::verify_receipt(proof_record)?;
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
					.to_legacy_receipt()
					.logs
					.into_iter()
					.find(|x| {
						x.address == <SetAuthoritiesAddress<T>>::get()
							&& x.topics[0] == eth_event.signature()
					})
					.ok_or(<Error<T>>::LogEntryNE)?;

				eth_event
					.parse_log(RawLog { topics: vec![topics[0]], data })
					.map_err(|_| <Error<T>>::EthLogPF)?
			};
			let term = log.params[0]
				.value
				.clone()
				.into_uint()
				.ok_or(<Error<T>>::BytesCF)?
				.saturated_into();
			let authorities = {
				let mut authorities = vec![];

				for token in log.params[1].value.clone().into_array().ok_or(<Error<T>>::ArrayCF)? {
					authorities.push(token.into_address().ok_or(<Error<T>>::AddressCF)?);
				}

				authorities
			};
			let beneficiary = {
				let raw_account_id =
					log.params[2].value.clone().into_fixed_bytes().ok_or(<Error<T>>::BytesCF)?;

				log::trace!("[ethereum-backing] Raw Account: {:?}", raw_account_id);

				Self::account_id_try_from_bytes(&raw_account_id)?
			};

			Ok((term, authorities, beneficiary))
		}
	}
	impl<T: Config> Sign<BlockNumberFor<T>> for Pallet<T> {
		type Message = EcdsaMessage;
		type Signature = EcdsaSignature;
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

	#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum RedeemFor {
		Token,
		Deposit,
	}
}
pub use pallet::*;

pub mod migration {
	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		pub fn pre_migrate() -> Result<(), &'static str> {
			Ok(())
		}
	}

	pub fn migrate() {}
}
