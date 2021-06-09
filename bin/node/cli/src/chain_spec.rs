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
use std::{collections::BTreeMap, marker::PhantomData};
// --- crates ---
use rand::{seq::SliceRandom, Rng};
// --- substrate ---
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
// --- darwinia ---
use darwinia_claims::ClaimsList;
use darwinia_ethereum_relay::DagsMerkleRootsLoader as DagsMerkleRootsLoaderR;
use darwinia_evm::GenesisAccount;
use drml_primitives::*;

pub type PangolinChainSpec = sc_service::GenericChainSpec<pangolin_runtime::GenesisConfig>;

type AccountPublic = <Signature as Verify>::Signer;

const PANGOLIN_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const TEAM_MEMBERS: &[&'static str] = &[
	// Huiyi
	"0x281b7ec1e05feb46457caa9c54cef0ebdaf7f65d31fd6ed740a34dbc9875304c",
	// Ron
	"0x9cf0c0ea7488a17e348f0abba9c229032f3240a793ffcfbedc4b46db0aeb306c",
	// Cheng
	"0x922b6854052ba1084c74dd323ee70047d58ae4eb068f20bc251831f1ec109030",
	// Jane
	"0xb26268877f72c4dcd9c2459a99dde0d2caf5a816c6b4cd3bd1721252b26f4909",
	// Cai
	"0xf41d3260d736f5b3db8a6351766e97619ea35972546a5f850bbf0b27764abe03",
	// Tiny
	"0xf29638cb649d469c317a4c64381e179d5f64ef4d30207b4c52f2725c9d2ec533",
	// Eve
	"0x1a7008a33fa595398b509ef56841db3340931c28a42881e36c9f34b1f15f9271",
	// Yuqi
	"0x500e3197e075610c1925ddcd86d66836bf93ae0a476c64f56f611afc7d64d16f",
	// Aki
	"0x129f002b1c0787ea72c31b2dc986e66911fe1b4d6dc16f83a1127f33e5a74c7d",
	// Alex
	"0x26fe37ba5d35ac650ba37c5cc84525ed135e772063941ae221a1caca192fff49",
	// Shell
	"0x187c272f576b1999d6cf3dd529b59b832db12125b43e57fb088677eb0c570a6b",
	// Xavier
	"0xb4f7f03bebc56ebe96bc52ea5ed3159d45a0ce3a8d7f082983c33ef133274747",
	// Xuelei
	"0x88d388115bd0df43e805b029207cfa4925cecfb29026e345979d9b0004466c49",
];
const EVM_ACCOUNTS: &[&'static str] = &[
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

const TOKEN_REDEEM_ADDRESS: &'static str = "0x49262B932E439271d05634c32978294C7Ea15d0C";
const DEPOSIT_REDEEM_ADDRESS: &'static str = "0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e";
const SET_AUTHORITIES_ADDRESS: &'static str = "0xD35Bb6F1bc1C84b53E0995c1830454AB7C4147f1";
const RING_TOKEN_ADDRESS: &'static str = "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0";
const KTON_TOKEN_ADDRESS: &'static str = "0x1994100c58753793D52c6f457f189aa3ce9cEe94";
const ETHEREUM_RELAY_AUTHORITY_SIGNER: &'static str = "0x68898db1012808808c903f390909c52d9f706749";
const MAPPING_FACTORY_ADDRESS: &'static str = "0xcB8531Bc0B7C8F41B55CF4E94698C37b130597B9";
const ETHEREUM_BACKING_ADDRESS: &'static str = "0xb2Bea2358d817dAE01B0FD0DC3aECB25910E65AA";

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> pangolin_runtime::SessionKeys {
	pangolin_runtime::SessionKeys {
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
	properties.insert("tokenSymbol".into(), vec!["PRING", "PKTON"].into());

	properties
}

fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn get_authority_keys_from_seed(
	s: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
		get_account_id_from_seed::<sr25519::Public>(s),
		get_from_seed::<BabeId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<ImOnlineId>(s),
		get_from_seed::<AuthorityDiscoveryId>(s),
	)
}

pub fn pangolin_config() -> Result<PangolinChainSpec, String> {
	PangolinChainSpec::from_json_bytes(&include_bytes!("../../../res/pangolin/pangolin.json")[..])
}

pub fn pangolin_build_spec_config() -> PangolinChainSpec {
	PangolinChainSpec::from_genesis(
		"Pangolin",
		"pangolin",
		ChainType::Live,
		pangolin_build_spec_genesis,
		vec![],
		Some(
			TelemetryEndpoints::new(vec![(PANGOLIN_TELEMETRY_URL.to_string(), 0)])
				.expect("Pangolin telemetry url is valid; qed"),
		),
		None,
		Some(properties()),
		None,
	)
}

fn pangolin_build_spec_genesis() -> pangolin_runtime::GenesisConfig {
	struct Keys {
		stash: AccountId,
		session: pangolin_runtime::SessionKeys,
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
	let collective_members = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let evm_accounts = {
		let mut map = BTreeMap::new();

		for account in EVM_ACCOUNTS.iter() {
			map.insert(
				array_bytes::hex_into_unchecked(account),
				GenesisAccount {
					nonce: 0.into(),
					balance: (MANY_COINS * (10 as Balance).pow(9)).into(),
					storage: BTreeMap::new(),
					code: vec![],
				},
			);
		}

		map
	};

	pangolin_runtime::GenesisConfig {
		frame_system: pangolin_runtime::SystemConfig {
			code: pangolin_runtime::wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_babe: pangolin_runtime::BabeConfig {
			authorities: vec![],
			epoch_config: Some(pangolin_runtime::BABE_GENESIS_EPOCH_CONFIG)
		},
		darwinia_balances_Instance1: pangolin_runtime::BalancesConfig {
			balances: vec![
				(root.clone(), BUNCH_OF_COINS),
				(get_account_id_from_seed::<sr25519::Public>("Alice"), A_FEW_COINS),
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
					.map(|n| (n.to_owned(), A_FEW_COINS))
			)
			.chain(
				TEAM_MEMBERS
					.iter()
					.map(|m| (array_bytes::hex_into_unchecked(m), MANY_COINS)),
			)
			.collect()
		},
		darwinia_balances_Instance2: pangolin_runtime::KtonConfig {
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
						.map(|n| (n.to_owned(), A_FEW_COINS))
				)
				.chain(
					TEAM_MEMBERS
						.iter()
						.map(|m| (array_bytes::hex_into_unchecked(m), A_FEW_COINS)),
				)
				.collect()
		},
		darwinia_staking: pangolin_runtime::StakingConfig {
			minimum_validator_count: 6,
			validator_count: 6,
			stakers: initial_authorities
				.iter()
				.map(|Keys { stash, .. }| (
					stash.to_owned(),
					stash.to_owned(),
					A_FEW_COINS,
					pangolin_runtime::StakerStatus::Validator
				))
				.chain(initial_nominators.iter().map(|n| {
					let mut rng = rand::thread_rng();
					let limit = (pangolin_runtime::MAX_NOMINATIONS as usize).min(initial_authorities.len());
					let count = rng.gen::<usize>() % limit;
					let nominations = initial_authorities
						.as_slice()
						.choose_multiple(&mut rng, count)
						.into_iter()
						.map(|c| c.stash.clone())
						.collect::<Vec<_>>();

					(n.clone(), n.clone(), A_FEW_COINS, pangolin_runtime::StakerStatus::Nominator(nominations))
				}))
				.collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		},
		pallet_session: pangolin_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|Keys { stash, session }| (
					stash.to_owned(),
					stash.to_owned(),
					session.to_owned()
				))
				.collect(),
		},
		pallet_grandpa: Default::default(),
		pallet_im_online: Default::default(),
		pallet_authority_discovery: Default::default(),
		darwinia_democracy: Default::default(),
		pallet_collective_Instance1: pangolin_runtime::CouncilConfig {
			phantom: PhantomData::<pangolin_runtime::CouncilCollective>,
			members: collective_members.clone(),
		},
		pallet_collective_Instance2: pangolin_runtime::TechnicalCommitteeConfig {
			phantom: PhantomData::<pangolin_runtime::TechnicalCollective>,
			members: collective_members
		},
		darwinia_elections_phragmen: Default::default(),
		pallet_membership_Instance1: Default::default(),
		darwinia_claims: Default::default(),
		darwinia_vesting: Default::default(),
		pallet_sudo: pangolin_runtime::SudoConfig { key: root.clone() },
		darwinia_crab_issuing: pangolin_runtime::CrabIssuingConfig { total_mapped_ring: BUNCH_OF_COINS },
		darwinia_crab_backing: pangolin_runtime::CrabBackingConfig { backed_ring: BUNCH_OF_COINS },
		darwinia_ethereum_relay: pangolin_runtime::EthereumRelayConfig {
			genesis_header_info: (
				vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 128, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 33, 123, 11, 188, 251, 114, 226, 213, 126, 40, 243, 60, 179, 97, 185, 152, 53, 19, 23, 119, 85, 220, 63, 51, 206, 62, 112, 34, 237, 98, 183, 123, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 132, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 136, 0, 0, 0, 0, 0, 0, 0, 66, 1, 65, 148, 16, 35, 104, 9, 35, 224, 254, 77, 116, 163, 75, 218, 200, 20, 31, 37, 64, 227, 174, 144, 98, 55, 24, 228, 125, 102, 209, 202, 74, 45],
				b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".into()
			),
			dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
				"bin/res/ethereum/dags-merkle-roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		},
		darwinia_ethereum_backing: pangolin_runtime::EthereumBackingConfig {
			token_redeem_address: array_bytes::hex_into_unchecked(TOKEN_REDEEM_ADDRESS),
			deposit_redeem_address: array_bytes::hex_into_unchecked(DEPOSIT_REDEEM_ADDRESS),
			set_authorities_address: array_bytes::hex_into_unchecked(SET_AUTHORITIES_ADDRESS),
			ring_token_address: array_bytes::hex_into_unchecked(RING_TOKEN_ADDRESS),
			kton_token_address: array_bytes::hex_into_unchecked(KTON_TOKEN_ADDRESS),
			backed_ring: BUNCH_OF_COINS,
			backed_kton: BUNCH_OF_COINS,
		},
		darwinia_ethereum_issuing: pangolin_runtime::EthereumIssuingConfig {
			mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
			ethereum_backing_address: array_bytes::hex_into_unchecked(ETHEREUM_BACKING_ADDRESS),
		},
		darwinia_relay_authorities_Instance1: pangolin_runtime::EthereumRelayAuthoritiesConfig {
			authorities: vec![(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				array_bytes::hex_into_unchecked(ETHEREUM_RELAY_AUTHORITY_SIGNER),
				1
			)]
		},
		darwinia_tron_backing: pangolin_runtime::TronBackingConfig {
			backed_ring: BUNCH_OF_COINS,
			backed_kton: BUNCH_OF_COINS,
		},
		darwinia_evm: pangolin_runtime::EVMConfig { accounts: evm_accounts },
		dvm_ethereum: Default::default(),
		darwinia_bridge_bsc: pangolin_runtime::BSCConfig {
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

pub fn pangolin_development_config() -> PangolinChainSpec {
	PangolinChainSpec::from_genesis(
		"Pangolin",
		"pangolin",
		ChainType::Development,
		pangolin_development_genesis,
		vec![],
		None,
		None,
		Some(properties()),
		None,
	)
}

fn pangolin_development_genesis() -> pangolin_runtime::GenesisConfig {
	let root = get_account_id_from_seed::<sr25519::Public>("Alice");
	let initial_authorities = vec![get_authority_keys_from_seed("Alice")];
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
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
					nonce: 0.into(),
					balance: (123_456_789_000_000_000_000_090 as Balance).into(),
					storage: BTreeMap::new(),
					code: vec![],
				},
			);
		}

		map
	};

	pangolin_runtime::GenesisConfig {
		frame_system: pangolin_runtime::SystemConfig {
			code: pangolin_runtime::wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_babe: pangolin_runtime::BabeConfig {
			authorities: vec![],
			epoch_config: Some(pangolin_runtime::BABE_GENESIS_EPOCH_CONFIG)
		},
		darwinia_balances_Instance1: pangolin_runtime::BalancesConfig {
			balances: endowed_accounts
				.clone()
				.into_iter()
				.map(|a| (a, MANY_COINS))
				.collect()
		},
		darwinia_balances_Instance2: pangolin_runtime::KtonConfig {
			balances: endowed_accounts
				.clone()
				.into_iter()
				.map(|a| (a, A_FEW_COINS))
				.collect()
		},
		darwinia_staking: pangolin_runtime::StakingConfig {
			minimum_validator_count: 1,
			validator_count: 2,
			stakers: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0, x.1, A_FEW_COINS, pangolin_runtime::StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().cloned().map(|x| x.0).collect(),
			force_era: darwinia_staking::Forcing::ForceAlways,
			slash_reward_fraction: Perbill::from_percent(10),
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		},
		pallet_session: pangolin_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5)))
				.collect(),
		},
		pallet_grandpa: Default::default(),
		pallet_im_online: Default::default(),
		pallet_authority_discovery: Default::default(),
		darwinia_democracy: Default::default(),
		pallet_collective_Instance1: pangolin_runtime::CouncilConfig {
			phantom: PhantomData::<pangolin_runtime::CouncilCollective>,
			members: collective_members.clone(),
		},
		pallet_collective_Instance2: pangolin_runtime::TechnicalCommitteeConfig {
			phantom: PhantomData::<pangolin_runtime::TechnicalCollective>,
			members: collective_members
		},
		darwinia_elections_phragmen: Default::default(),
		pallet_membership_Instance1: Default::default(),
		darwinia_claims: pangolin_runtime::ClaimsConfig {
			claims_list: ClaimsList::from_file(
				"bin/res/claims-list.json",
				"CLAIMS_LIST_PATH",
			),
		},
		darwinia_vesting: Default::default(),
		pallet_sudo: pangolin_runtime::SudoConfig { key: root.clone() },
		darwinia_crab_issuing: pangolin_runtime::CrabIssuingConfig { total_mapped_ring: BUNCH_OF_COINS },
		darwinia_crab_backing: pangolin_runtime::CrabBackingConfig { backed_ring: BUNCH_OF_COINS },
		darwinia_ethereum_relay: pangolin_runtime::EthereumRelayConfig {
			genesis_header_info: (
				vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 128, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 33, 123, 11, 188, 251, 114, 226, 213, 126, 40, 243, 60, 179, 97, 185, 152, 53, 19, 23, 119, 85, 220, 63, 51, 206, 62, 112, 34, 237, 98, 183, 123, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 132, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 136, 0, 0, 0, 0, 0, 0, 0, 66, 1, 65, 148, 16, 35, 104, 9, 35, 224, 254, 77, 116, 163, 75, 218, 200, 20, 31, 37, 64, 227, 174, 144, 98, 55, 24, 228, 125, 102, 209, 202, 74, 45],
				b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".into()
			),
			dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
				"bin/res/ethereum/dags-merkle-roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		},
		darwinia_ethereum_backing: pangolin_runtime::EthereumBackingConfig {
			token_redeem_address: array_bytes::hex_into_unchecked(TOKEN_REDEEM_ADDRESS),
			deposit_redeem_address: array_bytes::hex_into_unchecked(DEPOSIT_REDEEM_ADDRESS),
			set_authorities_address: array_bytes::hex_into_unchecked(SET_AUTHORITIES_ADDRESS),
			ring_token_address: array_bytes::hex_into_unchecked(RING_TOKEN_ADDRESS),
			kton_token_address: array_bytes::hex_into_unchecked(KTON_TOKEN_ADDRESS),
			backed_ring: BUNCH_OF_COINS,
			backed_kton: BUNCH_OF_COINS,
		},
		darwinia_ethereum_issuing: pangolin_runtime::EthereumIssuingConfig {
			mapping_factory_address: array_bytes::hex_into_unchecked(MAPPING_FACTORY_ADDRESS),
			ethereum_backing_address: array_bytes::hex_into_unchecked(ETHEREUM_BACKING_ADDRESS),
		},
		darwinia_relay_authorities_Instance1: pangolin_runtime::EthereumRelayAuthoritiesConfig {
			authorities: vec![(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				array_bytes::hex_into_unchecked(ETHEREUM_RELAY_AUTHORITY_SIGNER),
				1
			)]
		},
		darwinia_tron_backing: pangolin_runtime::TronBackingConfig {
			backed_ring: BUNCH_OF_COINS,
			backed_kton: BUNCH_OF_COINS,
		},
		darwinia_evm: pangolin_runtime::EVMConfig { accounts: evm_accounts },
		dvm_ethereum: Default::default(),
		darwinia_bridge_bsc: pangolin_runtime::BSCConfig {
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
