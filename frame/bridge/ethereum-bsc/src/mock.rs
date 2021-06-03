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

// --- substrate ---
use frame_system::mocking::*;
use sp_core::U256;
// --- darwinia ---
use crate::{self as darwinia_bridge_ethereum_bsc, *};
use bsc_primitives::BSCHeader;

pub type Block = MockBlock<Test>;
pub type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

pub type AccountId = u64;
pub type BlockNumber = u64;

/// Gas limit valid in test environment.
pub const GAS_LIMIT: u64 = 0x2000;

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = sp_core::H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Header = sp_runtime::testing::Header;
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

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub TestBSCConfiguration: BSCConfiguration = BSCConfiguration {
		chain_id: 56,
		min_gas_limit: 0x1388.into(),
		max_gas_limit: U256::max_value(),
		period: 0x03,
		epoch_length: 0xc8, // 200
	};
}
impl Config for Test {
	type BSCConfiguration = TestBSCConfiguration;
	type UnixTime = Timestamp;
	type OnHeadersSubmitted = ();
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		BSC: darwinia_bridge_ethereum_bsc::{Pallet, Storage, Call},
	}
}

pub struct ExtBuilder {
	genesis_header: BSCHeader,
	total_validators: usize,
}
impl ExtBuilder {
	pub fn genesis_header(mut self, header: BSCHeader) -> Self {
		self.genesis_header = header;

		self
	}

	pub fn total_validators(mut self, count: usize) -> Self {
		self.total_validators = count;

		self
	}
}
// impl Default for ExtBuilder {
// 	fn default() -> Self {
// 		Self {
// 			genesis_header: ,
// 		}
// 	}
// }

