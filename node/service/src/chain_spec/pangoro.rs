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
					"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c0670403d7dfc4c816a313885fe04b850f96f27b2e9fd88b147c882ad7caf9b964abfe6543625fcca73b56fe29d3046831574b0681d52bf5383d6f2187b6276c100",
					"gasLimit": "0x38ff37a",
					"gasUsed": "0x1364017",
					"logsBloom": "0x2c30123db854d838c878e978cd2117896aa092e4ce08f078424e9ec7f2312f1909b35e579fb2702d571a3be04a8f01328e51af205100a7c32e3dd8faf8222fcf03f3545655314abf91c4c0d80cea6aa46f122c2a9c596c6a99d5842786d40667eb195877bbbb128890a824506c81a9e5623d4355e08a16f384bf709bf4db598bbcb88150abcd4ceba89cc798000bdccf5cf4d58d50828d3b7dc2bc5d8a928a32d24b845857da0b5bcf2c5dec8230643d4bec452491ba1260806a9e68a4a530de612e5c2676955a17400ce1d4fd6ff458bc38a8b1826e1c1d24b9516ef84ea6d8721344502a6c732ed7f861bb0ea017d520bad5fa53cfc67c678a2e6f6693c8ee",
					"miner": "0xe9ae3261a475a27bb1028f140bc2a7c843318afd",
					"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"nonce": "0x0000000000000000",
					"number": "0x7594c8",
					"parentHash": "0x5cb4b6631001facd57be810d5d1383ee23a31257d2430f097291d25fc1446d4f",
					"receiptsRoot": "0x1bfba16a9e34a12ff7c4b88be484ccd8065b90abea026f6c1f97c257fdb4ad2b",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"stateRoot": "0xa6cd7017374dfe102e82d2b3b8a43dbe1d41cc0e4569f3dc45db6c4e687949ae",
					"timestamp": "0x60ac7137",
					"transactionsRoot": "0x657f5876113ac9abe5cf0460aa8d6b3b53abfc336cea4ab3ee594586f8b584ca"
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
					"extraData": "0xd683010108846765746886676f312e3137856c696e757800000000005865ba3cd1d5f5f372bbfb5deeef7ce4b5637139b1a5d9a03e21496ddabac6a6e5d829a218ee28045873c8427cb5a77c2ce86815a3c110986c0a7ac15f560661e709ff9b01",
					"gasLimit": "0x1c7f9be",
					"gasUsed": "0x20f04d",
					"logsBloom": "0x0000100000000000044000000000000080000000001000000000400000000000100000000000000020000000100000000801040002400000000000000020000000000000000000100000080e000000002010000000000000000000800000400808080020020208000400000000000802084001000000080000000010000001000000000000200080000000000801000000000440000000000000000000900020020000020000000001000000000000000000042000000000000000000020000000000006000000008000000000000000002008000000000000000042000020400010000000000000610000000000008002000000000000080000000008000001",
					"miner": "0x1284214b9b9c85549ab3d2b972df0deef66ac2c9",
					"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"nonce": "0x0000000000000000",
					"number": "0xfd2c2b",
					"parentHash": "0x14e7e1cf1be0318b419a611f02c024533a9e52083fab41ca7009e69312a3e4a3",
					"receiptsRoot": "0xd12bb98573b478b7210e7387a09a8323c288b7b989b06d808b997ba8b4124e61",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"stateRoot": "0x6e008b9b78dbb45a9c2725c94a2ed46d6a77e187d052fcc1225ac4b3087e8295",
					"timestamp": "0x62032bb0",
					"transactionsRoot": "0x4a884d96f75c303ba9bd9add83e8e2b55b91bad0a0a1f953ae529fb8226daa62"
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
