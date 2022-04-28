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

macro_rules! impl_authority_keys {
	() => {
		pub struct AuthorityKeys {
			stash_key: AccountId,
			session_keys: SessionKeys,
		}
		impl AuthorityKeys {
			pub fn new(sr25519: &str, ed25519: &str, ecdsa: &str) -> Self {
				let sr25519 = array_bytes::hex2array_unchecked(sr25519);
				let ed25519 = array_bytes::hex2array_unchecked(ed25519);
				let ecdsa = array_bytes::hex2array_unchecked(ecdsa);

				Self {
					stash_key: sr25519.into(),
					session_keys: session_keys(
						sr25519.unchecked_into(),
						ed25519.unchecked_into(),
						ecdsa.unchecked_into(),
						sr25519.unchecked_into(),
						sr25519.unchecked_into(),
					),
				}
			}

			pub fn testnet_authorities() -> Vec<Self> {
				vec![
					AuthorityKeys::new(
						"0x9c43c00407c0a51e0d88ede9d531f165e370013b648e6b62f4b3bcff4689df02",
						"0x63e122d962a835020bef656ad5a80dbcc994bb48a659f1af955552f4b3c27b09",
						"0x021842ca1a9aff1549b811126a9c1171d18ffddbd4434478675606f840cfc2fd09",
					),
					AuthorityKeys::new(
						"0x741a9f507722713ec0a5df1558ac375f62469b61d1f60fa60f5dedfc85425b2e",
						"0x8a50704f41448fca63f608575debb626639ac00ad151a1db08af1368be9ccb1d",
						"0x0312aed9a712318917535314973525902d09d298cb04520f7c9ed9959fe69678f3",
					),
					AuthorityKeys::new(
						"0x2276a3162f1b63c21b3396c5846d43874c5b8ba69917d756142d460b2d70d036",
						"0xb28fade2d023f08c0d5a131eac7d64a107a2660f22a0aca09b37a3f321259ef6",
						"0x0374704a3dd21e01cff4d47e53f84fa4a91a0971ebde83007ef6b567b183344558",
					),
					AuthorityKeys::new(
						"0x7a8b265c416eab5fdf8e5a1b3c7635131ca7164fbe6f66d8a70feeeba7c4dd7f",
						"0x305bafd512366e7fd535fdc144c7034b8683e1814d229c84a116f3cb27a97643",
						"0x034a972603f389797cad3705eacb90584c55aa004cf973206811b1e53e56a02f5a",
					),
				]
			}
		}
	};
}

pub mod pangolin;
pub use pangolin::ChainSpec as PangolinChainSpec;

pub mod pangoro;
pub use pangoro::ChainSpec as PangoroChainSpec;

#[cfg(feature = "template")]
pub mod template;
#[cfg(feature = "template")]
pub use template::ChainSpec as TemplateChainSpec;

pub use beefy_primitives::crypto::AuthorityId as BeefyId;
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use sc_finality_grandpa::AuthorityId as GrandpaId;
pub use sp_consensus_babe::AuthorityId as BabeId;

// --- crates.io ---
use serde::{Deserialize, Serialize};
// --- paritytech ---
use sc_chain_spec::ChainSpecExtension;
use sc_client_api::{BadBlocks, ForkBlocks};
use sc_sync_state_rpc::LightSyncStateExtension;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::IdentifyAccount;
// --- darwinia-network ---
use drml_primitives::{AccountId, AccountPublic, OpaqueBlock};

const DEFAULT_PROTOCOL_ID: &str = "drml";

const TEAM_MEMBERS: &[&str] = &[
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

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: ForkBlocks<OpaqueBlock>,
	/// Known bad block hashes.
	pub bad_blocks: BadBlocks<OpaqueBlock>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: LightSyncStateExtension,
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, BabeId, GrandpaId, BeefyId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BeefyId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}
