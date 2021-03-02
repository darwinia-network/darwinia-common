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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for cross chain assets issuing.

#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]

// --- substrate ---
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
    dispatch::DispatchResultWithPostInfo,
};
use frame_system::{ensure_root, ensure_signed};
use ethereum_types::{H160, H256, U256};
use dvm_ethereum::TransactionAction;
use core::str::FromStr;
use rustc_hex::{FromHex, ToHex};
use dvm_ethereum::TransactionSignature;

use darwinia_support::{
	traits::SystemDvmCaller as SystemDvmCallerT,
};

mod types {
    pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
}

use types::*;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type SysdvmCaller: SystemDvmCallerT<dvm_ethereum::Transaction, DispatchResultWithPostInfo>;
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
        /// test
        Test(AccountId),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthereumIssuing {
		pub TestNumber get(fn test_number): u128 = 1001;
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		#[weight = 10_000_000]
        pub fn set_number(origin, number: u128) {
			ensure_signed(origin)?;
			TestNumber::put(number);
        }

        #[weight = 10_000_000]
        pub fn test_call(origin) {
            debug::info!(target: "darwinia-issuing", "start to call tx");
			ensure_signed(origin)?;
            let transaction = dvm_ethereum::Transaction {
                nonce: U256::zero(),
                gas_price: U256::from(1),
                gas_limit: U256::from(0x100000),
                action: dvm_ethereum::TransactionAction::Call(H160::from_str("55D8ECEE33841AaCcb890085AcC7eE0d8A92b5eF").unwrap()),
                value: U256::zero(),
                input: FromHex::from_hex("40c10f190000000000000000000000004ad6e21bef59268f2ccf10bfa18c20c8c13ed8590000000000000000000000000000000000000000000000000de0b6b3a7640000").unwrap(),
                signature: TransactionSignature::new(
                    0x78,
                    H256::from_slice(&[55u8; 32]),
                    H256::from_slice(&[55u8; 32]),
                ).unwrap(),
            };
            let result = T::SysdvmCaller::sys_transact(transaction);
            debug::info!(
                target: "darwinia-issuing",
                "sys call return {:?}",
                result
            );

        }
    }
}

