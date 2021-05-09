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

//! Mock file for treasury.

mod treasury {
	// --- darwinia ---
	// Re-export needed for `impl_outer_event!`.
	pub use super::super::*;
}

// --- substrate ---
use frame_support::traits::GenesisBuild;
use frame_system::mocking::*;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	ModuleId,
};
// --- darwinia ---
use crate::{self as darwinia_treasury, *};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

type Balance = u64;

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
	type AccountId = u128;
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

pub struct Tippers;
impl Contains<u128> for Tippers {
	fn sorted_members() -> Vec<u128> {
		TEN_TO_FOURTEEN.with(|v| v.borrow().clone())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(new: &u128) {
		TEN_TO_FOURTEEN.with(|v| {
			let mut members = v.borrow_mut();
			members.push(*new);
			members.sort();
		})
	}
}
impl ContainsLengthBound for Tippers {
	fn min_len() -> usize {
		0
	}
	fn max_len() -> usize {
		TEN_TO_FOURTEEN.with(|v| v.borrow().len())
	}
}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub static TenToFourteen: Vec<u128> = vec![10, 11, 12, 13, 14];
}
impl darwinia_balances::Config<KtonInstance> for Test {
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
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"da/trsry");
	pub const TipCountdown: u64 = 1;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1;
	pub const DataDepositPerByte: Balance = 1;
	pub const BountyDepositBase: u64 = 80;
	pub const BountyDepositPayoutDelay: u64 = 3;
	pub const BountyUpdatePeriod: u32 = 20;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: u64 = 1;
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 1;
	pub const KtonProposalBondMinimum: Balance = 1;
	pub const SpendPeriod: u64 = 2;
	pub const Burn: Permill = Permill::from_percent(50);
}
impl Config for Test {
	type ModuleId = TreasuryModuleId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin = frame_system::EnsureRoot<u128>;
	type RejectOrigin = frame_system::EnsureRoot<u128>;
	type Tippers = Tippers;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type OnSlashRing = ();
	type OnSlashKton = ();
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyValueMinimum = BountyValueMinimum;
	type MaximumReasonLength = MaximumReasonLength;
	type RingBurnDestination = (); // Just gets burned.
	type KtonBurnDestination = (); // Just gets burned.
	type WeightInfo = ();
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
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Treasury: darwinia_treasury::{Pallet, Call, Storage, Config, Event<T>},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(0, 100), (1, 98), (2, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	darwinia_balances::GenesisConfig::<Test, KtonInstance> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(0, 100), (1, 98), (2, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	darwinia_treasury::GenesisConfig::default()
		.assimilate_storage::<Test, _>(&mut t)
		.unwrap();

	t.into()
}

pub fn last_event() -> RawEvent<u128, H256, u64, u64, DefaultInstance> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let Event::darwinia_treasury(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.last()
		.unwrap()
}

pub fn tip_hash() -> H256 {
	BlakeTwo256::hash_of(&(BlakeTwo256::hash(b"awesome.darwinia"), 3u128))
}
