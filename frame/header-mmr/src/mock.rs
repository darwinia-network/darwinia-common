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

//! Test utilities

// --- crates ---
use codec::Encode;
// --- paritytech ---
use frame_system::mocking::*;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DigestItem,
};
// --- darwinia ---
use crate::{self as darwinia_header_mmr, *};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

impl Config for Test {
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"mmr";
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config},
		HeaderMMR: darwinia_header_mmr::{Pallet, Call, Storage},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

pub fn header_mmr_log(hash: H256) -> DigestItem<H256> {
	let mmr_root_log = MerkleMountainRangeRootLog::<H256> {
		prefix: LOG_PREFIX,
		parent_mmr_root: hash,
	};

	DigestItem::Other(mmr_root_log.encode())
}

pub fn initialize_block(number: u64, parent_hash: H256) {
	System::initialize(
		&number,
		&parent_hash,
		&Default::default(),
		Default::default(),
	);
}
