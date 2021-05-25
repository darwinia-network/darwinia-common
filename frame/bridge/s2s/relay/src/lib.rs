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
	decl_error, decl_event, decl_module, decl_storage,
    traits::Get,
	PalletId,
	weights::Weight,
};

use sp_runtime::DispatchError;

pub mod weights;
pub use weights::WeightInfo;
use darwinia_relay_primitives::Relay;

use darwinia_asset_primitives::token::Token;
use ethereum_primitives::EthereumAddress;

mod types {
	use crate::*;
	pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
	pub type AccountId<T> = <T as frame_system::Config>::AccountId;
    pub type RelayMessage = (Token, EthereumAddress);
}

pub trait Config: frame_system::Config {
	/// The ethereum-relay's module id, used for deriving its sovereign account ID.
	type PalletId: Get<PalletId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
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
	pub enum Error for Module<T: Config> {
        /// The proof is not in backing list
		InvalidProof,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Substrate2SubstrateRelay {
		pub BackingAddressList
			get(fn backing_address_list)
			: map hasher(identity) AccountId<T> => Option<EthereumAddress>;
    }

    add_extra_genesis {
		config(backings): Vec<(AccountId<T>, EthereumAddress)>;
		build(|config: &GenesisConfig<T>| {
			for (address, account) in &config.backings {
				<BackingAddressList<T>>::insert(address, &account);
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call
    where
		origin: T::Origin
	{
		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			0
		}
    }
}

impl<T: Config> Relay for Module<T> {
    type RelayProof = AccountId<T>;
    type RelayMessage = RelayMessage;
    type VerifiedResult = Result<EthereumAddress, DispatchError>;
    fn verify(proof: &Self::RelayProof) -> Self::VerifiedResult {
        let address = <BackingAddressList<T>>::get(proof).ok_or(<Error<T>>::InvalidProof)?;
        Ok(address)
    }

    // todo, use s2s relay message transaction
    fn relay_message(_message: &Self::RelayMessage) {
    }
}

