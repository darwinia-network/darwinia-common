// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! # Mock file for relay authorities

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_io::{hashing, TestExternalities};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill, RuntimeDebug,
};
// --- darwinia ---
use crate::*;
use darwinia_relay_primitives::relay_authorities::Sign as SignT;

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Index = u64;
pub type Balance = u128;

pub type System = frame_system::Module<Test>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;
pub type RelayAuthorities = Module<Test>;

pub type RelayAuthoritiesError = Error<Test, DefaultInstance>;

impl_outer_origin! {
	pub enum Origin for Test {}
}

darwinia_support::impl_test_account_data! {}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
pub struct DarwiniaMMR;
impl MMR<BlockNumber, H256> for DarwiniaMMR {
	fn get_root(_: BlockNumber) -> Option<H256> {
		unimplemented!()
	}
}
pub struct Sign;
impl SignT<BlockNumber> for Sign {
	type Signature = [u8; 65];
	type Message = [u8; 32];
	type Signer = [u8; 20];

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message {
		hashing::blake2_256(raw_message.as_ref())
	}

	fn verify_signature(_: &Self::Signature, _: &Self::Message, _: &Self::Signer) -> bool {
		unimplemented!()
	}
}
parameter_types! {
	pub const LockId: LockIdentifier = *b"lockidts";
	pub const TermDuration: BlockNumber = 10;
	pub const MaxCandidates: usize = 7;
	pub const SignThreshold: Perbill = Perbill::from_percent(60);
	pub const SubmitDuration: BlockNumber = 3;
}
impl Trait for Test {
	type Event = ();
	type RingCurrency = Ring;
	type LockId = LockId;
	type TermDuration = TermDuration;
	type MaxCandidates = MaxCandidates;
	type AddOrigin = EnsureRoot<Self::AccountId>;
	type RemoveOrigin = EnsureRoot<Self::AccountId>;
	type ResetOrigin = EnsureRoot<Self::AccountId>;
	type DarwiniaMMR = DarwiniaMMR;
	type Sign = Sign;
	type SignThreshold = SignThreshold;
	type SubmitDuration = SubmitDuration;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const MaxLocks: u32 = 1024;
}
impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = ();
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestExternalities {
	let mut storage = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		balances: (1..10)
			.map(|i: AccountId| vec![(i, 100 * i as Balance), (10 * i, 1000 * i as Balance)])
			.flatten()
			.collect(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	storage.into()
}

pub fn request_authority(account_id: AccountId) -> DispatchResult {
	RelayAuthorities::request_authority(Origin::signed(account_id), 1, [0; 20])
}

pub fn request_authority_with_stake(account_id: AccountId, stake: Balance) -> DispatchResult {
	RelayAuthorities::request_authority(Origin::signed(account_id), stake, [0; 20])
}
