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
use std::{collections::BTreeMap, marker::PhantomData, str::FromStr};
// --- crates.io ---
use rand::{seq::SliceRandom, Rng};
// --- paritytech ---
use sc_chain_spec::{ChainType, GenericChainSpec, Properties};
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::Perbill;
// --- darwinia-network ---
use super::*;
use darwinia_bridge_ethereum::DagsMerkleRootsLoader as DagsMerkleRootsLoaderR;
use darwinia_claims::ClaimsList;
use darwinia_evm::GenesisAccount;
use drml_common_primitives::*;
use pangolin_runtime::*;

pub type ChainSpec = GenericChainSpec<GenesisConfig, Extensions>;

const PANGOLIN_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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

const S2S_RELAYER: &str = "ec7c1c10c73a2d90c6a4fc92a5212caaff849a65193db3a2b2aa1ffdadb99f06";
const TOKEN_REDEEM_ADDRESS: &str = "0x49262B932E439271d05634c32978294C7Ea15d0C";
const DEPOSIT_REDEEM_ADDRESS: &str = "0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e";
const SET_AUTHORITIES_ADDRESS: &str = "0xD35Bb6F1bc1C84b53E0995c1830454AB7C4147f1";
const RING_TOKEN_ADDRESS: &str = "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0";
const KTON_TOKEN_ADDRESS: &str = "0x1994100c58753793D52c6f457f189aa3ce9cEe94";
const ETHEREUM_RELAY_AUTHORITY_SIGNER: &str = "0x68898db1012808808c903f390909c52d9f706749";
const MAPPING_FACTORY_ADDRESS: &str = "0xE1586e744b99bF8e4C981DfE4dD4369d6f8Ed88A";
const ETHEREUM_BACKING_ADDRESS: &str = "0xb2Bea2358d817dAE01B0FD0DC3aECB25910E65AA";

impl_authority_keys!();

pub fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	beefy: BeefyId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys {
		babe,
		grandpa,
		beefy,
		im_online,
		authority_discovery,
	}
}

pub fn properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenDecimals".into(), vec![9, 9].into());
	properties.insert("tokenSymbol".into(), vec!["PRING", "PKTON"].into());

	properties
}

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/pangolin/pangolin.json")[..])
}

pub fn genesis_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let root = AccountId::from(array_bytes::hex2array_unchecked(
			"0x72819fbc1b93196fa230243947c1726cbea7e33044c7eb6f736ff345561f9e4c",
		));
		let s2s_relayer = array_bytes::hex_into_unchecked(S2S_RELAYER);
		let initial_authorities = AuthorityKeys::testnet_authorities();
		let initial_nominators = <Vec<AccountId>>::new();
		let collective_members = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
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
				epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG)
			},
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
				.chain(
					initial_nominators
						.iter()
						.map(|n| (n.to_owned(), A_FEW_COINS))
				)
				.chain(
					TEAM_MEMBERS
						.iter()
						.map(|m| (array_bytes::hex_into_unchecked(m), MANY_COINS)),
				)
				.collect()
			},
			kton: KtonConfig {
				balances: vec![(root.clone(), BUNCH_OF_COINS), (s2s_relayer, BUNCH_OF_COINS)]
					.into_iter()
					.chain(
						initial_authorities
							.iter()
							.map(|AuthorityKeys { stash_key, .. }| (stash_key.to_owned(), A_FEW_COINS)),
					)
					.chain(
						initial_nominators
							.iter()
							.map(|n| (n.to_owned(), A_FEW_COINS))
					)
					.chain(
						TEAM_MEMBERS
							.iter()
							.map(|m| (array_bytes::hex_into_unchecked(m), A_FEW_COINS)),
					)
					.collect()
			},
			staking: StakingConfig {
				minimum_validator_count: 6,
				validator_count: 6,
				stakers: initial_authorities
					.iter()
					.map(|AuthorityKeys { stash_key, .. }| (
						stash_key.to_owned(),
						stash_key.to_owned(),
						A_FEW_COINS,
						StakerStatus::Validator
					))
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
					.map(|AuthorityKeys { stash_key, session_keys }| (
						stash_key.to_owned(),
						stash_key.to_owned(),
						session_keys.to_owned()
					))
					.collect(),
			},
			grandpa: Default::default(),
			beefy: Default::default(),
			im_online: Default::default(),
			authority_discovery: Default::default(),
			democracy: Default::default(),
			council: Default::default(),
			technical_committee: Default::default(),
			phragmen_election: PhragmenElectionConfig {
				members: collective_members
					.iter()
					.cloned()
					.map(|a| (a, A_FEW_COINS))
					.collect(),
			},
			technical_membership: TechnicalMembershipConfig {
				phantom: PhantomData::<TechnicalMembershipInstance>,
				members: collective_members.clone(),
			},
			treasury: Default::default(),
			kton_treasury: Default::default(),
			claims: Default::default(),
			vesting: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			ethereum_relay: EthereumRelayConfig {
				genesis_header_parcel: r#"{
					"header": {
						"baseFeePerGas": "0xeb",
						"difficulty": "0x4186f54e",
						"extraData": "0xd883010a06846765746888676f312e31352e36856c696e7578",
						"gasLimit": "0x7a1200",
						"gasUsed": "0x5e949",
						"hash": "0x9db735cdbe337477d38b70d96998decb9d8ea1d796cdc6c97546132978db668c",
						"logsBloom": "0x00200000000000000000000080000000000000004000001000010000000000000000000000000000000000000000000000000000000000000000000008000000040000000020400000004008000020200000010000000000004000008000000000000400020000800100000000000800080000000000400000000010000000000000000000000000004000000080000000000081010000080000004000200000000080000020000000000000000000000000200000080000000000000000000000000006000000000000000000000000000000200000001000002000000020000000000000000000000a00000000200000002000000000400000000000000000",
						"miner": "0xfbb61b8b98a59fbc4bd79c23212addbefaeb289f",
						"mixHash": "0xbb166a439393a562d5c71973a7e3f1b87bc6bb65b1b2524e846b021c6c170a16",
						"nonce": "0xee2e3a941040cee1",
						"number": "0xa367a4",
						"parentHash": "0xcaf94fe7cc38a012316dba0cc1296fa2ab3fb401aacef819c39aac934c29ef34",
						"receiptsRoot": "0x27f5405108f65bd36455ddddf2ce32fe2b87851be97fce3e5eff48636ee52f1e",
						"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
						"size": "0x794",
						"stateRoot": "0xfcd5f2e0b1a728dbb2112c21c375cdfe425568493dde3bb71d036509c404a236",
						"timestamp": "0x60fe2f75",
						"totalDifficulty": "0x79b2e0d1c5829f",
						"transactions": [],
						"transactionsRoot": "0x2169e889c51cc5605d055a54a3fb095a90a33db18fbcf28e86073fd33288fbb4",
						"uncles": []
					},
					"parent_mmr_root": "0x1183acf36ada5ca93e31e618e7632c3ed23eddf3cebf077eb868873d6212179a"
				}"#.into(),
				dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
					"bin/res/ethereum/dags-merkle-roots.json",
					"DAG_MERKLE_ROOTS_PATH",
				),
				..Default::default()
			},
			ethereum_backing: EthereumBackingConfig {
				token_redeem_address: array_bytes::hex_into_unchecked(TOKEN_REDEEM_ADDRESS),
				deposit_redeem_address: array_bytes::hex_into_unchecked(DEPOSIT_REDEEM_ADDRESS),
				set_authorities_address: array_bytes::hex_into_unchecked(SET_AUTHORITIES_ADDRESS),
				ring_token_address: array_bytes::hex_into_unchecked(RING_TOKEN_ADDRESS),
				kton_token_address: array_bytes::hex_into_unchecked(KTON_TOKEN_ADDRESS),
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			ethereum_issuing: EthereumIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
				ethereum_backing_address: array_bytes::hex_into_unchecked(ETHEREUM_BACKING_ADDRESS),
			},
			ethereum_relay_authorities: EthereumRelayAuthoritiesConfig {
				authorities: vec![(
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					array_bytes::hex_into_unchecked(ETHEREUM_RELAY_AUTHORITY_SIGNER),
					1
				)]
			},
			tron_backing: TronBackingConfig {
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			evm: EVMConfig { accounts: evm_accounts },
			ethereum: Default::default(),
			substrate_2_substrate_issuing: Substrate2SubstrateIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
			},
			bsc: BSCConfig {
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
		"Pangolin",
		"pangolin",
		ChainType::Live,
		genesis,
		[
			"/dns4/t1.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWLc6ZD4PGjnRz8CuVioW1dEr8rVBVEAFb1vpxFHXU4g2Y",
			"/dns4/t2.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWHf1v45q3u1qPrkwSUq7ybzNfXf5ELPcpoBTJ4k49axfk",
			"/dns4/t3.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWCXW7Ds6invyE1rF4BSfwpMgNKzzBxbnEGGjcqZ6cSgap",
			"/dns4/t4.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWHokmaoAJp2vVPkw2YG3HFa799RUAJvdfy4dcaEzBdkGw",
			"/dns4/t5.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWGJM9oAV95rM67Vad7j7jZGcH7mRoXM4R3gFNYGWE8Nsj",
			"/dns4/t6.pangolin-p2p.darwinia.network/tcp/30333/p2p/12D3KooWKhUXATik7HPz7EC3865dd7XihbnbCA3ciVjuvPv3YXwr"
		]
		.iter()
		.filter_map(|s| FromStr::from_str(s).ok())
		.collect(),
		Some(
			TelemetryEndpoints::new(vec![(PANGOLIN_TELEMETRY_URL.to_string(), 0)])
				.expect("Pangolin telemetry url is valid; qed"),
		),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties()),
		Default::default(),
	)
}

pub fn development_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let root = get_account_id_from_seed::<sr25519::Public>("Alice");
		let s2s_relayer = array_bytes::hex_into_unchecked(S2S_RELAYER);
		let initial_authorities = vec![get_authority_keys_from_seed("Alice")];
		let endowed_accounts = vec![
			root.clone(),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			s2s_relayer,
		]
		.into_iter()
		.chain(
			TEAM_MEMBERS
				.iter()
				.map(|m| array_bytes::hex_into_unchecked(m)),
		)
		.collect::<Vec<_>>();
		let collective_members = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
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
				epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG)
			},
			balances: BalancesConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, MANY_COINS))
					.collect()
			},
			kton: KtonConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, A_FEW_COINS))
					.collect()
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
					.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5, x.6)))
					.collect(),
			},
			grandpa: Default::default(),
			beefy: Default::default(),
			im_online: Default::default(),
			authority_discovery: Default::default(),
			democracy: Default::default(),
			council: Default::default(),
			technical_committee: Default::default(),
			phragmen_election: PhragmenElectionConfig {
				members: collective_members
					.iter()
					.cloned()
					.map(|a| (a, A_FEW_COINS))
					.collect(),
			},
			technical_membership: TechnicalMembershipConfig {
				phantom: PhantomData::<TechnicalMembershipInstance>,
				members: collective_members.clone(),
			},
			treasury: Default::default(),
			kton_treasury: Default::default(),
			claims: ClaimsConfig {
				claims_list: ClaimsList::from_file(
					"bin/res/claims-list.json",
					"CLAIMS_LIST_PATH",
				),
			},
			vesting: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			ethereum_relay: EthereumRelayConfig {
				genesis_header_parcel: r#"{
					"header": {
						"baseFeePerGas": "0xeb",
						"difficulty": "0x4186f54e",
						"extraData": "0xd883010a06846765746888676f312e31352e36856c696e7578",
						"gasLimit": "0x7a1200",
						"gasUsed": "0x5e949",
						"hash": "0x9db735cdbe337477d38b70d96998decb9d8ea1d796cdc6c97546132978db668c",
						"logsBloom": "0x00200000000000000000000080000000000000004000001000010000000000000000000000000000000000000000000000000000000000000000000008000000040000000020400000004008000020200000010000000000004000008000000000000400020000800100000000000800080000000000400000000010000000000000000000000000004000000080000000000081010000080000004000200000000080000020000000000000000000000000200000080000000000000000000000000006000000000000000000000000000000200000001000002000000020000000000000000000000a00000000200000002000000000400000000000000000",
						"miner": "0xfbb61b8b98a59fbc4bd79c23212addbefaeb289f",
						"mixHash": "0xbb166a439393a562d5c71973a7e3f1b87bc6bb65b1b2524e846b021c6c170a16",
						"nonce": "0xee2e3a941040cee1",
						"number": "0xa367a4",
						"parentHash": "0xcaf94fe7cc38a012316dba0cc1296fa2ab3fb401aacef819c39aac934c29ef34",
						"receiptsRoot": "0x27f5405108f65bd36455ddddf2ce32fe2b87851be97fce3e5eff48636ee52f1e",
						"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
						"size": "0x794",
						"stateRoot": "0xfcd5f2e0b1a728dbb2112c21c375cdfe425568493dde3bb71d036509c404a236",
						"timestamp": "0x60fe2f75",
						"totalDifficulty": "0x79b2e0d1c5829f",
						"transactions": [],
						"transactionsRoot": "0x2169e889c51cc5605d055a54a3fb095a90a33db18fbcf28e86073fd33288fbb4",
						"uncles": []
					},
					"parent_mmr_root": "0x1183acf36ada5ca93e31e618e7632c3ed23eddf3cebf077eb868873d6212179a"
				}"#.into(),
				dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
					"bin/res/ethereum/dags-merkle-roots.json",
					"DAG_MERKLE_ROOTS_PATH",
				),
				..Default::default()
			},
			ethereum_backing: EthereumBackingConfig {
				token_redeem_address: array_bytes::hex_into_unchecked(TOKEN_REDEEM_ADDRESS),
				deposit_redeem_address: array_bytes::hex_into_unchecked(DEPOSIT_REDEEM_ADDRESS),
				set_authorities_address: array_bytes::hex_into_unchecked(SET_AUTHORITIES_ADDRESS),
				ring_token_address: array_bytes::hex_into_unchecked(RING_TOKEN_ADDRESS),
				kton_token_address: array_bytes::hex_into_unchecked(KTON_TOKEN_ADDRESS),
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			ethereum_issuing: EthereumIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
				ethereum_backing_address: array_bytes::hex_into_unchecked(ETHEREUM_BACKING_ADDRESS),
			},
			ethereum_relay_authorities: EthereumRelayAuthoritiesConfig {
				authorities: vec![(
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					array_bytes::hex_into_unchecked(ETHEREUM_RELAY_AUTHORITY_SIGNER),
					1
				)]
			},
			tron_backing: TronBackingConfig {
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			evm: EVMConfig { accounts: evm_accounts },
			ethereum: Default::default(),
			substrate_2_substrate_issuing: Substrate2SubstrateIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
			},
			bsc: BSCConfig {
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
		"Pangolin",
		"pangolin_dev",
		ChainType::Development,
		genesis,
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties()),
		Default::default(),
	)
}

pub fn local_testnet_config() -> ChainSpec {
	fn genesis() -> GenesisConfig {
		let root = get_account_id_from_seed::<sr25519::Public>("Alice");
		let s2s_relayer = array_bytes::hex_into_unchecked(S2S_RELAYER);
		let initial_authorities = vec![
			get_authority_keys_from_seed("Alice"),
			get_authority_keys_from_seed("Bob"),
			get_authority_keys_from_seed("Charlie"),
			get_authority_keys_from_seed("Dave"),
			get_authority_keys_from_seed("Eve"),
			get_authority_keys_from_seed("Ferdie"),
		];
		let endowed_accounts = vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
			s2s_relayer,
		]
		.into_iter()
		.chain(
			TEAM_MEMBERS
				.iter()
				.map(|m| array_bytes::hex_into_unchecked(m)),
		)
		.collect::<Vec<_>>();
		let collective_members = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
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
				epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG)
			},
			balances: BalancesConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, MANY_COINS))
					.collect()
			},
			kton: KtonConfig {
				balances: endowed_accounts
					.clone()
					.into_iter()
					.map(|a| (a, A_FEW_COINS))
					.collect()
			},
			staking: StakingConfig {
				minimum_validator_count: 6,
				validator_count: 6,
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
					.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5, x.6)))
					.collect(),
			},
			grandpa: Default::default(),
			beefy: Default::default(),
			im_online: Default::default(),
			authority_discovery: Default::default(),
			democracy: Default::default(),
			council: Default::default(),
			technical_committee: Default::default(),
			phragmen_election: PhragmenElectionConfig {
				members: collective_members
					.iter()
					.cloned()
					.map(|a| (a, A_FEW_COINS))
					.collect(),
			},
			technical_membership: TechnicalMembershipConfig {
				phantom: PhantomData::<TechnicalMembershipInstance>,
				members: collective_members.clone(),
			},
			treasury: Default::default(),
			kton_treasury: Default::default(),
			claims: ClaimsConfig {
				claims_list: ClaimsList::from_file(
					"bin/res/claims-list.json",
					"CLAIMS_LIST_PATH",
				),
			},
			vesting: Default::default(),
			sudo: SudoConfig { key: root.clone() },
			ethereum_relay: EthereumRelayConfig {
				genesis_header_parcel: r#"{
					"header": {
						"baseFeePerGas": "0xeb",
						"difficulty": "0x4186f54e",
						"extraData": "0xd883010a06846765746888676f312e31352e36856c696e7578",
						"gasLimit": "0x7a1200",
						"gasUsed": "0x5e949",
						"hash": "0x9db735cdbe337477d38b70d96998decb9d8ea1d796cdc6c97546132978db668c",
						"logsBloom": "0x00200000000000000000000080000000000000004000001000010000000000000000000000000000000000000000000000000000000000000000000008000000040000000020400000004008000020200000010000000000004000008000000000000400020000800100000000000800080000000000400000000010000000000000000000000000004000000080000000000081010000080000004000200000000080000020000000000000000000000000200000080000000000000000000000000006000000000000000000000000000000200000001000002000000020000000000000000000000a00000000200000002000000000400000000000000000",
						"miner": "0xfbb61b8b98a59fbc4bd79c23212addbefaeb289f",
						"mixHash": "0xbb166a439393a562d5c71973a7e3f1b87bc6bb65b1b2524e846b021c6c170a16",
						"nonce": "0xee2e3a941040cee1",
						"number": "0xa367a4",
						"parentHash": "0xcaf94fe7cc38a012316dba0cc1296fa2ab3fb401aacef819c39aac934c29ef34",
						"receiptsRoot": "0x27f5405108f65bd36455ddddf2ce32fe2b87851be97fce3e5eff48636ee52f1e",
						"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
						"size": "0x794",
						"stateRoot": "0xfcd5f2e0b1a728dbb2112c21c375cdfe425568493dde3bb71d036509c404a236",
						"timestamp": "0x60fe2f75",
						"totalDifficulty": "0x79b2e0d1c5829f",
						"transactions": [],
						"transactionsRoot": "0x2169e889c51cc5605d055a54a3fb095a90a33db18fbcf28e86073fd33288fbb4",
						"uncles": []
					},
					"parent_mmr_root": "0x1183acf36ada5ca93e31e618e7632c3ed23eddf3cebf077eb868873d6212179a"
				}"#.into(),
				dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
					"bin/res/ethereum/dags-merkle-roots.json",
					"DAG_MERKLE_ROOTS_PATH",
				),
				..Default::default()
			},
			ethereum_backing: EthereumBackingConfig {
				token_redeem_address: array_bytes::hex_into_unchecked(TOKEN_REDEEM_ADDRESS),
				deposit_redeem_address: array_bytes::hex_into_unchecked(DEPOSIT_REDEEM_ADDRESS),
				set_authorities_address: array_bytes::hex_into_unchecked(SET_AUTHORITIES_ADDRESS),
				ring_token_address: array_bytes::hex_into_unchecked(RING_TOKEN_ADDRESS),
				kton_token_address: array_bytes::hex_into_unchecked(KTON_TOKEN_ADDRESS),
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			ethereum_issuing: EthereumIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
				ethereum_backing_address: array_bytes::hex_into_unchecked(ETHEREUM_BACKING_ADDRESS),
			},
			ethereum_relay_authorities: EthereumRelayAuthoritiesConfig {
				authorities: vec![(
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					array_bytes::hex_into_unchecked(ETHEREUM_RELAY_AUTHORITY_SIGNER),
					1
				)]
			},
			tron_backing: TronBackingConfig {
				backed_ring: BUNCH_OF_COINS,
				backed_kton: BUNCH_OF_COINS,
			},
			evm: EVMConfig { accounts: evm_accounts },
			ethereum: Default::default(),
			substrate_2_substrate_issuing: Substrate2SubstrateIssuingConfig {
				mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
			},
			bsc: BSCConfig {
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
		"Pangolin",
		"pangolin_dev",
		ChainType::Development,
		genesis,
		vec![
			"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
				.parse()
				.unwrap(),
			"/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD"
				.parse()
				.unwrap(),
			"/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x"
				.parse()
				.unwrap(),
			"/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st"
				.parse()
				.unwrap(),
			"/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2"
				.parse()
				.unwrap(),
			"/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk"
				.parse()
				.unwrap(),
		],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties()),
		Default::default(),
	)
}
