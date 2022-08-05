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
// --- crates.io ---
use rand::{seq::SliceRandom, Rng};
// --- paritytech ---
use sc_chain_spec::{ChainType, GenericChainSpec, Properties};
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::Perbill;
// --- darwinia-network ---
use super::*;
use drml_primitives::*;
use fp_evm::GenesisAccount;
use pangoro_runtime::*;

pub type ChainSpec = GenericChainSpec<GenesisConfig, Extensions>;

const PANGORO_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const ECDSA_AUTHORITY: &str = "0x68898db1012808808c903f390909c52d9f706749";
const EVM_ACCOUNTS: &[&str] = &[
	"0x68898db1012808808c903f390909c52d9f706749",
	"0x6be02d1d3665660d22ff9624b7be0551ee1ac91b",
	"0xB90168C8CBcd351D069ffFdA7B71cd846924d551",
	// Echo
	"0x0f14341A7f464320319025540E8Fe48Ad0fe5aec",
	// for External Project
	"0x7682Ba569E3823Ca1B7317017F5769F8Aa8842D4",
	// Subswap
	"0xbB3E51d20CA651fBE19b1a1C2a6C8B1A4d950437",
];

const A_FEW_COINS: Balance = 1 << 44;
const MANY_COINS: Balance = A_FEW_COINS << 6;
const BUNCH_OF_COINS: Balance = MANY_COINS << 6;

impl_authority_keys!();

pub fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	beefy: BeefyId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { babe, grandpa, beefy, im_online, authority_discovery }
}

pub fn properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("ss58Format".into(), 18.into());
	properties.insert("tokenDecimals".into(), vec![9, 9].into());
	properties.insert("tokenSymbol".into(), vec!["ORING", "OKTON"].into());

	properties
}

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/pangoro/pangoro.json")[..])
}

pub fn genesis_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let root = AccountId::from(array_bytes::hex2array_unchecked(
			"0x72819fbc1b93196fa230243947c1726cbea7e33044c7eb6f736ff345561f9e4c",
		));
		let initial_authorities = AuthorityKeys::testnet_authorities();
		let initial_nominators = <Vec<AccountId>>::new();

		let evm_accounts = {
			let mut map = BTreeMap::new();

			// for account in EVM_ACCOUNTS.iter() {
			// 	map.insert(
			// 		array_bytes::hex_into_unchecked(account),
			// 		GenesisAccount {
			// 			balance: (MANY_COINS * (10 as Balance).pow(9)).into(),
			// 			..Default::default()
			// 		},
			// 	);
			// }

			map
		};

		GenesisConfig {
			system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
			babe: BabeConfig { authorities: vec![], epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG) },
			balances: BalancesConfig {
				balances: vec![
					(root.clone(), BUNCH_OF_COINS),
					(get_account_id_from_seed::<sr25519::Public>("Alice"), A_FEW_COINS),
				]
				.into_iter()
				.chain(
					initial_authorities
						.iter()
						.map(|AuthorityKeys { stash_key, .. }| (stash_key.to_owned(), A_FEW_COINS)),
				)
				.chain(initial_nominators.iter().map(|n| (n.to_owned(), A_FEW_COINS)))
				.chain(
					TEAM_MEMBERS.iter().map(|m| (array_bytes::hex_into_unchecked(m), MANY_COINS)),
				)
				.collect(),
			},
			kton: KtonConfig {
				balances: vec![(root.clone(), BUNCH_OF_COINS)]
					.into_iter()
					.chain(
						initial_authorities.iter().map(|AuthorityKeys { stash_key, .. }| {
							(stash_key.to_owned(), A_FEW_COINS)
						}),
					)
					.chain(initial_nominators.iter().map(|n| (n.to_owned(), A_FEW_COINS)))
					.chain(
						TEAM_MEMBERS
							.iter()
							.map(|m| (array_bytes::hex_into_unchecked(m), A_FEW_COINS)),
					)
					.collect(),
			},
			staking: StakingConfig {
				minimum_validator_count: 2,
				validator_count: 4,
				stakers: initial_authorities
					.iter()
					.map(|AuthorityKeys { stash_key, .. }| {
						(
							stash_key.to_owned(),
							stash_key.to_owned(),
							A_FEW_COINS,
							StakerStatus::Validator,
						)
					})
					.chain(initial_nominators.iter().map(|n| {
						let mut rng = rand::thread_rng();
						let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
						let count = rng.gen::<usize>() % limit;
						let nominations = initial_authorities
							.as_slice()
							.choose_multiple(&mut rng, count)
							.into_iter()
							.map(|c| c.stash_key.clone())
							.collect::<Vec<_>>();

						(n.clone(), n.clone(), A_FEW_COINS, StakerStatus::Nominator(nominations))
					}))
					.collect(),
				slash_reward_fraction: Perbill::from_percent(10),
				payout_fraction: Perbill::from_percent(50),
				..Default::default()
			},
			session: SessionConfig {
				keys: initial_authorities
					.iter()
					.map(|AuthorityKeys { stash_key, session_keys }| {
						(stash_key.to_owned(), stash_key.to_owned(), session_keys.to_owned())
					})
					.collect(),
			},
			grandpa: Default::default(),
			beefy: Default::default(),
			message_gadget: Default::default(),
			ecdsa_authority: EcdsaAuthorityConfig {
				authorities: vec![array_bytes::hex_into_unchecked(ECDSA_AUTHORITY)],
			},
			im_online: Default::default(),
			authority_discovery: Default::default(),
			treasury: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			substrate_2_substrate_backing: Substrate2SubstrateBackingConfig {
				secure_limited_period: DAYS,
				secure_limited_ring_amount: 1_000_000 * COIN,
				remote_mapping_token_factory_account: Default::default(),
			},
			evm: EVMConfig { accounts: evm_accounts },
			ethereum: Default::default(),
			base_fee: Default::default(),
		}
	}

	ChainSpec::from_genesis(
		"Pangoro",
		"pangoro",
		ChainType::Live,
		genesis,
		[
			"/dns4/g1.pangoro-p2p.darwinia.network/tcp/30333/p2p/12D3KooWLc6ZD4PGjnRz8CuVioW1dEr8rVBVEAFb1vpxFHXU4g2Y",
			"/dns4/g2.pangoro-p2p.darwinia.network/tcp/30333/p2p/12D3KooWHf1v45q3u1qPrkwSUq7ybzNfXf5ELPcpoBTJ4k49axfk",
			"/dns4/g3.pangoro-p2p.darwinia.network/tcp/30333/p2p/12D3KooWCXW7Ds6invyE1rF4BSfwpMgNKzzBxbnEGGjcqZ6cSgap",
			"/dns4/g4.pangoro-p2p.darwinia.network/tcp/30333/p2p/12D3KooWHokmaoAJp2vVPkw2YG3HFa799RUAJvdfy4dcaEzBdkGw",
		]
		.iter()
		.filter_map(|s| FromStr::from_str(s).ok())
		.collect(),
		Some(
			TelemetryEndpoints::new(vec![(PANGORO_TELEMETRY_URL.to_string(), 0)])
				.expect("Pangoro telemetry url is valid; qed"),
		),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties()),
		Default::default(),
	)
}

pub fn development_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let root = get_account_id_from_seed::<sr25519::Public>("Alice");
		let initial_authorities = vec![get_authority_keys_from_seed("Alice")];
		let endowed_accounts = vec![
			root.clone(),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		]
		.into_iter()
		.chain(TEAM_MEMBERS.iter().map(|m| array_bytes::hex_into_unchecked(m)))
		.collect::<Vec<_>>();

		let evm_accounts = {
			let mut map = BTreeMap::new();

			// for account in EVM_ACCOUNTS.iter() {
			// 	map.insert(
			// 		array_bytes::hex_into_unchecked(account),
			// 		GenesisAccount {
			// 			balance: (123_456_789_000_000_000_000_090 as Balance).into(),
			// 			..Default::default()
			// 		},
			// 	);
			// }

			map
		};

		GenesisConfig {
			system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
			babe: BabeConfig { authorities: vec![], epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG) },
			balances: BalancesConfig {
				balances: endowed_accounts.clone().into_iter().map(|a| (a, MANY_COINS)).collect(),
			},
			kton: KtonConfig {
				balances: endowed_accounts.clone().into_iter().map(|a| (a, A_FEW_COINS)).collect(),
			},
			staking: StakingConfig {
				minimum_validator_count: 1,
				validator_count: 2,
				stakers: initial_authorities
					.iter()
					.cloned()
					.map(|x| (x.0, x.1, A_FEW_COINS, StakerStatus::Validator))
					.collect(),
				invulnerables: initial_authorities.iter().cloned().map(|x| x.0).collect(),
				force_era: Forcing::ForceNew,
				slash_reward_fraction: Perbill::from_percent(10),
				payout_fraction: Perbill::from_percent(50),
				..Default::default()
			},
			session: SessionConfig {
				keys: initial_authorities
					.iter()
					.cloned()
					.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5, x.6)))
					.collect(),
			},
			grandpa: Default::default(),
			beefy: Default::default(),
			message_gadget: Default::default(),
			ecdsa_authority: EcdsaAuthorityConfig {
				authorities: vec![array_bytes::hex_into_unchecked(ECDSA_AUTHORITY)],
			},
			im_online: Default::default(),
			authority_discovery: Default::default(),
			treasury: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			substrate_2_substrate_backing: Substrate2SubstrateBackingConfig {
				secure_limited_period: DAYS,
				secure_limited_ring_amount: 100_000 * COIN,
				remote_mapping_token_factory_account: Default::default(),
			},
			evm: EVMConfig { accounts: evm_accounts },
			ethereum: Default::default(),
			base_fee: Default::default(),
		}
	}

	ChainSpec::from_genesis(
		"Pangoro Development Testnet",
		"pangoro_dev",
		ChainType::Development,
		genesis,
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties()),
		Default::default(),
	)
}
