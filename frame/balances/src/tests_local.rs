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

//! Test utilities

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	assert_err, assert_noop, assert_ok, assert_storage_noop, parameter_types,
	traits::{
		BalanceStatus, Currency, Everything, ExistenceRequirement, GenesisBuild, Imbalance,
		LockIdentifier, NamedReservableCurrency, ReservableCurrency, StorageMapShim,
		WithdrawReasons,
	},
	weights::{DispatchInfo, IdentityFee, Weight},
};
use frame_system::{mocking::*, RawOrigin};
use pallet_transaction_payment::{ChargeTransactionPayment, CurrencyAdapter, Multiplier};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BadOrigin, BlakeTwo256, IdentityLookup, SignedExtension, Zero},
	ArithmeticError, FixedPointNumber, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{self as darwinia_balances, pallet::*};
use darwinia_support::balance::*;

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

type Balance = u64;

darwinia_support::impl_test_account_data! {}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = Balance;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = Balance;
	type BlockWeights = BlockWeights;
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = Balance;
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

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}
impl pallet_transaction_payment::Config for Test {
	type FeeMultiplierUpdate = ();
	type OnChargeTransaction = CurrencyAdapter<Ring, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<u64>;
}

parameter_types! {
	pub static ExistentialDeposit: u64 = 0;
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
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type OtherCurrencies = (Kton,);
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl Config<KtonInstance> for Test {
	type AccountStore = StorageMapShim<
		Account<Test, KtonInstance>,
		frame_system::Provider<Test>,
		Balance,
		AccountData<Balance>,
	>;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type OtherCurrencies = (Ring,);
	type ReserveIdentifier = [u8; 8];
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
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
	}
}

pub struct ExtBuilder {
	existential_deposit: Balance,
	monied: bool,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self { existential_deposit: 1, monied: false }
	}
}
impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: Balance) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}

	pub fn monied(mut self, monied: bool) -> Self {
		self.monied = monied;
		if self.existential_deposit == 0 {
			self.existential_deposit = 1;
		}
		self
	}

	pub fn set_associated_constants(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_constants();
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		darwinia_balances::GenesisConfig::<Test, RingInstance> {
			balances: if self.monied {
				vec![
					(1, 10 * self.existential_deposit),
					(2, 20 * self.existential_deposit),
					(3, 30 * self.existential_deposit),
					(4, 40 * self.existential_deposit),
					(12, 10 * self.existential_deposit),
				]
			} else {
				vec![]
			},
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

decl_tests! { Test, ExtBuilder, EXISTENTIAL_DEPOSIT }

#[test]
fn emit_events_with_no_existential_deposit_suicide_with_dust() {
	<ExtBuilder>::default().existential_deposit(2).build().execute_with(|| {
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::NewAccount(1)),
				Event::Ring(darwinia_balances::Event::Endowed(1, 100)),
				Event::Ring(darwinia_balances::Event::BalanceSet(1, 100, 0)),
			]
		);

		let res = Ring::slash(&1, 98);
		assert_eq!(res, (NegativeImbalance::new(98), 0));

		// no events
		assert_eq!(events(), []);

		let res = Ring::slash(&1, 1);
		assert_eq!(res, (NegativeImbalance::new(1), 0));

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::KilledAccount(1)),
				Event::Ring(darwinia_balances::Event::DustLost(1, 1))
			]
		);
	});
}

#[test]
fn dust_collector_should_work() {
	<ExtBuilder>::default().existential_deposit(100).build().execute_with(|| {
		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::NewAccount(1)),
				Event::Ring(darwinia_balances::Event::Endowed(1, 100)),
				Event::Ring(darwinia_balances::Event::BalanceSet(1, 100, 0)),
			]
		);

		let _ = Ring::slash(&1, 1);

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::KilledAccount(1)),
				Event::Ring(darwinia_balances::Event::DustLost(1, 99))
			]
		);

		// ---

		assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));
		assert_ok!(Kton::set_balance(RawOrigin::Root.into(), 1, 100, 0));

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::NewAccount(1)),
				Event::Ring(darwinia_balances::Event::Endowed(1, 100)),
				Event::Ring(darwinia_balances::Event::BalanceSet(1, 100, 0)),
				Event::Kton(darwinia_balances::Event::Endowed(1, 100)),
				Event::Kton(darwinia_balances::Event::BalanceSet(1, 100, 0)),
			]
		);

		let _ = Ring::slash(&1, 1);

		assert_eq!(events(), []);

		let _ = Kton::slash(&1, 1);

		assert_eq!(
			events(),
			[
				Event::System(frame_system::Event::KilledAccount(1)),
				Event::Ring(darwinia_balances::Event::DustLost(1, 99)),
				Event::Kton(darwinia_balances::Event::DustLost(1, 99)),
			]
		);
	});
}
