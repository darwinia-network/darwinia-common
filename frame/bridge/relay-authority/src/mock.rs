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

//! # Mock file for relay authorities

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::traits::{Everything, GenesisBuild, OnFinalize, OnInitialize};
use frame_system::{mocking::*, EnsureRoot};
use sp_core::H256;
use sp_io::{hashing, TestExternalities};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	RuntimeDebug,
};
// --- darwinia-network ---
use crate::{self as darwinia_relay_authority, *};
use darwinia_relay_primitives::relay_authorities::Sign as SignT;

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

pub(super) type BlockNumber = u64;
pub(super) type AccountId = u64;
pub(super) type Balance = u128;

pub(super) type RelayAuthoritiesError = Error<Test>;

type Hash = H256;
type Index = u64;

pub(super) const DEFAULT_SIGNATURE: [u8; 65] = [0; 65];

darwinia_support::impl_test_account_data! {}

impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = Index;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

frame_support::parameter_types! {
	 const MaxLocks: u32 = 1024;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl darwinia_header_mmr::Config for Test {
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"";
}

pub struct Sign;
impl SignT<BlockNumber> for Sign {
	type Message = [u8; 32];
	type Signature = [u8; 65];
	type Signer = [u8; 20];

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message {
		hashing::blake2_256(raw_message.as_ref())
	}

	fn verify_signature(_: &Self::Signature, _: &Self::Message, _: &Self::Signer) -> bool {
		true
	}
}
impl Clone for MaxMembers {
	fn clone(&self) -> Self {
		Self {}
	}
}
impl core::fmt::Debug for MaxMembers {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("MaxMembers").finish()
	}
}
impl PartialEq for MaxMembers {
	fn eq(&self, _: &Self) -> bool {
		true
	}
}
frame_support::parameter_types! {
	  pub const LockId: LockIdentifier = *b"lockidts";
	  pub const TermDuration: BlockNumber = 10;
	  pub const MaxMembers: u32 = 7;
	  pub const MaxSchedules: u32 = 10;
	  pub const SignThreshold: Perbill = Perbill::from_percent(60);
	  pub const SubmitDuration: BlockNumber = 3;
}
impl Config for Test {
	type AddOrigin = EnsureRoot<Self::AccountId>;
	type Currency = Ring;
	type Event = Event;
	type LockId = LockId;
	type MaxMembers = MaxMembers;
	type MaxSchedules = MaxSchedules;
	type Mmr = HeaderMmr;
	type OpCodes = ();
	type RemoveOrigin = EnsureRoot<Self::AccountId>;
	type ResetOrigin = EnsureRoot<Self::AccountId>;
	type Sign = Sign;
	type SignThreshold = SignThreshold;
	type SubmitDuration = SubmitDuration;
	type TermDuration = TermDuration;
	// type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		HeaderMmr: darwinia_header_mmr::{Pallet, Storage},
		RelayAuthorities: darwinia_relay_authority{Pallet, Call, Storage, Config<T>, Event<T>}
	}
}

pub fn new_test_ext() -> TestExternalities {
	sp_tracing::try_init_simple();

	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		balances: (1..10)
			.map(|i: AccountId| vec![(i, 100 * i as Balance), (10 * i, 1000 * i as Balance)])
			.flatten()
			.collect(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	darwinia_relay_authorityGenesisConfig::<Test> { authorities: vec![(9, signer_of(9), 1)] }
		.assimilate_storage(&mut storage)
		.unwrap();

	storage.into()
}

pub(super) fn run_to_block(n: BlockNumber) {
	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		<RelayAuthorities as OnInitialize<_>>::on_initialize(b);
	}
}

pub(super) fn run_to_block_from_genesis(n: BlockNumber) -> Vec<Header> {
	let mut headers = vec![<frame_system::Pallet<Test>>::finalize()];

	for block_number in 1..=n {
		System::set_block_number(block_number);

		<frame_system::Pallet<Test>>::initialize(
			&block_number,
			&headers[headers.len() - 1].hash(),
			&Default::default(),
			Default::default(),
		);
		<RelayAuthorities as OnInitialize<_>>::on_initialize(block_number);
		<HeaderMmr as OnFinalize<_>>::on_finalize(block_number);

		headers.push(<frame_system::Pallet<Test>>::finalize());
	}

	headers
}

pub(super) fn events() -> Vec<Event> {
	let events = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	events
}

pub(super) fn relay_authorities_events() -> Vec<Event> {
	events().into_iter().filter(|e| matches!(e, Event::RelayAuthorities(_))).collect()
}

pub(super) fn request_authority(account_id: AccountId) -> DispatchResult {
	RelayAuthorities::request_authority(Origin::signed(account_id), 1, signer_of(account_id))
}

pub(super) fn request_authority_with_stake(
	account_id: AccountId,
	stake: Balance,
) -> DispatchResult {
	RelayAuthorities::request_authority(Origin::signed(account_id), stake, signer_of(account_id))
}

pub(super) fn signer_of(account_id: AccountId) -> [u8; 20] {
	[account_id as _; 20]
}
