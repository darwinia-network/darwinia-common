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
use darwinia_evm::GenesisAccount;
use darwinia_staking::StakerStatus;
use drml_common_primitives::*;
use pangoro_runtime::*;

pub type ChainSpec = GenericChainSpec<GenesisConfig, Extensions>;

const PANGORO_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
		];
		let initial_nominators = <Vec<AccountId>>::new();

		let evm_accounts = {
			let mut map = BTreeMap::new();

			for account in EVM_ACCOUNTS.iter() {
				map.insert(
					array_bytes::hex_into_unchecked(account),
					GenesisAccount {
						balance: (MANY_COINS * (10 as Balance).pow(9)).into(),
						..Default::default()
					},
				);
			}

			map
		};

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
				minimum_validator_count: 2,
				validator_count: 4,
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
			evm: EVMConfig {
				accounts: evm_accounts,
			},
			ethereum: Default::default(),
			base_fee: Default::default(),
			bsc: BscConfig {
				genesis_header: serde_json::from_str(r#"{
					"difficulty": "0x2",
					"extraData": "0xd683010108846765746886676f312e3137856c696e757800000000005865ba3c049153b8dae0a232ac90d20c78f1a5d1de7b7dc51284214b9b9c85549ab3d2b972df0deef66ac2c935552c16704d214347f29fa77f77da6d75d7c7524df189c73c714dd636a99aa4f3317ccd72a05d62980a75ecd1309ea12fa2ed87a8744fbfc9b863d5a2959d3f95eae5dc7d70144ce1b73b403b7eb6e0adac84746417fbfba17480e6cbc1360bca54330eb71b214cb885500844365e95cd9942c7276e7fd8f474cf03cceff28abc65c9cbae594f725c80e12d862d7205d7212bea8032fe2d71b6e182e72ccf9a18e2bddb9ffdfc189587c2f20a17afce0ccf13fb0b7eab75bffb94483caf97cdb63751c0fa85ba6120e397ed00",
					"gasLimit": "0x1c9c380",
					"gasUsed": "0xdd591",
					"logsBloom": "0x00020000000000000040000000004000810000000000002000004000000000000008000000200000200000000000100000010000000000000000000001002008000000100000800000000008000000002010000000000000a0000000000040000000002000024000040000000000000408400100010000000002001000080100200000002000000001000000000000000000244002400000800000000000002000000002000000000000008000000000000000000000000000000000000000000000000a004000000000400000000000000008000008040000000002000000000000000000000200010000000000008000010000000000400000080000000400",
					"miner": "0xa2959d3f95eae5dc7d70144ce1b73b403b7eb6e0",
					"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"nonce": "0x0000000000000000",
					"number": "0xfe37b0",
					"parentHash": "0x2a9a3f5769fbc24bd736eb5dc81ca663dd1f8b6a9f294fedfb4d43d9194c11dd",
					"receiptsRoot": "0x61bf3a54af15912a2e3661ae5458265a48567c1bf41e9c54638d5f6ed7cba594",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"stateRoot": "0x0556c6bfceb4595b11f1efd339c6a0841ab1009c8de1c821bdfc05175e9ecf31",
					"timestamp": "0x62064e60",
					"transactionsRoot": "0x71ec83a4e5176046dab6255a6c3afd6cf8144ab5907d4431d521144b2174373a"
				}"#).unwrap()
			},

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
		.chain(
			TEAM_MEMBERS
				.iter()
				.map(|m| array_bytes::hex_into_unchecked(m)),
		)
		.collect::<Vec<_>>();

		let evm_accounts = {
			let mut map = BTreeMap::new();

			for account in EVM_ACCOUNTS.iter() {
				map.insert(
					array_bytes::hex_into_unchecked(account),
					GenesisAccount {
						balance: (123_456_789_000_000_000_000_090 as Balance).into(),
						..Default::default()
					},
				);
			}

			map
		};

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
			evm: EVMConfig {
				accounts: evm_accounts,
			},
			ethereum: Default::default(),
			base_fee: Default::default(),
			bsc: BscConfig {
				genesis_header: serde_json::from_str(r#"{
					"difficulty": "0x2",
					"extraData": "0xd683010108846765746886676f312e3137856c696e757800000000005865ba3c049153b8dae0a232ac90d20c78f1a5d1de7b7dc51284214b9b9c85549ab3d2b972df0deef66ac2c935552c16704d214347f29fa77f77da6d75d7c7524df189c73c714dd636a99aa4f3317ccd72a05d62980a75ecd1309ea12fa2ed87a8744fbfc9b863d5a2959d3f95eae5dc7d70144ce1b73b403b7eb6e0adac84746417fbfba17480e6cbc1360bca54330eb71b214cb885500844365e95cd9942c7276e7fd8f474cf03cceff28abc65c9cbae594f725c80e12d862d7205d7212bea8032fe2d71b6e182e72ccf9a18e2bddb9ffdfc189587c2f20a17afce0ccf13fb0b7eab75bffb94483caf97cdb63751c0fa85ba6120e397ed00",
					"gasLimit": "0x1c9c380",
					"gasUsed": "0xdd591",
					"logsBloom": "0x00020000000000000040000000004000810000000000002000004000000000000008000000200000200000000000100000010000000000000000000001002008000000100000800000000008000000002010000000000000a0000000000040000000002000024000040000000000000408400100010000000002001000080100200000002000000001000000000000000000244002400000800000000000002000000002000000000000008000000000000000000000000000000000000000000000000a004000000000400000000000000008000008040000000002000000000000000000000200010000000000008000010000000000400000080000000400",
					"miner": "0xa2959d3f95eae5dc7d70144ce1b73b403b7eb6e0",
					"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"nonce": "0x0000000000000000",
					"number": "0xfe37b0",
					"parentHash": "0x2a9a3f5769fbc24bd736eb5dc81ca663dd1f8b6a9f294fedfb4d43d9194c11dd",
					"receiptsRoot": "0x61bf3a54af15912a2e3661ae5458265a48567c1bf41e9c54638d5f6ed7cba594",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"stateRoot": "0x0556c6bfceb4595b11f1efd339c6a0841ab1009c8de1c821bdfc05175e9ecf31",
					"timestamp": "0x62064e60",
					"transactionsRoot": "0x71ec83a4e5176046dab6255a6c3afd6cf8144ab5907d4431d521144b2174373a"
				}"#).unwrap()
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
