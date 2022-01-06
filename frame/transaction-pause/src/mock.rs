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

//! Mocks for the transaction pause module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types, traits::Everything};
use frame_system::EnsureSignedBy;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

pub type AccountId = u128;
pub type Balance = u128;
pub const ALICE: AccountId = 1;

mod transaction_pause {
	pub use super::super::*;
}

darwinia_support::impl_test_account_data! {}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const NativeTokenExistentialDeposit: Balance = 10;
	pub const MaxReserves: u32 = 50;
}

impl darwinia_balances::Config<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = NativeTokenExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type BalanceInfo = AccountData<Balance>;
	type OtherCurrencies = ();
	type WeightInfo = ();
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

impl Config for Test {
	type Event = Event;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		TransactionPause: transaction_pause::{Pallet, Storage, Call, Event<T>},
		Balances: darwinia_balances::<Instance1>::{Pallet, Storage, Call, Event<T>},
	}
);

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		t.into()
	}
}
