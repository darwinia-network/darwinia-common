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

#[macro_use]
extern crate alloc;

use ethereum_types::{
    U256,
    H160,
    H256,
    Address as EthereumAddress
};
use ethabi::{
    Param,
    param_type::ParamType,
    Function,
    token::Token,
    Result as AbiResult,
    Bytes,
    RawLog,
    EventParam,
    Error,
    ErrorKind,
};

pub use ethabi::{Log, Event};
use sp_std::vec::Vec;

pub struct Abi {
}

pub struct Address([u8; 20]);
pub struct Amount([u64; 4]);
pub struct Topic([u8; 32]);

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
}

impl From<Address> for EthereumAddress {
    fn from(addr: Address) -> EthereumAddress {
        H160(addr.0)
    }
}

impl From<[u64; 4]> for Amount {
    fn from(bytes: [u64; 4]) -> Self {
        Self(bytes)
    }
}

impl From<Amount> for U256  {
    fn from(value: Amount) -> U256 {
        U256(value.0)
    }
}

impl From<[u8; 32]> for Topic {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Topic> for H256 {
    fn from(topic: Topic) -> H256 {
        H256(topic.0)
    }
}

impl Abi {
    fn mint() -> Function {
        let inputs = vec![
            Param {
                name: "account".into(), 
                kind: ParamType::Address,
            },
            Param {
                name: "amount".into(),
                kind: ParamType::Uint(256),
            }];

        Function {
            name: "mint".into(),
            inputs: inputs,
            outputs: vec![],
            constant: false,
        }
    }

    fn create_erc20() -> Function {
        let inputs = vec![
            Param { name: "name".into(), kind: ParamType::String },
            Param { name: "symbol".into(), kind: ParamType::String },
            Param { name: "decimals".into(), kind: ParamType::Uint(8) },
            Param { name: "backing".into(), kind: ParamType::Address },
            Param { name: "source".into(), kind: ParamType::Address }
        ];

        let outputs = vec![
            Param {
                name: "token".into(),
                kind: ParamType::Address,
            }
        ];

        Function {
            name: "createERC20Contract".into(),
            inputs: inputs,
            outputs: outputs,
            constant: false,
        }
    }

    /// register event
    pub fn register_event() -> Event {
        Event {
            name: "NewTokenRegisted".into(),
            inputs: vec![
                EventParam {
                    name: "token".into(),
                    kind: ParamType::Address,
                    indexed: true,
                },
                EventParam {
                    name: "name".into(),
                    kind: ParamType::String,
                    indexed: false,
                },
                EventParam {
                    name: "symbol".into(),
                    kind: ParamType::String,
                    indexed: false,
                },
                EventParam {
                    name: "decimals".into(),
                    kind: ParamType::Uint(8),
                    indexed: false,
                }
            ],
            anonymous: false,
        }
    }

    /// backing event
    pub fn backing_event() -> Event {
        Event {
            name: "BackingLock".into(),
            inputs: vec![
                EventParam {
                    name: "token".into(),
                    kind: ParamType::Address,
                    indexed: true,
                },
                EventParam {
                    name: "target".into(),
                    kind: ParamType::Address,
                    indexed: false,
                },
                EventParam {
                    name: "amount".into(),
                    kind: ParamType::Uint(256),
                    indexed: false,
                },
                EventParam {
                    name: "recipient".into(),
                    kind: ParamType::Address,
                    indexed: false,
                },
            ],
            anonymous: false,
        }
    }

    /// encode mint function for erc20
    pub fn encode_mint(target: Address, amount: Amount) -> AbiResult<Bytes> {
        let mint = Self::mint();
        let account = Token::Address(target.into());
        let value = Token::Uint(amount.into());
        mint.encode_input(vec![account, value].as_slice())
    }

    /// encode erc20 create function
    pub fn encode_create_erc20(
        name: &str,
        symbol: &str,
        decimals: u8,
        backing: Address,
        source: Address) -> AbiResult<Bytes> {
        let create = Self::create_erc20();
        create.encode_input(
            vec![
            Token::String(name.into()),
            Token::String(symbol.into()),
            Token::Uint(U256::from(decimals)),
            Token::Address(backing.into()),
            Token::Address(source.into())
            ].as_slice())
    }

    /// parse token register event
    pub fn parse_event(topics: Vec<Topic>, data: Vec<u8>, eth_event: Event) -> AbiResult<Log> {
        //let eth_event = Self::register_event();
        let log = RawLog {
            topics: topics
                .into_iter()
                .map(|t| -> H256 {
                    t.into()
                }).collect(),
            data: data.clone(),
        };
        eth_event.parse_log(log)
    }

    /// get mapped token from source
    pub fn mapping_token() -> Function {
        let inputs = vec![
            Param { name: "backing".into(), kind: ParamType::Address },
            Param { name: "source".into(), kind: ParamType::Address }
        ];

        let outputs = vec![
            Param {
                name: "target".into(),
                kind: ParamType::Address,
            }
        ];

        Function {
            name: "mappingToken".into(),
            inputs: inputs,
            outputs: outputs,
            constant: true,
        }
    }

    /// encode mapping token function
    pub fn encode_mapping_token (
        backing: Address,
        source: Address) -> AbiResult<Bytes> {
        let mapping = Self::mapping_token();
        mapping.encode_input(
            vec![
            Token::Address(backing.into()),
            Token::Address(source.into())
            ].as_slice())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TokenRegisterInfo(pub H160, pub H160, pub H160);

impl TokenRegisterInfo {
    pub fn decode(data: &[u8]) -> AbiResult<Self> {
        let tokens = ethabi::decode(
            &[
                ParamType::Address,
                ParamType::Address,
                ParamType::Address
            ], &data)?;
        match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone()) {
            (Token::Address(backing), Token::Address(source), Token::Address(target)) => {
                Ok(TokenRegisterInfo(backing, source, target))
            },
            _ => Err(Error::ErrorKind(ErrorKind::InvalidData))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TokenBurnInfo {
    pub backing: H160,
    pub source: H160,
    pub recipient: H160,
    pub amount: U256,
    pub is_native: bool,
}

impl TokenBurnInfo {
    pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
            &[
                ParamType::Address, 
                ParamType::Address,
                ParamType::Address,
                ParamType::Uint(256),
                ParamType::Bool
            ], &data)?;
        match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone(), tokens[3].clone(), tokens[4].clone(),) {
            (Token::Address(backing), Token::Address(source), Token::Address(recipient), Token::Uint(amount), Token::Bool(is_native)) => Ok(TokenBurnInfo {
                backing: backing,
                source: source,
                recipient: recipient,
                amount: amount,
                is_native: is_native,
            }),
            _ => Err(Error::ErrorKind(ErrorKind::InvalidData))
        }
    }
}

