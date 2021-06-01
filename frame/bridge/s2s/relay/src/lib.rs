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

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, traits::Get, weights::Weight, PalletId,
};

use sp_runtime::DispatchError;

pub mod weights;
use darwinia_relay_primitives::{Relay, RelayAccount};
use darwinia_support::traits::CallToPayload;
pub use weights::WeightInfo;

use darwinia_asset_primitives::token::Token;
use darwinia_s2s_chain::ChainSelector;
use ethereum_primitives::EthereumAddress;
use sp_runtime::traits::AccountIdConversion;
use frame_system::RawOrigin;

use frame_support::Parameter;
use bp_runtime::Size;

use pallet_bridge_messages::MessageSender;

mod types {
	pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
	pub type AccountId<T> = <T as frame_system::Config>::AccountId;
}

pub trait Config: frame_system::Config {
	/// The ethereum-relay's module id, used for deriving its sovereign account ID.
	type PalletId: Get<PalletId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;

    type OutboundPayload: Parameter + Size;

    type OutboundMessageFee: From<u64>;

    type CallToPayload: CallToPayload<AccountId<Self>, Self::OutboundPayload>;

    type MessageSenderT: MessageSender<Self::Origin, OutboundPayload=Self::OutboundPayload, OutboundMessageFee=Self::OutboundMessageFee>;
}

use types::*;

decl_event! {
    pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
		/// new message relayed
		NewMessageRelayed(AccountId, u8),
	}
}

decl_error! {
    pub enum Error for Pallet<T: Config> {
		/// The proof is not in backing list
		InvalidProof,
		/// Invalid Backing address
		InvalidBackingAddr,
		/// Encode Invalid
		EncodeInv,
	}
}

decl_storage! {
    trait Store for Pallet<T: Config> as Substrate2SubstrateRelay {
		pub BackingAddressList
			get(fn backing_address_list)
			: map hasher(identity) AccountId<T> => Option<(EthereumAddress, ChainSelector)>;
	}

	add_extra_genesis {
		config(backings): Vec<(AccountId<T>, EthereumAddress, ChainSelector)>;
		build(|config| {
			for (address, account, selector) in &config.backings {
                <BackingAddressList<T>>::insert(address, (account, selector));
			}
		});
	}
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			0
		}
	}
}

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
        let issuing_id: AccountId<T> = T::PalletId::get().into_account();
        let payload = T::CallToPayload::to_payload(issuing_id.clone(), encoded);
        T::MessageSenderT::raw_send_message(
            RawOrigin::Signed(issuing_id).into(),
            [0; 4],
            payload,
            0.into(),
            )?;
		Ok(())
	}
}
