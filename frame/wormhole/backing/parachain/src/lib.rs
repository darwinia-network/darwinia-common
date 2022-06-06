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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for s2s cross chain assets backing.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weight;
pub use weight::WeightInfo;

// --- crates.io ---
use ethereum_types::H256;
// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_messages::{
	source_chain::{MessagesBridge, OnDeliveryConfirmed},
	BridgeMessageId, DeliveredMessages, LaneId, MessageNonce,
};
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement::*, Get},
	transactional,
	weights::PostDispatchInfo,
	PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*, RawOrigin};
use sp_runtime::{
	traits::{AccountIdConversion, Convert, Saturating, Zero},
	DispatchErrorWithPostInfo, MultiSignature, MultiSigner,
};
use sp_std::prelude::*;
// --- darwinia-network ---
use darwinia_support::s2s::{ensure_source_account, LatestMessageNoncer};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

/// The parameters box for the pallet runtime call.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum IssuingCall<T: pallet::Config> {
	#[codec(index = 0)]
	ParachainIssuingPalletIssueFromRemote(RingBalance<T>, AccountId<T>),
}

pub trait IssueFromRemotePayload<
	SourceChainAccountId,
	TargetChainAccountPublic,
	TargetChainSignature,
	T: pallet::Config,
>
{
	type Payload: Encode;
	fn create(
		origin: CallOrigin<SourceChainAccountId, TargetChainAccountPublic, TargetChainSignature>,
		spec_version: u32,
		weight: u64,
		call: IssuingCall<T>,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str>;
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The pallet id of this pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The max lock amount per transaction for security.
		#[pallet::constant]
		type MaxLockRingAmountPerTx: Get<RingBalance<Self>>;

		/// The *RING* currency.
		type RingCurrency: Currency<AccountId<Self>>;

		/// The bridge account id converter.
		/// `remote account` + `remote chain id` derive the new account
		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;

		/// The bridged chain id
		type BridgedChainId: Get<ChainId>;

		/// Outbound payload creator used for s2s message
		type OutboundPayloadCreator: Parameter
			+ IssueFromRemotePayload<Self::AccountId, MultiSigner, MultiSignature, Self>;

		/// The message noncer to get the message nonce from the bridge
		type MessageNoncer: LatestMessageNoncer;

		/// The lane id of the s2s bridge
		type MessageLaneId: Get<LaneId>;

		/// The message bridge instance to send message
		type MessagesBridge: MessagesBridge<
			Self::AccountId,
			RingBalance<Self>,
			<<Self as Config>::OutboundPayloadCreator as IssueFromRemotePayload<
				Self::AccountId,
				MultiSigner,
				MultiSignature,
				Self,
			>>::Payload,
			Error = DispatchErrorWithPostInfo<PostDispatchInfo>,
		>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Token locked \[lane_id, message_nonce, token address, sender, recipient, amount\]
		TokenLocked(LaneId, MessageNonce, AccountId<T>, AccountId<T>, RingBalance<T>),
		/// Token unlocked \[lane_id, message_nonce, recipient, amount\]
		TokenUnlocked(LaneId, MessageNonce, AccountId<T>, RingBalance<T>),
		/// Token locked confirmed from remote \[lane_id, message_nonce, user, amount, result\]
		TokenLockedConfirmed(LaneId, MessageNonce, AccountId<T>, RingBalance<T>, bool),
		/// Update remote mapping token factory address \[account\]
		RemoteMappingFactoryAddressUpdated(AccountId<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance.
		InsufficientBalance,
		/// Ring Lock LIMITED.
		RingLockLimited,
		/// Redeem Daily Limited
		RingDailyLimited,
		/// Message nonce duplicated.
		NonceDuplicated,
	}

	/// Period between security limitation. Zero means there is no period limitation.
	#[pallet::storage]
	#[pallet::getter(fn secure_limited_period)]
	pub type SecureLimitedPeriod<T> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// `(Spent, Maximum)` amount of *RING* security limitation each [`LimitedPeriod`].
	#[pallet::storage]
	#[pallet::getter(fn secure_limited_ring_amount)]
	pub type SecureLimitedRingAmount<T> =
		StorageValue<_, (RingBalance<T>, RingBalance<T>), ValueQuery>;

	/// `(sender, amount)` the user *sender* lock and remote issuing amount of asset
	#[pallet::storage]
	#[pallet::getter(fn transaction_infos)]
	pub type TransactionInfos<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BridgeMessageId,
		(AccountId<T>, RingBalance<T>),
		OptionQuery,
	>;

	/// The remote mapping token factory account, here use to ensure the remote caller
	#[pallet::storage]
	#[pallet::getter(fn remote_mapping_token_factory_account)]
	pub type RemoteMappingTokenFactoryAccount<T: Config> =
		StorageValue<_, AccountId<T>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub secure_limited_period: BlockNumberFor<T>,
		pub secure_limited_ring_amount: RingBalance<T>,
		pub remote_mapping_token_factory_account: AccountId<T>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				secure_limited_period: Zero::zero(),
				secure_limited_ring_amount: Zero::zero(),
				remote_mapping_token_factory_account: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<SecureLimitedPeriod<T>>::put(self.secure_limited_period);
			<SecureLimitedRingAmount<T>>::put((
				<RingBalance<T>>::zero(),
				self.secure_limited_ring_amount,
			));
			<RemoteMappingTokenFactoryAccount<T>>::put(
				self.remote_mapping_token_factory_account.clone(),
			);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let secure_limited_period = <SecureLimitedPeriod<T>>::get();

			if !secure_limited_period.is_zero() && (now % secure_limited_period).is_zero() {
				<SecureLimitedRingAmount<T>>::mutate(|(used, _)| *used = Zero::zero());

				T::DbWeight::get().reads_writes(2, 1)
			} else {
				T::DbWeight::get().reads(1)
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Lock token in this chain and cross transfer to the target chain
		///
		/// Target is the id of the target chain defined in s2s_chain pallet
		#[pallet::weight(<T as Config>::WeightInfo::lock_and_remote_issue())]
		#[transactional]
		pub fn lock_and_remote_issue(
			origin: OriginFor<T>,
			spec_version: u32,
			weight: u64,
			#[pallet::compact] value: RingBalance<T>,
			#[pallet::compact] fee: RingBalance<T>,
			recipient: AccountId<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// Make sure the locked value is less than the max lock limited
			ensure!(
				value < T::MaxLockRingAmountPerTx::get() && !value.is_zero(),
				<Error<T>>::RingLockLimited
			);
			// Make sure the user's balance is enough to lock
			let total_cost = value + fee;
			ensure!(
				T::RingCurrency::free_balance(&user) > total_cost,
				<Error<T>>::InsufficientBalance
			);

			// this pallet account as the submitter of the remote message
			// we need to transfer fee from user to this account to pay the bridge fee
			T::RingCurrency::transfer(&user, &Self::pallet_account_id(), total_cost, KeepAlive)?;

			let payload = T::OutboundPayloadCreator::create(
				CallOrigin::SourceAccount(Self::pallet_account_id()),
				spec_version,
				weight,
				IssuingCall::<T>::ParachainIssuingPalletIssueFromRemote(value, recipient.clone()),
				DispatchFeePayment::AtSourceChain,
			)?;
			T::MessagesBridge::send_message(
				RawOrigin::Signed(Self::pallet_account_id()),
				T::MessageLaneId::get(),
				payload,
				fee,
			)?;

			let message_nonce =
				T::MessageNoncer::outbound_latest_generated_nonce(T::MessageLaneId::get());
			let message_id: BridgeMessageId = (T::MessageLaneId::get(), message_nonce);
			ensure!(!<TransactionInfos<T>>::contains_key(message_id), Error::<T>::NonceDuplicated);
			<TransactionInfos<T>>::insert(message_id, (user.clone(), value));
			Self::deposit_event(Event::TokenLocked(
				T::MessageLaneId::get(),
				message_nonce,
				user,
				recipient,
				value,
			));
			Ok(().into())
		}

		/// Receive target chain locked message and unlock token in this chain.
		#[pallet::weight(<T as Config>::WeightInfo::unlock_from_remote())]
		pub fn unlock_from_remote(
			origin: OriginFor<T>,
			amount: RingBalance<T>,
			recipient: AccountId<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// Check call origin
			ensure_source_account::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				<RemoteMappingTokenFactoryAccount<T>>::get(),
				&user,
			)?;

			// Make sure the total transfer is less than the security limitation
			{
				let (used, limitation) = <SecureLimitedRingAmount<T>>::get();
				ensure!(
					<SecureLimitedPeriod<T>>::get().is_zero()
						|| used.saturating_add(amount) <= limitation,
					<Error<T>>::RingDailyLimited
				);
			}
			// Make sure the user's balance is enough to lock
			ensure!(
				T::RingCurrency::free_balance(&Self::pallet_account_id()) > amount,
				<Error<T>>::InsufficientBalance
			);

			T::RingCurrency::transfer(&Self::pallet_account_id(), &recipient, amount, KeepAlive)?;
			<SecureLimitedRingAmount<T>>::mutate(|(used, _)| *used = used.saturating_add(amount));
			let message_nonce =
				T::MessageNoncer::inbound_latest_received_nonce(T::MessageLaneId::get()) + 1;
			Self::deposit_event(Event::TokenUnlocked(
				T::MessageLaneId::get(),
				message_nonce,
				recipient,
				amount,
			));

			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_secure_limited_period())]
		pub fn set_secure_limited_period(
			origin: OriginFor<T>,
			period: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<SecureLimitedPeriod<T>>::put(period);

			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_security_limitation_ring_amount())]
		pub fn set_security_limitation_ring_amount(
			origin: OriginFor<T>,
			limitation: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<SecureLimitedRingAmount<T>>::mutate(|(_, limitation_)| *limitation_ = limitation);

			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_remote_mapping_token_factory_account())]
		pub fn set_remote_mapping_token_factory_account(
			origin: OriginFor<T>,
			account: AccountId<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<RemoteMappingTokenFactoryAccount<T>>::put(account.clone());
			Self::deposit_event(Event::RemoteMappingFactoryAddressUpdated(account));

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn pallet_account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}
	}

	impl<T: Config> OnDeliveryConfirmed for Pallet<T> {
		fn on_messages_delivered(lane: &LaneId, messages: &DeliveredMessages) -> Weight {
			if *lane != T::MessageLaneId::get() {
				return 0;
			}
			for nonce in messages.begin..=messages.end {
				let result = messages.message_dispatch_result(nonce);
				if let Some((user, amount)) = <TransactionInfos<T>>::take((*lane, nonce)) {
					if !result {
						// if remote issue mapped token failed, this fund need to transfer token
						// back to the user. The balance always comes from the user's locked
						// currency while calling the dispatch call `lock_and_remote_issue`.
						// This transfer will always successful except some extreme scene, since the
						// user must lock some currency first, then this transfer can be triggered.
						let _ = T::RingCurrency::transfer(
							&Self::pallet_account_id(),
							&user,
							amount,
							KeepAlive,
						);
					}
					Self::deposit_event(Event::TokenLockedConfirmed(
						*lane, nonce, user, amount, result,
					));
				}
			}
			// TODO: The returned weight should be more accurately. See: https://github.com/darwinia-network/darwinia-common/issues/911
			<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
		}
	}
}
