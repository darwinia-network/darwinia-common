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

//! Test utilities

// --- crates.io ---
use codec::Encode;
// --- github.com ---
use mmr::MMRStore;
// --- paritytech ---
use frame_support::traits::{Everything, OnFinalize, OnInitialize};
use frame_system::mocking::*;
use sp_core::{
	offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
	H256,
};
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DigestItem,
};
// --- darwinia-network ---
use crate::{self as darwinia_header_mmr, primitives::*, *};

pub type BlockNumber = u64;
pub type Hash = H256;

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
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

	const INDEXING_PREFIX: &'static [u8] = b"header-mmr-";
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config},
		HeaderMmr: darwinia_header_mmr::{Pallet, Storage},
	}
}

pub fn new_test_ext() -> TestExternalities {
	sp_tracing::try_init_simple();

	frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

#[allow(unused)]
pub fn register_offchain_ext(ext: &mut TestExternalities) {
	ext.persist_offchain_overlay();

	let (offchain, _) = TestOffchainExt::with_offchain_db(ext.offchain_db());

	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
}

pub fn header_parent_mmr_log(hash: Hash) -> DigestItem<Hash> {
	let mmr_root_log =
		MerkleMountainRangeRootLog::<Hash> { prefix: LOG_PREFIX, parent_mmr_root: hash };

	DigestItem::Other(mmr_root_log.encode())
}

pub fn mmr_with_size<StorageType>(size: NodeIndex) -> Mmr<StorageType, Test>
where
	Storage<StorageType, Test>: MMRStore<Hash>,
{
	<Mmr<StorageType, Test>>::with_size(size)
}

pub fn mmr<StorageType>() -> Mmr<StorageType, Test>
where
	Storage<StorageType, Test>: MMRStore<Hash>,
{
	mmr_with_size(HeaderMmr::mmr_size())
}

pub fn new_block_with_parent_hash(parent_hash: Hash) -> Header {
	let number = <frame_system::Pallet<Test>>::block_number() + 1;

	<frame_system::Pallet<Test>>::initialize(
		&number,
		&parent_hash,
		&Default::default(),
		Default::default(),
	);
	HeaderMmr::on_initialize(number);
	HeaderMmr::on_finalize(number);
	<frame_system::Pallet<Test>>::finalize()
}

pub fn new_block() -> Header {
	let number = <frame_system::Pallet<Test>>::block_number() + 1;
	let parent_hash = Hash::repeat_byte(number as _);

	new_block_with_parent_hash(parent_hash)
}

#[allow(unused)]
pub fn run_to_block_from_genesis(n: BlockNumber) -> Vec<Header> {
	let mut headers = vec![new_block()];

	for _ in 2..=n {
		headers.push(new_block_with_parent_hash(headers[headers.len() - 1].hash()));
	}

	headers
}
