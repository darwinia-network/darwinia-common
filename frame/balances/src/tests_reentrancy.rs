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

//! Test setup for potential reentracy and lost updates of nested mutations.

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	assert_ok,
	traits::{
		BalanceStatus, Currency, GenesisBuild, OnUnbalanced, ReservableCurrency, StorageMapShim,
	},
	weights::IdentityFee,
};
use frame_system::{mocking::*, RawOrigin};
use pallet_transaction_payment::CurrencyAdapter;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, RuntimeDebug};
// --- darwinia ---
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
	type BaseCallFilter = ();
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
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
	pub const TransactionByteFee: Balance = 1;
}
impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = CurrencyAdapter<Ring, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

pub struct OnDustRemoval;
impl OnUnbalanced<NegativeImbalance<Test, RingInstance>> for OnDustRemoval {
	fn on_nonzero_unbalanced(amount: NegativeImbalance<Test, RingInstance>) {
		assert_ok!(Ring::resolve_into_existing(&1, amount));
	}
}
frame_support::parameter_types! {
	pub const MaxLocks: u32 = 50;
}
impl Config<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = OnDustRemoval;
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = StorageMapShim<
		Account<Test, RingInstance>,
		frame_system::Provider<Test>,
		Balance,
		AccountData<Balance>,
	>;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = ();
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
		Self {
			existential_deposit: 1,
		}
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
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
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
	ExtBuilder::default()
		.existential_deposit(100)
		.build()
		.execute_with(|| {
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
			// Number of events expected is 8
			assert_eq!(System::events().len(), 11);

			System::assert_has_event(Event::darwinia_balances_Instance1(crate::Event::Transfer(
				2, 3, 450,
			)));
			System::assert_has_event(Event::darwinia_balances_Instance1(crate::Event::DustLost(
				2, 50,
			)));
		});
}

#[test]
fn transfer_dust_removal_tst2_should_work() {
	ExtBuilder::default()
		.existential_deposit(100)
		.build()
		.execute_with(|| {
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
			// Number of events expected is 8
			assert_eq!(System::events().len(), 9);

			System::assert_has_event(Event::darwinia_balances_Instance1(crate::Event::Transfer(
				2, 1, 450,
			)));
			System::assert_has_event(Event::darwinia_balances_Instance1(crate::Event::DustLost(
				2, 50,
			)));
		});
}

#[test]
fn repatriating_reserved_balance_dust_removal_should_work() {
	ExtBuilder::default()
		.existential_deposit(100)
		.build()
		.execute_with(|| {
			// Verification of reentrancy in dust removal
			assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 1000, 0));
			assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 2, 500, 0));

			// Reserve a value on account 2,
			// Such that free balance is lower than
			// Exestintial deposit.
			assert_ok!(Ring::reserve(&2, 450));

			// Transfer of reserved fund from slashed account 2 to
			// beneficiary account 1
			assert_ok!(
				Ring::repatriate_reserved(&2, &1, 450, BalanceStatus::Free),
				0
			);

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
			// Number of events expected is 10
			assert_eq!(System::events().len(), 10);

			System::assert_has_event(Event::darwinia_balances_Instance1(
				crate::Event::ReserveRepatriated(2, 1, 450, BalanceStatus::Free),
			));
			System::assert_last_event(Event::darwinia_balances_Instance1(crate::Event::DustLost(
				2, 50,
			)));
		});
}
