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

pub mod pangolin;
pub use pangolin::ChainSpec as PangolinChainSpec;

pub mod pangoro;
pub use pangoro::ChainSpec as PangoroChainSpec;

#[cfg(feature = "template")]
pub mod template;
#[cfg(feature = "template")]
pub use template::ChainSpec as TemplateChainSpec;

// --- paritytech ---
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_finality_grandpa::AuthorityId as GrandpaId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::IdentifyAccount;
// --- darwinia-network ---
use drml_common_primitives::{AccountId, AccountPublic};

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
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}
