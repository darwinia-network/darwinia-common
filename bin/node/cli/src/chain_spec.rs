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
use std::collections::BTreeMap;
// --- substrate ---
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public, H160};
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
use pangolin_runtime::{constants::COIN, BalancesConfig as RingConfig, *};

pub type PangolinChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

type AccountPublic = <Signature as Verify>::Signer;

const PANGOLIN_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub fn pangolin_config() -> Result<PangolinChainSpec, String> {
	PangolinChainSpec::from_json_bytes(&include_bytes!("../../../res/pangolin/pangolin.json")[..])
}

pub fn pangolin_session_keys(
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

pub fn properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("ss58Format".into(), 18.into());
	properties.insert("tokenDecimals".into(), vec![9, 9].into());
	properties.insert("tokenSymbol".into(), vec!["PRING", "PKTON"].into());

	properties
}

pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn get_authority_keys_from_seed(
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

fn pangolin_build_spec_genesis() -> GenesisConfig {
	const ROOT: &'static str = "0x72819fbc1b93196fa230243947c1726cbea7e33044c7eb6f736ff345561f9e4c";
	const GENESIS_VALIDATOR: &'static str = "Alice";
	const GENESIS_VALIDATOR_STASH: &'static str = "Alice//stash";
	const GENESIS_VALIDATOR_BOND: Balance = COIN;
	const GENESIS_EVM_ACCOUNT: &'static str = "0x68898db1012808808c903f390909c52d9f706749";
	const GENESIS_ETHEREUM_RELAY_AUTHORITY_SIGNER: &'static str =
		"0x6aA70f55E5D770898Dd45aa1b7078b8A80AAbD6C";

	const TOKEN_REDEEM_ADDRESS: &'static str = "0x49262B932E439271d05634c32978294C7Ea15d0C";
	const DEPOSIT_REDEEM_ADDRESS: &'static str = "0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e";
	const SET_AUTHORITIES_ADDRESS: &'static str = "0xE4A2892599Ad9527D76Ce6E26F93620FA7396D85";
	const RING_TOKEN_ADDRESS: &'static str = "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0";
	const KTON_TOKEN_ADDRESS: &'static str = "0x1994100c58753793D52c6f457f189aa3ce9cEe94";

	let root = AccountId::from(array_bytes::hex2array_unchecked!(ROOT, 32));
	let evm = array_bytes::hex2array_unchecked!(GENESIS_EVM_ACCOUNT, 20).into();
	let initial_authorities = vec![get_authority_keys_from_seed(GENESIS_VALIDATOR)];
	let endowed_accounts = vec![
		(root.clone(), 1 << 56),
		(
			get_account_id_from_seed::<sr25519::Public>(GENESIS_VALIDATOR_STASH),
			GENESIS_VALIDATOR_BOND,
		),
	];
	let mut evm_accounts = BTreeMap::new();

	evm_accounts.insert(
		evm,
		GenesisAccount {
			nonce: 0.into(),
			balance: 20_000_000_000_000_000_000_000_000u128.into(),
			storage: BTreeMap::new(),
			code: vec![],
		},
	);

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_babe: Default::default(),
		darwinia_balances_Instance0: RingConfig { balances: endowed_accounts },
		darwinia_balances_Instance1: Default::default(),
		darwinia_staking: StakingConfig {
			minimum_validator_count: 1,
			validator_count: 7,
			stakers: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0, x.1, GENESIS_VALIDATOR_BOND, StakerStatus::Validator))
				.collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		},
		pallet_session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0.clone(), x.0, pangolin_session_keys(x.2, x.3, x.4, x.5)))
				.collect(),
		},
		pallet_grandpa: Default::default(),
		pallet_im_online: Default::default(),
		pallet_authority_discovery: Default::default(),
		darwinia_democracy: Default::default(),
		pallet_collective_Instance0: Default::default(),
		pallet_collective_Instance1: Default::default(),
		darwinia_elections_phragmen: Default::default(),
		pallet_membership_Instance0: Default::default(),
		darwinia_claims: Default::default(),
		darwinia_vesting: Default::default(),
		pallet_sudo: SudoConfig { key: root.clone() },
		darwinia_crab_issuing: CrabIssuingConfig {
			total_mapped_ring: 1 << 56
		},
		darwinia_crab_backing: CrabBackingConfig {
			backed_ring: 1 << 56
		},
		darwinia_ethereum_backing: EthereumBackingConfig {
			token_redeem_address: array_bytes::hex2array_unchecked!(TOKEN_REDEEM_ADDRESS, 20).into(),
			deposit_redeem_address: array_bytes::hex2array_unchecked!(DEPOSIT_REDEEM_ADDRESS, 20).into(),
			set_authorities_address: array_bytes::hex2array_unchecked!(SET_AUTHORITIES_ADDRESS, 20).into(),
			ring_token_address: array_bytes::hex2array_unchecked!(RING_TOKEN_ADDRESS, 20).into(),
			kton_token_address: array_bytes::hex2array_unchecked!(KTON_TOKEN_ADDRESS, 20).into(),
			ring_locked: 1 << 56,
			kton_locked: 1 << 56,
		},
		darwinia_ethereum_relay: EthereumRelayConfig {
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
		darwinia_tron_backing: TronBackingConfig {
			backed_ring: 1 << 56,
			backed_kton: 1 << 56,
		},
		darwinia_evm: EVMConfig {
			accounts: evm_accounts,
		},
		dvm_ethereum: Default::default(),
		darwinia_relay_authorities_Instance0: EthereumRelayAuthoritiesConfig {
			authorities: vec![(root, array_bytes::hex2array_unchecked!(GENESIS_ETHEREUM_RELAY_AUTHORITY_SIGNER, 20).into(), 1)]
		},
	}
}

pub fn pangolin_development_config() -> PangolinChainSpec {
	PangolinChainSpec::from_genesis(
		"Development",
		"pangolin_dev",
		ChainType::Development,
		|| {
			let initial_evm_account = vec![
				array_bytes::hex2array_unchecked!("0x6be02d1d3665660d22ff9624b7be0551ee1ac91b", 20)
					.into(),
				array_bytes::hex2array_unchecked!("0xB90168C8CBcd351D069ffFdA7B71cd846924d551", 20)
					.into(),
			];
			let mut evm_accounts = BTreeMap::new();

			for account_id in initial_evm_account.iter() {
				evm_accounts.insert(
					*account_id,
					GenesisAccount {
						nonce: 0.into(),
						balance: 123_456_789_000_000_000_090u128.into(),
						storage: BTreeMap::new(),
						code: vec![],
					},
				);
			}

			pangolin_development_genesis(
				vec![get_authority_keys_from_seed("Alice")],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				evm_accounts,
			)
		},
		vec![],
		None,
		None,
		Some(properties()),
		None,
	)
}

fn pangolin_development_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	mut endowed_accounts: Vec<AccountId>,
	evm_accounts: BTreeMap<H160, GenesisAccount>,
) -> GenesisConfig {
	const GENESIS_ETHEREUM_RELAY_AUTHORITY_SIGNER: &'static str =
		"0x6aA70f55E5D770898Dd45aa1b7078b8A80AAbD6C";

	const TOKEN_REDEEM_ADDRESS: &'static str = "0x49262B932E439271d05634c32978294C7Ea15d0C";
	const DEPOSIT_REDEEM_ADDRESS: &'static str = "0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e";
	const SET_AUTHORITIES_ADDRESS: &'static str = "0xE4A2892599Ad9527D76Ce6E26F93620FA7396D85";
	const RING_TOKEN_ADDRESS: &'static str = "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0";
	const KTON_TOKEN_ADDRESS: &'static str = "0x1994100c58753793D52c6f457f189aa3ce9cEe94";

	initial_authorities.iter().for_each(|x| {
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	});

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_babe: Default::default(),
		darwinia_balances_Instance0: RingConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 56))
				.collect(),
		},
		darwinia_balances_Instance1: KtonConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 56))
				.collect(),
		},
		darwinia_staking: StakingConfig {
			minimum_validator_count: 1,
			validator_count: 2,
			stakers: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0, x.1, 1 << 56, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().cloned().map(|x| x.0).collect(),
			force_era: darwinia_staking::Forcing::ForceAlways,
			slash_reward_fraction: Perbill::from_percent(10),
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		},
		pallet_session: SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0.clone(), x.0, pangolin_session_keys(x.2, x.3, x.4, x.5)))
				.collect(),
		},
		pallet_grandpa: Default::default(),
		pallet_im_online: Default::default(),
		pallet_authority_discovery: Default::default(),
		darwinia_democracy: Default::default(),
		pallet_collective_Instance0: Default::default(),
		pallet_collective_Instance1: Default::default(),
		darwinia_elections_phragmen: Default::default(),
		pallet_membership_Instance0: Default::default(),
		darwinia_claims: ClaimsConfig {
			claims_list: ClaimsList::from_file(
				"bin/res/claims-list.json",
				"CLAIMS_LIST_PATH",
			),
		},
		darwinia_vesting: Default::default(),
		pallet_sudo: SudoConfig { key: root_key.clone() },
		darwinia_crab_issuing: CrabIssuingConfig {
			total_mapped_ring: 1 << 56
		},
		darwinia_crab_backing: CrabBackingConfig {
			backed_ring: 1 << 56
		},
		darwinia_ethereum_backing: EthereumBackingConfig {
			token_redeem_address: array_bytes::hex2array_unchecked!(TOKEN_REDEEM_ADDRESS, 20).into(),
			deposit_redeem_address: array_bytes::hex2array_unchecked!(DEPOSIT_REDEEM_ADDRESS, 20).into(),
			set_authorities_address: array_bytes::hex2array_unchecked!(SET_AUTHORITIES_ADDRESS, 20).into(),
			ring_token_address: array_bytes::hex2array_unchecked!(RING_TOKEN_ADDRESS, 20).into(),
			kton_token_address: array_bytes::hex2array_unchecked!(KTON_TOKEN_ADDRESS, 20).into(),
			ring_locked: 1 << 56,
			kton_locked: 1 << 56,
		},
		darwinia_ethereum_relay: EthereumRelayConfig {
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
		darwinia_tron_backing: TronBackingConfig {
			backed_ring: 1 << 56,
			backed_kton: 1 << 56,
		},
		darwinia_evm: EVMConfig {
			accounts: evm_accounts,
		},
		dvm_ethereum: Default::default(),
		darwinia_relay_authorities_Instance0: EthereumRelayAuthoritiesConfig {
			authorities: vec![(root_key, array_bytes::hex2array_unchecked!(GENESIS_ETHEREUM_RELAY_AUTHORITY_SIGNER, 20).into(), 1)]
		},
	}
}
