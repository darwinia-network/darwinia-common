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

#![allow(dead_code)]

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::traits::GenesisBuild;
use frame_system::mocking::*;
use sp_io::TestExternalities;
use sp_runtime::ModuleId;
use sp_runtime::{
	testing::{Header, H256},
	traits::{BlakeTwo256, IdentityLookup},
	RuntimeDebug,
};
// --- darwinia ---
use crate::{self as darwinia_crab_issuing, *};

// Global primitives
pub type Block = MockBlock<Test>;
pub type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
pub type AccountId = u64;
pub type Balance = u128;

darwinia_support::impl_test_account_data! {}

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
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

frame_support::parameter_types! {
	pub const ExistentialDeposit: Balance = 0;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const CrabIssuingModuleId: ModuleId = ModuleId(*b"da/crabi");
}
impl Config for Test {
	type WeightInfo = ();
	type ModuleId = CrabIssuingModuleId;
	type RingCurrency = Ring;
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
		CrabIssuing: darwinia_crab_issuing::{Pallet, Call, Storage, Config},
	}
}

pub fn new_test_ext() -> TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		balances: (1..10)
			.map(|i: AccountId| vec![(i, 100 * i as Balance), (10 * i, 1000 * i as Balance)])
			.flatten()
			.collect(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	<darwinia_crab_issuing::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
		&darwinia_crab_issuing::GenesisConfig {
			total_mapped_ring: 4_000,
		},
		&mut t,
	)
	.unwrap();

	t.into()
}
