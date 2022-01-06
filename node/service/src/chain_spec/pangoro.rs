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

// --- std ---
use std::str::FromStr;
// --- crates.io ---
use rand::{seq::SliceRandom, Rng};
// --- paritytech ---
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::{ChainType, GenericChainSpec, Properties};
use sc_telemetry::TelemetryEndpoints;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::Perbill;
// --- darwinia-network ---
use super::*;
use darwinia_staking::StakerStatus;
use drml_common_primitives::*;
use pangoro_runtime::*;

pub type ChainSpec = GenericChainSpec<GenesisConfig, Extensions>;

const PANGORO_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const A_FEW_COINS: Balance = 1 << 44;
const MANY_COINS: Balance = A_FEW_COINS << 6;
const BUNCH_OF_COINS: Balance = MANY_COINS << 6;

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn properties() -> Properties {
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
		struct Keys {
			stash: AccountId,
			session: SessionKeys,
		}
		impl Keys {
			fn new(sr25519: &str, ed25519: &str) -> Self {
				let sr25519 = array_bytes::hex2array_unchecked(sr25519);
				let ed25519 = array_bytes::hex2array_unchecked(ed25519);

				Self {
					stash: sr25519.into(),
					session: session_keys(
						sr25519.unchecked_into(),
						ed25519.unchecked_into(),
						sr25519.unchecked_into(),
						sr25519.unchecked_into(),
					),
				}
			}
		}

		let root = AccountId::from(array_bytes::hex2array_unchecked(
			"0x72819fbc1b93196fa230243947c1726cbea7e33044c7eb6f736ff345561f9e4c",
		));
		let initial_authorities = vec![
			Keys::new(
				"0x9c43c00407c0a51e0d88ede9d531f165e370013b648e6b62f4b3bcff4689df02",
				"0x63e122d962a835020bef656ad5a80dbcc994bb48a659f1af955552f4b3c27b09",
			),
			Keys::new(
				"0x741a9f507722713ec0a5df1558ac375f62469b61d1f60fa60f5dedfc85425b2e",
				"0x8a50704f41448fca63f608575debb626639ac00ad151a1db08af1368be9ccb1d",
			),
			Keys::new(
				"0x2276a3162f1b63c21b3396c5846d43874c5b8ba69917d756142d460b2d70d036",
				"0xb28fade2d023f08c0d5a131eac7d64a107a2660f22a0aca09b37a3f321259ef6",
			),
			Keys::new(
				"0x7a8b265c416eab5fdf8e5a1b3c7635131ca7164fbe6f66d8a70feeeba7c4dd7f",
				"0x305bafd512366e7fd535fdc144c7034b8683e1814d229c84a116f3cb27a97643",
			),
			Keys::new(
				"0xe446c1f1f419cc0927ad3319e141501b02844dee6252d905aae406f0c7097d1a",
				"0xc3c9880f6821b6e906c4396e54137297b1ee6c4c448b6a98abc5e29ffcdcec81",
			),
			Keys::new(
				"0xae05263d9508581f657ce584184721884ee2886eb66765db0c4f5195aa1d4e21",
				"0x1ed7de3855ffcce134d718b570febb49bbbbeb32ebbc8c319f44fb9f5690643a",
			),
		];
		let initial_nominators = <Vec<AccountId>>::new();

		GenesisConfig {
			system: SystemConfig {
				code: wasm_binary_unwrap().to_vec(),
				changes_trie_config: Default::default(),
			},
			babe: BabeConfig {
				authorities: vec![],
				epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG),
			},
			balances: BalancesConfig {
				balances: vec![
					(root.clone(), BUNCH_OF_COINS),
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						A_FEW_COINS,
					),
				]
				.into_iter()
				.chain(
					initial_authorities
						.iter()
						.map(|Keys { stash, .. }| (stash.to_owned(), A_FEW_COINS)),
				)
				.chain(
					initial_nominators
						.iter()
						.map(|n| (n.to_owned(), A_FEW_COINS)),
				)
				.chain(
					TEAM_MEMBERS
						.iter()
						.map(|m| (array_bytes::hex_into_unchecked(m), MANY_COINS)),
				)
				.collect(),
			},
			kton: KtonConfig {
				balances: vec![(root.clone(), BUNCH_OF_COINS)]
					.into_iter()
					.chain(
						initial_authorities
							.iter()
							.map(|Keys { stash, .. }| (stash.to_owned(), A_FEW_COINS)),
					)
					.chain(
						initial_nominators
							.iter()
							.map(|n| (n.to_owned(), A_FEW_COINS)),
					)
					.chain(
						TEAM_MEMBERS
							.iter()
							.map(|m| (array_bytes::hex_into_unchecked(m), A_FEW_COINS)),
					)
					.collect(),
			},
			staking: StakingConfig {
				minimum_validator_count: 6,
				validator_count: 6,
				stakers: initial_authorities
					.iter()
					.map(|Keys { stash, .. }| {
						(
							stash.to_owned(),
							stash.to_owned(),
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
							.map(|c| c.stash.clone())
							.collect::<Vec<_>>();

						(
							n.clone(),
							n.clone(),
							A_FEW_COINS,
							StakerStatus::Nominator(nominations),
						)
					}))
					.collect(),
				slash_reward_fraction: Perbill::from_percent(10),
				payout_fraction: Perbill::from_percent(50),
				..Default::default()
			},
			session: SessionConfig {
				keys: initial_authorities
					.iter()
					.map(|Keys { stash, session }| {
						(stash.to_owned(), stash.to_owned(), session.to_owned())
					})
					.collect(),
			},
			grandpa: Default::default(),
			im_online: Default::default(),
			authority_discovery: Default::default(),
			treasury: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			substrate_2_substrate_backing: Substrate2SubstrateBackingConfig {
				secure_limited_period: DAYS,
				secure_limited_ring_amount: 1_000_000 * COIN,
				remote_mapping_token_factory_account: Default::default(),
			},
		}
	}

	ChainSpec::from_genesis(
		"Pangoro",
		"pangoro",
		ChainType::Live,
		genesis,
		[
			"/dns4/t1.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWLc6ZD4PGjnRz8CuVioW1dEr8rVBVEAFb1vpxFHXU4g2Y",
			"/dns4/t2.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWHf1v45q3u1qPrkwSUq7ybzNfXf5ELPcpoBTJ4k49axfk",
			"/dns4/t3.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWCXW7Ds6invyE1rF4BSfwpMgNKzzBxbnEGGjcqZ6cSgap",
			"/dns4/t4.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWHokmaoAJp2vVPkw2YG3HFa799RUAJvdfy4dcaEzBdkGw",
			"/dns4/t5.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWGJM9oAV95rM67Vad7j7jZGcH7mRoXM4R3gFNYGWE8Nsj",
			"/dns4/t6.pangoro-p2p.darwinia.network/tcp/40333/p2p/12D3KooWKhUXATik7HPz7EC3865dd7XihbnbCA3ciVjuvPv3YXwr"
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
		.chain(
			TEAM_MEMBERS
				.iter()
				.map(|m| array_bytes::hex_into_unchecked(m)),
		)
		.collect::<Vec<_>>();

		GenesisConfig {
			system: SystemConfig {
				code: wasm_binary_unwrap().to_vec(),
				changes_trie_config: Default::default(),
			},
			babe: BabeConfig {
				authorities: vec![],
				epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG),
			},
			balances: BalancesConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, MANY_COINS))
					.collect(),
			},
			kton: KtonConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, A_FEW_COINS))
					.collect(),
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
				force_era: darwinia_staking::Forcing::ForceAlways,
				slash_reward_fraction: Perbill::from_percent(10),
				payout_fraction: Perbill::from_percent(50),
				..Default::default()
			},
			session: SessionConfig {
				keys: initial_authorities
					.iter()
					.cloned()
					.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5)))
					.collect(),
			},
			grandpa: Default::default(),
			im_online: Default::default(),
			authority_discovery: Default::default(),
			treasury: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			substrate_2_substrate_backing: Substrate2SubstrateBackingConfig {
				secure_limited_period: DAYS,
				secure_limited_ring_amount: 100_000 * COIN,
				remote_mapping_token_factory_account: Default::default(),
			},
		}
	}

	ChainSpec::from_genesis(
		"Pangoro",
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
