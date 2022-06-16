// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test setup for potential reentracy and lost updates of nested mutations.

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	assert_ok,
	traits::{
		BalanceStatus, Currency, Everything, GenesisBuild, OnUnbalanced, ReservableCurrency,
		StorageMapShim,
	},
	weights::IdentityFee,
};
use frame_system::{mocking::*, RawOrigin};
use pallet_transaction_payment::CurrencyAdapter;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, RuntimeDebug};
// --- darwinia-network ---
use crate::{self as darwinia_balances, pallet::*};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

type Balance = u64;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
	pub static ExistentialDeposit: Balance = 0;
}
impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = u64;
	type BaseCallFilter = Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = BlockWeights;
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = u64;
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
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}
impl pallet_transaction_payment::Config for Test {
	type FeeMultiplierUpdate = ();
	type OnChargeTransaction = CurrencyAdapter<Ring, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
}

pub struct OnDustRemoval;
impl OnUnbalanced<NegativeImbalance<Test, RingInstance>> for OnDustRemoval {
	fn on_nonzero_unbalanced(amount: NegativeImbalance<Test, RingInstance>) {
		assert_ok!(Ring::resolve_into_existing(&1, amount));
	}
}
frame_support::parameter_types! {
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 2;
}
impl Config<RingInstance> for Test {
	type AccountStore = StorageMapShim<
		Account<Test, RingInstance>,
		frame_system::Provider<Test>,
		Balance,
		AccountData<Balance>,
	>;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = OnDustRemoval;
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

pub struct ExtBuilder {
	existential_deposit: Balance,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self { existential_deposit: 1 }
	}
}
impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: Balance) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}

	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_consts();
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		darwinia_balances::GenesisConfig::<Test, RingInstance> { balances: vec![] }
			.assimilate_storage(&mut t)
			.unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

#[test]
fn transfer_dust_removal_tst1_should_work() {
	ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
		// Verification of reentrancy in dust removal
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 1000, 0));
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 2, 500, 0));

		// In this transaction, account 2 free balance
		// drops below existential balance
		// and dust balance is removed from account 2
		assert_ok!(Ring::transfer(RawOrigin::Signed(2).into(), 3, 450));

		// As expected dust balance is removed.
		assert_eq!(Ring::free_balance(&2), 0);

		// As expected beneficiary account 3
		// received the transfered fund.
		assert_eq!(Ring::free_balance(&3), 450);

		// Dust balance is deposited to account 1
		// during the process of dust removal.
		assert_eq!(Ring::free_balance(&1), 1050);

		// Verify the events
		assert_eq!(System::events().len(), 12);

		System::assert_has_event(Event::Ring(crate::Event::Transfer(2, 3, 450)));
		System::assert_has_event(Event::Ring(crate::Event::DustLost(2, 50)));
		System::assert_has_event(Event::Ring(crate::Event::Deposit(1, 50)));
	});
}

#[test]
fn transfer_dust_removal_tst2_should_work() {
	ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
		// Verification of reentrancy in dust removal
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 1000, 0));
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 2, 500, 0));

		// In this transaction, account 2 free balance
		// drops below existential balance
		// and dust balance is removed from account 2
		assert_ok!(Ring::transfer(RawOrigin::Signed(2).into(), 1, 450));

		// As expected dust balance is removed.
		assert_eq!(Ring::free_balance(&2), 0);

		// Dust balance is deposited to account 1
		// during the process of dust removal.
		assert_eq!(Ring::free_balance(&1), 1500);

		// Verify the events
		assert_eq!(System::events().len(), 10);

		System::assert_has_event(Event::Ring(crate::Event::Transfer(2, 1, 450)));
		System::assert_has_event(Event::Ring(crate::Event::DustLost(2, 50)));
		System::assert_has_event(Event::Ring(crate::Event::Deposit(1, 50)));
	});
}

#[test]
fn repatriating_reserved_balance_dust_removal_should_work() {
	ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
		// Verification of reentrancy in dust removal
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 1000, 0));
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 2, 500, 0));

		// Reserve a value on account 2,
		// Such that free balance is lower than
		// Exestintial deposit.
		assert_ok!(Ring::reserve(&2, 450));

		// Transfer of reserved fund from slashed account 2 to
		// beneficiary account 1
		assert_ok!(Ring::repatriate_reserved(&2, &1, 450, BalanceStatus::Free), 0);

		// Since free balance of account 2 is lower than
		// existential deposit, dust amount is
		// removed from the account 2
		assert_eq!(Ring::reserved_balance(2), 0);
		assert_eq!(Ring::free_balance(2), 0);

		// account 1 is credited with reserved amount
		// together with dust balance during dust
		// removal.
		assert_eq!(Ring::reserved_balance(1), 0);
		assert_eq!(Ring::free_balance(1), 1500);

		// Verify the events
		assert_eq!(System::events().len(), 11);

		System::assert_has_event(Event::Ring(crate::Event::ReserveRepatriated(
			2,
			1,
			450,
			BalanceStatus::Free,
		)));
		System::assert_has_event(Event::Ring(crate::Event::DustLost(2, 50)));
		System::assert_last_event(Event::Ring(crate::Event::Deposit(1, 50)));
	});
}
