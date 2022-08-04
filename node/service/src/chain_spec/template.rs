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

// --- std ---
use std::{collections::BTreeMap, str::FromStr};
// --- paritytech ---
use sc_service::{ChainType, GenericChainSpec};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::sr25519;
use sp_finality_grandpa::AuthorityId as GrandpaId;
// --- darwinia-network ---
use super::*;
use darwinia_evm::GenesisAccount;
use template_runtime::*;

pub type ChainSpec = GenericChainSpec<GenesisConfig>;

pub fn development_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let initial_authorities = vec![get_authority_keys_from_seed("Alice")];
		let root_key = get_account_id_from_seed::<sr25519::Public>("Alice");
		let endowed_accounts = vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		];

		GenesisConfig {
			system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
			balances: BalancesConfig {
				balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
			},
			kton: KtonConfig {
				balances: endowed_accounts.clone().into_iter().map(|a| (a, 1 << 60)).collect(),
			},
			aura: AuraConfig {
				authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
			},
			grandpa: GrandpaConfig {
				authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
			},
			sudo: SudoConfig { key: root_key },
			evm: EVMConfig {
				accounts: {
					let mut map = BTreeMap::new();
					map.insert(
						array_bytes::hex_into_unchecked(
							"0x6be02d1d3665660d22ff9624b7be0551ee1ac91b",
						),
						GenesisAccount {
							balance: FromStr::from_str("0xffffffffffffffffffffffffffffffff")
								.unwrap(),
							..Default::default()
						},
					);
					map
				},
			},
			ethereum: EthereumConfig {},
			base_fee: Default::default(),
		}
	}

	ChainSpec::from_genesis(
		"Template",
		"template_dev",
		ChainType::Development,
		genesis,
		vec![],
		None,
		None,
		None,
		None,
	)
}

fn get_authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}
