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

#![cfg_attr(not(feature = "std"), no_std)]

use ethereum_types::{U256, H160};

pub struct Abi {
}

pub struct Address([u8; 20]);
pub struct Amount([u64; 4]);

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Address {
        Self(bytes)
    }
}

impl From<Address> for ethereum_types::Address {
    fn from(addr: Address) -> ethereum_types::Address {
        H160(addr.0)
    }
}

impl From<[u64; 4]> for Amount {
    fn from(bytes: [u64; 4]) -> Amount {
        Self(bytes)
    }
}

impl From<Amount> for U256  {
    fn from(value: Amount) -> U256 {
        U256(value.0)
    }
}

impl Abi {
    /// get mint function
    pub fn mint_function() -> ethabi::Function {
        let inputs = vec![
            ethabi::Param {
                name: "account".into(), 
                kind: ethabi::param_type::ParamType::Address,
            },
            ethabi::Param {
                name: "amount".into(),
                kind: ethabi::param_type::ParamType::Uint(256),
            }];

        ethabi::Function {
            name: "mint".into(),
            inputs: inputs,
            outputs: vec![],
            constant: false,
        }
    }

    pub fn encode_mint(target: Address, amount: Amount) -> ethabi::Result<ethabi::Bytes> {
        let mint = Self::mint_function();
        let account = ethabi::token::Token::Address(target.into());
        let value = ethabi::token::Token::Uint(amount.into());
        mint.encode_input(vec![account, value].as_slice())
    }
}
