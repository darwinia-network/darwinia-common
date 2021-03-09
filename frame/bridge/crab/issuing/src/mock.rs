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

pub mod crab_issuing {
	// --- darwinia ---
	pub use crate::Event;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use sp_io::TestExternalities;
use sp_runtime::{
	testing::{Header, H256},
	traits::{BlakeTwo256, IdentityLookup},
	RuntimeDebug,
};
// --- darwinia ---
use crate::*;

pub type AccountId = u64;
pub type Balance = u128;

pub type System = frame_system::Module<Test>;
pub type CrabIssuing = Module<Test>;

pub type CrabIssuingError = Error<Test>;

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_event! {
	pub enum Event for Test {
		frame_system <T>,
		darwinia_balances Instance0<T>,
		crab_issuing <T>,
	}
}

darwinia_support::impl_test_account_data! { deprecated }

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const CrabIssuingModuleId: ModuleId = ModuleId(*b"da/crabi");
}
impl Config for Test {
	type Event = Event;
	type ModuleId = CrabIssuingModuleId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = ();
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
}

parameter_types! {
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

pub fn new_test_ext() -> TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	RingConfig {
		balances: (1..10)
			.map(|i: AccountId| vec![(i, 100 * i as Balance), (10 * i, 1000 * i as Balance)])
			.flatten()
			.collect(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	GenesisConfig {
		total_mapped_ring: 4_000,
	}
	.assimilate_storage::<Test>(&mut t)
	.unwrap();

	t.into()
}

pub fn events() -> Vec<Event> {
	let events = System::events()
		.into_iter()
		.map(|evt| evt.event)
		.collect::<Vec<_>>();

	System::reset_events();

	events
}

pub fn crab_issuing_events() -> Vec<Event> {
	events()
		.into_iter()
		.filter(|e| matches!(e, Event::crab_issuing(_)))
		.collect()
}
