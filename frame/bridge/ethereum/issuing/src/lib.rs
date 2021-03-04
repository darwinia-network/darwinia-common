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
    parameter_types,
};
use frame_system::{ensure_root, ensure_signed};
use ethereum_types::{H160, H256, U256};
use dvm_ethereum::TransactionAction;
use rustc_hex::{FromHex, ToHex};
use dvm_ethereum::TransactionSignature;

use sp_std::vec::Vec;

use sp_runtime::{
    DispatchError,
};

use darwinia_support::{
	traits::DvmRawTransactor as DvmRawTransactorT,
};

const ISSUING_ACCOUNT: &str = "1000000000000000000000000000000000000001";
const MAPPING_FACTORY_ADDRESS: &str = "55D8ECEE33841AaCcb890085AcC7eE0d8A92b5eF";

mod types {
    pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
}

use types::*;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type DvmCaller: DvmRawTransactorT<H160, dvm_ethereum::Transaction, DispatchResultWithPostInfo>;
}

decl_error! {
	/// Issuing pallet errors.
	pub enum Error for Module<T: Trait> {
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
	}
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
        pub fn test_call(origin, amount: U256) {
            debug::info!(target: "darwinia-issuing", "start to call tx");
			ensure_signed(origin)?;
            //let mint = Self::mint_function();
            let recvaddr: Vec<u8> = FromHex::from_hex(MAPPING_FACTORY_ADDRESS).unwrap();
            let receiver: ethereum_types::Address = H160::from_slice(&recvaddr);
            let bytes = darwinia_ethereum_issuing_contract::Abi::encode_mint(receiver.0.into(), amount.0.into());
            debug::info!(target: "darwinia-issuing", "mint bytes {:?}", bytes);
            let mapping_factory_address: Vec<u8> = FromHex::from_hex(MAPPING_FACTORY_ADDRESS).unwrap();
            let issuing_address: Vec<u8> = FromHex::from_hex(ISSUING_ACCOUNT).unwrap();

            let transaction = Self::unsigned_transaction(H160::from_slice(&mapping_factory_address), FromHex::from_hex("40c10f190000000000000000000000004ad6e21bef59268f2ccf10bfa18c20c8c13ed8590000000000000000000000000000000000000000000000000de0b6b3a7640000").unwrap());
            let issuing_account = H160::from_slice(&issuing_address);
            let result = T::DvmCaller::raw_transact(issuing_account, transaction).map_err(|e| -> &'static str {e.into()} )?;
            debug::info!(
                target: "darwinia-issuing",
                "sys call return {:?}",
                result
            );
        }
    }
}

impl<T: Trait> Module<T> {
    /// get dvm ethereum unsigned transaction
    pub fn unsigned_transaction(target: H160, input: Vec<u8>) -> dvm_ethereum::Transaction {
        dvm_ethereum::Transaction {
            nonce: U256::zero(),
            gas_price: U256::from(1),
            gas_limit: U256::from(0x100000),
            action: dvm_ethereum::TransactionAction::Call(target),
            value: U256::zero(),
            input: input,
            signature: TransactionSignature::new(
                0x0,
                H256::from_slice(&[0u8; 32]),
                H256::from_slice(&[0u8; 32]),
            ).unwrap(),
        }
    }

    // get mint function
    //pub fn mint_function() -> ethabi::Function {
        //let inputs = vec![
            //ethabi::Param {
                //name: "account".into(), 
                //kind: ethabi::param_type::ParamType::Address,
            //},
            //ethabi::Param {
                //name: "amount".into(),
                //kind: ethabi::param_type::ParamType::Uint(256),
            //}];

        //ethabi::Function {
            //name: "mint".into(),
            //inputs: inputs,
            //outputs: vec![],
            //constant: false,
        //}
    //}
}

