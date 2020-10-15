// --- substrate ---
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::{ChainType, Properties};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public, H160, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
// --- darwinia ---
use array_bytes::fixed_hex_bytes_unchecked;
use darwinia_claims::ClaimsList;
use darwinia_ethereum_relay::DagsMerkleRootsLoader as DagsMerkleRootsLoaderR;
use node_template_runtime::{BalancesConfig as RingConfig, EVMConfig, EthereumConfig, *};

// std
use std::collections::BTreeMap;
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `NodeTemplateChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type NodeTemplateChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

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

/// Properties for Darwinia.
pub fn properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenDecimals".into(), 9.into());
	properties.insert("tokenSymbol".into(), "CRING".into());
	properties.insert("ktonTokenDecimals".into(), 9.into());
	properties.insert("ktonTokenSymbol".into(), "CKTON".into());

	properties
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate an authority key for Babe
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

pub fn node_template_development_config() -> NodeTemplateChainSpec {
	NodeTemplateChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		|| {
			testnet_genesis(
				vec![get_authority_keys_from_seed("Alice")],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
			)
		},
		vec![],
		None,
		None,
		Some(properties()),
		None,
	)
}

pub fn node_template_local_testnet_config() -> NodeTemplateChainSpec {
	NodeTemplateChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		|| {
			testnet_genesis(
				vec![
					get_authority_keys_from_seed("Alice"),
					get_authority_keys_from_seed("Bob"),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
			)
		},
		vec![],
		None,
		None,
		Some(properties()),
		None,
	)
}

fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	let gerald_evm_account_id = H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b").unwrap();
	let mut evm_accounts = BTreeMap::new();
	evm_accounts.insert(
		gerald_evm_account_id,
		evm::GenesisAccount {
			nonce: 0.into(),
			balance: U256::from(123456_123_000_000_000_000_000u128),
			storage: BTreeMap::new(),
			code: vec![],
		},
	);

	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_babe: Some(Default::default()),
		darwinia_balances_Instance0: Some(RingConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 56))
				.collect(),
		}),
		darwinia_balances_Instance1: Some(KtonConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 56))
				.collect(),
		}),
		darwinia_staking: Some(StakingConfig {
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
		}),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities
				.iter()
				.cloned()
				.map(|x| (x.0.clone(), x.0, session_keys(x.2, x.3, x.4, x.5)))
				.collect(),
		}),
		pallet_grandpa: Some(Default::default()),
		pallet_im_online: Some(Default::default()),
		pallet_authority_discovery: Some(Default::default()),
		darwinia_democracy: Some(Default::default()),
		pallet_collective_Instance0: Some(Default::default()),
		pallet_collective_Instance1: Some(Default::default()),
		darwinia_elections_phragmen: Some(Default::default()),
		pallet_membership_Instance0: Some(Default::default()),
		darwinia_claims: Some(ClaimsConfig {
			claims_list: ClaimsList::from_file(
				"bin/node-template/node/res/claims-list.json",
				"CLAIMS_LIST_PATH",
			),
		}),
		darwinia_vesting: Some(Default::default()),
		pallet_sudo: Some(SudoConfig { key: root_key }),
		darwinia_crab_issuing: Some(CrabIssuingConfig {
			total_mapped_ring: 1 << 56
		}),
		darwinia_crab_backing: Some(CrabBackingConfig {
			backed_ring: 1 << 56
		}),
		darwinia_ethereum_backing: Some(EthereumBackingConfig {
			token_redeem_address: fixed_hex_bytes_unchecked!(
				"0x49262B932E439271d05634c32978294C7Ea15d0C",
				20
			)
			.into(),
			deposit_redeem_address: fixed_hex_bytes_unchecked!(
				"0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e",
				20
			)
			.into(),
			ring_token_address: fixed_hex_bytes_unchecked!(
				"0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
				20
			).into(),
			kton_token_address: fixed_hex_bytes_unchecked!(
				"0x1994100c58753793D52c6f457f189aa3ce9cEe94",
				20
			).into(),
			ring_locked: 1 << 56,
			kton_locked: 1 << 56,
		}),
		darwinia_ethereum_relay: Some(EthereumRelayConfig {
			genesis_header_info: (
				vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 128, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 33, 123, 11, 188, 251, 114, 226, 213, 126, 40, 243, 60, 179, 97, 185, 152, 53, 19, 23, 119, 85, 220, 63, 51, 206, 62, 112, 34, 237, 98, 183, 123, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 132, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 136, 0, 0, 0, 0, 0, 0, 0, 66, 1, 65, 148, 16, 35, 104, 9, 35, 224, 254, 77, 116, 163, 75, 218, 200, 20, 31, 37, 64, 227, 174, 144, 98, 55, 24, 228, 125, 102, 209, 202, 74, 45],
				b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".into()
			),
			dags_merkle_roots_loader: DagsMerkleRootsLoaderR::from_file(
				"bin/node-template/node/res/dags-merkle-roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		}),
		darwinia_tron_backing: Some(TronBackingConfig {
			backed_ring: 1 << 56,
			backed_kton: 1 << 56,
		}),
		// frontier
		frame_evm: Some(EVMConfig {
			accounts: evm_accounts,
		}),
		frame_ethereum: Some(EthereumConfig {}),
	}
}
