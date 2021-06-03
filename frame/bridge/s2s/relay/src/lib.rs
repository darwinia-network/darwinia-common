// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
	}

	use frame_support::{traits::Get, weights::Weight, PalletId};
	pub use types::*;

	use sp_runtime::DispatchError;

	use darwinia_relay_primitives::{Relay, RelayAccount};
	use darwinia_support::traits::CallToPayload;

	use darwinia_asset_primitives::token::Token;
	use darwinia_s2s_chain::ChainSelector;
	use ethereum_primitives::EthereumAddress;
	use frame_system::RawOrigin;
	use sp_runtime::traits::AccountIdConversion;

	use bp_runtime::Size;
	use frame_support::{pallet_prelude::*, Parameter};

	use pallet_bridge_messages::MessageSender;

	use crate::weights::WeightInfo;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The ethereum-relay's module id, used for deriving its sovereign account ID.
		type PalletId: Get<PalletId>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		type OutboundPayload: Parameter + Size;

		type OutboundMessageFee: From<u64>;

		type CallToPayload: CallToPayload<AccountId<Self>, Self::OutboundPayload>;

		type MessageSenderT: MessageSender<
			Self::Origin,
			OutboundPayload = Self::OutboundPayload,
			OutboundMessageFee = Self::OutboundMessageFee,
		>;
	}

	#[pallet::event]
	#[pallet::metadata(
		AccountId<T> = "AccountId",
	)]
	pub enum Event<T: Config> {
		/// new message relayed
		NewMessageRelayed(AccountId<T>, u8),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The proof is not in backing list
		InvalidProof,
		/// Invalid Backing address
		InvalidBackingAddr,
		/// Encode Invalid
		EncodeInv,
	}

	#[pallet::storage]
	#[pallet::getter(fn backing_address_list)]
	pub type BackingAddressList<T> = StorageMap<
		_,
		Blake2_128Concat,
		AccountId<T>,
		Option<(EthereumAddress, ChainSelector)>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub backings: Vec<(AccountId<T>, EthereumAddress, ChainSelector)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				backings: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (address, account, selector) in &self.backings {
				<BackingAddressList<T>>::insert(address, Some((account, selector)));
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumber<T>) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Relay for Pallet<T> {
		type RelayProof = AccountId<T>;
		type RelayMessage = (ChainSelector, Token, RelayAccount<AccountId<T>>);
		type VerifiedResult = Result<(EthereumAddress, ChainSelector), DispatchError>;
		type RelayMessageResult = Result<(), DispatchError>;
		fn verify(proof: &Self::RelayProof) -> Self::VerifiedResult {
			let address = <BackingAddressList<T>>::get(proof).ok_or(<Error<T>>::InvalidProof)?;
			Ok(address)
		}

		fn relay_message(message: &Self::RelayMessage) -> Self::RelayMessageResult {
			let msg = message.clone();
			let encoded = darwinia_s2s_chain::encode_relay_message(msg.0, msg.1, msg.2)
				.map_err(|_| <Error<T>>::EncodeInv)?;
			let relay_id: AccountId<T> = T::PalletId::get().into_account();
			let payload = T::CallToPayload::to_payload(relay_id.clone(), encoded);
			T::MessageSenderT::raw_send_message(
				RawOrigin::Signed(relay_id).into(),
				[0; 4],
				payload,
				0.into(),
			)?;
			Ok(())
		}
	}
}

pub use pallet::*;
