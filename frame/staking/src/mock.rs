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

//! Test utilities

#![allow(unused)]

// --- std ---
use std::{
	cell::RefCell,
	collections::{BTreeMap, HashSet},
};
// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_election_provider_support::*;
use frame_support::{
	assert_ok, parameter_types,
	storage::IterableStorageMap,
	traits::{
		Currency, Everything, FindAuthor, GenesisBuild, Get, Imbalance, OnFinalize, OnInitialize,
		OnUnbalanced, OneSessionHandler, UnixTime,
	},
	weights::constants::RocksDbWeight,
	PalletId, StorageValue,
};
use frame_system::mocking::*;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{IdentityLookup, Zero},
	Perbill, RuntimeDebug,
};
use sp_staking::{offence::*, *};
// --- darwinia-network ---
use crate::{self as darwinia_staking, *};

pub type AccountId = u64;
pub type AccountIndex = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub type Block = MockBlock<Test>;
pub type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
pub type Extrinsic = TestXt<Call, ()>;

pub type StakingCall = darwinia_staking::Call<Test>;
pub type TestRuntimeCall = <Test as frame_system::Config>::Call;

pub type StakingError = Error<Test>;

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const CAP: Balance = 10_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

pub const INIT_TIMESTAMP: TsInMs = 30_000;
pub const BLOCK_TIME: u64 = 1_000;

darwinia_support::impl_test_account_data! {}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_: bool, validators: I, _: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
		SESSION_VALIDATORS.with(|x| {
			*x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new())
		});
	}

	fn on_disabled(validator_index: usize) {
		SESSION_VALIDATORS.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index];
			d.1.insert(value);
		})
	}
}
impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

pub fn is_disabled(controller: AccountId) -> bool {
	let stash = Staking::ledger(&controller).unwrap().stash;
	SESSION_VALIDATORS.with(|d| d.borrow().1.contains(&stash))
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			frame_support::weights::constants::WEIGHT_PER_SECOND * 2
		);
}
impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = BlockWeights;
	type Call = Call;
	type DbWeight = RocksDbWeight;
	type Event = Event;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = AccountIndex;
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

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}
impl pallet_session::Config for Test {
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type Event = Event;
	type Keys = SessionKeys;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf<Test>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

parameter_types! {
	pub const UncleGenerations: u64 = 0;
}
impl pallet_authorship::Config for Test {
	type EventHandler = Pallet<Test>;
	type FilterUncle = ();
	type FindAuthor = Author11;
	type UncleGenerations = UncleGenerations;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxLocks: u32 = 1024;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl onchain::Config for Test {
	type Accuracy = Perbill;
	type DataProvider = Staking;
}

parameter_types! {
	pub const StakingPalletId: PalletId = PalletId(*b"da/staki");
	pub const BondingDurationInEra: EraIndex = 3;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
	pub static SessionsPerEra: SessionIndex = 3;
	pub static BondingDurationInBlockNumber: BlockNumber = bonding_duration_in_blocks();
	pub static ExistentialDeposit: Balance = 1;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static SessionValidators: (Vec<AccountId>, HashSet<AccountId>) = Default::default();
	pub static RingRewardRemainderUnbalanced: Balance = 0;
}
impl Config for Test {
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type BondingDurationInEra = BondingDurationInEra;
	type Cap = Cap;
	type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
	type Event = Event;
	type GenesisElectionProvider = Self::ElectionProvider;
	type KtonCurrency = Kton;
	type KtonReward = ();
	type KtonSlash = ();
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type NextNewSession = Session;
	type PalletId = StakingPalletId;
	type RingCurrency = Ring;
	type RingReward = ();
	type RingRewardRemainder = RingRewardRemainderMock;
	type RingSlash = ();
	type SessionInterface = Self;
	type SessionsPerEra = SessionsPerEra;
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SlashDeferDuration = SlashDeferDuration;
	type SortedListProvider = UseNominatorsMap<Self>;
	type TotalPower = TotalPower;
	type UnixTime = SuppressUnixTimeWarning;
	type WeightInfo = ();

	const MAX_NOMINATIONS: u32 = 16;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Staking: darwinia_staking::{Pallet, Call, Storage, Config<T>, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
	}
}

pub struct ExtBuilder {
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	invulnerables: Vec<AccountId>,
	has_stakers: bool,
	initialize_first_session: bool,
	min_nominator_bond: Balance,
	min_validator_bond: Balance,
	balance_factor: Balance,
	status: BTreeMap<AccountId, StakerStatus<AccountId>>,
	stakes: BTreeMap<AccountId, Balance>,
	stakers: Vec<(AccountId, AccountId, Balance, StakerStatus<AccountId>)>,
	init_kton: bool,
}
impl ExtBuilder {
	pub fn sessions_per_era(self, length: SessionIndex) -> Self {
		SESSIONS_PER_ERA.with(|v| *v.borrow_mut() = length);
		self
	}

	pub fn period(self, length: BlockNumber) -> Self {
		PERIOD.with(|v| *v.borrow_mut() = length);
		self
	}

	pub fn existential_deposit(self, existential_deposit: Balance) -> Self {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = existential_deposit);
		self
	}

	pub fn nominate(mut self, nominate: bool) -> Self {
		self.nominate = nominate;
		self
	}

	pub fn validator_count(mut self, count: u32) -> Self {
		self.validator_count = count;
		self
	}

	pub fn minimum_validator_count(mut self, count: u32) -> Self {
		self.minimum_validator_count = count;
		self
	}

	pub fn slash_defer_duration(mut self, eras: EraIndex) -> Self {
		SLASH_DEFER_DURATION.with(|v| *v.borrow_mut() = eras);
		self
	}

	pub fn invulnerables(mut self, invulnerables: Vec<AccountId>) -> Self {
		self.invulnerables = invulnerables;
		self
	}

	pub fn has_stakers(mut self, has: bool) -> Self {
		self.has_stakers = has;
		self
	}

	pub fn initialize_first_session(mut self, init: bool) -> Self {
		self.initialize_first_session = init;
		self
	}

	pub fn offset(self, offset: BlockNumber) -> Self {
		OFFSET.with(|v| *v.borrow_mut() = offset);
		self
	}

	pub fn min_nominator_bond(mut self, amount: Balance) -> Self {
		self.min_nominator_bond = amount;
		self
	}

	pub fn min_validator_bond(mut self, amount: Balance) -> Self {
		self.min_validator_bond = amount;
		self
	}

	pub fn balance_factor(mut self, factor: Balance) -> Self {
		self.balance_factor = factor;
		self
	}

	pub fn set_status(mut self, who: AccountId, status: StakerStatus<AccountId>) -> Self {
		self.status.insert(who, status);
		self
	}

	pub fn set_stake(mut self, who: AccountId, stake: Balance) -> Self {
		self.stakes.insert(who, stake);
		self
	}

	pub fn add_staker(
		mut self,
		stash: AccountId,
		ctrl: AccountId,
		stake: Balance,
		status: StakerStatus<AccountId>,
	) -> Self {
		self.stakers.push((stash, ctrl, stake, status));
		self
	}

	pub fn init_kton(mut self, init: bool) -> Self {
		self.init_kton = init;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		sp_tracing::try_init_simple();

		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let _ = darwinia_balances::GenesisConfig::<Test, RingInstance> {
			balances: vec![
				(1, 10 * self.balance_factor),
				(2, 20 * self.balance_factor),
				(3, 300 * self.balance_factor),
				(4, 400 * self.balance_factor),
				(10, self.balance_factor),
				(11, 1000 * self.balance_factor),
				(20, self.balance_factor),
				(21, 2000 * self.balance_factor),
				(30, self.balance_factor),
				(31, 2000 * self.balance_factor),
				(40, self.balance_factor),
				(41, 2000 * self.balance_factor),
				(50, self.balance_factor),
				(51, 2000 * self.balance_factor),
				(60, self.balance_factor),
				(61, 2000 * self.balance_factor),
				(70, self.balance_factor),
				(71, 2000 * self.balance_factor),
				(80, self.balance_factor),
				(81, 2000 * self.balance_factor),
				(100, 2000 * self.balance_factor),
				(101, 2000 * self.balance_factor),
				// This allows us to have a total_payout different from 0.
				(999, 1_000_000_000_000),
			],
		}
		.assimilate_storage(&mut storage);

		if self.init_kton {
			let _ = darwinia_balances::GenesisConfig::<Test, KtonInstance> {
				balances: vec![
					(1, 10 * self.balance_factor),
					(2, 20 * self.balance_factor),
					(3, 300 * self.balance_factor),
					(4, 400 * self.balance_factor),
					(10, self.balance_factor),
					(11, 1000 * self.balance_factor),
					(20, self.balance_factor),
					(21, 2000 * self.balance_factor),
					(30, self.balance_factor),
					(31, 2000 * self.balance_factor),
					(40, self.balance_factor),
					(41, 2000 * self.balance_factor),
					(100, 2000 * self.balance_factor),
					(101, 2000 * self.balance_factor),
					// This allows us to have a total_payout different from 0.
					(999, 1_000_000_000_000),
				],
			}
			.assimilate_storage(&mut storage);
		}

		let mut stakers = vec![];

		if self.has_stakers {
			stakers = vec![
				// (stash, ctrl, stake, status)
				// these two will be elected in the default test where we elect 2.
				(11, 10, self.balance_factor * 1000, <StakerStatus<AccountId>>::Validator),
				(21, 20, self.balance_factor * 1000, <StakerStatus<AccountId>>::Validator),
				// a loser validator
				(31, 30, self.balance_factor * 500, <StakerStatus<AccountId>>::Validator),
				// an idle validator
				(41, 40, self.balance_factor * 1000, <StakerStatus<AccountId>>::Idle),
			];
			// optionally add a nominator
			if self.nominate {
				stakers.push((
					101,
					100,
					self.balance_factor * 500,
					<StakerStatus<AccountId>>::Nominator(vec![11, 21]),
				))
			}
			// replace any of the status if needed.
			self.status.into_iter().for_each(|(stash, status)| {
				let (_, _, _, ref mut prev_status) = stakers
					.iter_mut()
					.find(|s| s.0 == stash)
					.expect("set_status staker should exist; qed");
				*prev_status = status;
			});
			// replaced any of the stakes if needed.
			self.stakes.into_iter().for_each(|(stash, stake)| {
				let (_, _, ref mut prev_stake, _) = stakers
					.iter_mut()
					.find(|s| s.0 == stash)
					.expect("set_stake staker should exits; qed.");
				*prev_stake = stake;
			});
			// extend stakers if needed.
			stakers.extend(self.stakers);
		}
		let _ = darwinia_staking::GenesisConfig::<Test> {
			history_depth: 84,
			stakers,
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			slash_reward_fraction: Perbill::from_percent(10),
			min_nominator_bond: self.min_nominator_bond,
			min_validator_bond: self.min_validator_bond,
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		}
		.assimilate_storage(&mut storage);
		let _ = pallet_session::GenesisConfig::<Test> {
			keys: if self.has_stakers {
				// genesis election will overwrite this, no worries.
				Default::default()
			} else {
				// set some dummy validators in genesis.
				(0..self.validator_count as u64)
					.map(|x| (x, x, SessionKeys { other: UintAuthorityId(x as u64) }))
					.collect()
			},
		}
		.assimilate_storage(&mut storage);
		let mut ext = sp_io::TestExternalities::from(storage);

		ext.execute_with(|| {
			let validators = Session::validators();
			SESSION_VALIDATORS.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
		});

		if self.initialize_first_session {
			// We consider all test to start after timestamp is initialized This must be ensured by
			// having `timestamp::on_initialize` called before `staking::on_initialize`. Also, if
			// session length is 1, then it is already triggered.
			ext.execute_with(|| {
				System::set_block_number(1);
				Session::on_initialize(1);
				Staking::on_initialize(1);
				Timestamp::set_timestamp(INIT_TIMESTAMP);
			});
		}

		ext
	}

	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		let mut ext = self.build();
		ext.execute_with(test);
		ext.execute_with(post_conditions);
	}
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			nominate: true,
			validator_count: 2,
			minimum_validator_count: 0,
			invulnerables: vec![],
			has_stakers: true,
			initialize_first_session: true,
			min_nominator_bond: ExistentialDeposit::get(),
			min_validator_bond: ExistentialDeposit::get(),
			balance_factor: 1,
			status: Default::default(),
			stakes: Default::default(),
			stakers: Default::default(),
			init_kton: false,
		}
	}
}

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<AccountId> for Author11 {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some(11)
	}
}

pub struct RingRewardRemainderMock;
impl OnUnbalanced<RingNegativeImbalance<Test>> for RingRewardRemainderMock {
	fn on_nonzero_unbalanced(amount: RingNegativeImbalance<Test>) {
		RING_REWARD_REMAINDER_UNBALANCED.with(|v| {
			*v.borrow_mut() += amount.peek();
		});
		drop(amount);
	}
}

pub struct SuppressUnixTimeWarning;
impl UnixTime for SuppressUnixTimeWarning {
	fn now() -> core::time::Duration {
		core::time::Duration::from_millis(Timestamp::now() as _)
	}
}

fn post_conditions() {
	check_nominators();
	check_exposures();
	check_ledgers();
	check_count();
}

fn check_count() {
	let nominator_count = <Nominators<Test>>::iter().count() as u32;
	let validator_count = <Validators<Test>>::iter().count() as u32;

	assert_eq!(nominator_count, <CounterForNominators<Test>>::get());
	assert_eq!(validator_count, <CounterForValidators<Test>>::get());

	// the voters that the `SortedListProvider` list is storing for us.
	let external_voters = <Test as Config>::SortedListProvider::count();

	assert_eq!(external_voters, nominator_count);
}

fn check_ledgers() {
	// check the ledger of all stakers.
	<Bonded<Test>>::iter().for_each(|(_, controller)| assert_ledger_consistent(controller))
}

fn check_exposures() {
	// a check per validator to ensure the exposure struct is always sane.
	let era = active_era();
	<ErasStakers<Test>>::iter_prefix_values(era).for_each(|expo| {
		assert_eq!(
			expo.total_power,
			expo.own_power + expo.others.iter().map(|e| e.power).sum::<Power>(),
			"wrong total exposure.",
		);
	})
}

fn check_nominators() {
	// a check per nominator to ensure their entire stake is correctly distributed. Will only kick-
	// in if the nomination was submitted before the current era.
	let era = active_era();
	<Nominators<Test>>::iter()
		.filter_map(
			|(nominator, nomination)| {
				if nomination.submitted_in > era {
					Some(nominator)
				} else {
					None
				}
			},
		)
		.for_each(|nominator| {
			// must be bonded.
			assert_is_stash(nominator);
			let mut sum = 0;
			Session::validators().iter().map(|v| Staking::eras_stakers(era, v)).for_each(|e| {
				let individual = e.others.iter().filter(|e| e.who == nominator).collect::<Vec<_>>();
				let len = individual.len();
				match len {
					0 => { /* not supporting this validator at all. */ },
					1 => sum += individual[0].power,
					_ => panic!("nominator cannot back a validator more than once."),
				};
			});

			let nominator_stake = Staking::power_of(&nominator);
			// a nominator cannot over-spend.
			assert!(
				nominator_stake >= sum,
				"failed: Nominator({}) stake({}) >= sum divided({})",
				nominator,
				nominator_stake,
				sum,
			);

			let diff = nominator_stake - sum;
			assert!(diff < 100);
		});
}

fn assert_is_stash(acc: AccountId) {
	assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

pub fn assert_ledger_consistent(controller: AccountId) {
	let ledger = Staking::ledger(controller).unwrap();
	let real_total_ring: Balance =
		ledger.ring_staking_lock.unbondings.iter().fold(ledger.active, |a, c| a + c.amount);
	let real_total_kton: Balance =
		ledger.kton_staking_lock.unbondings.iter().fold(ledger.active_kton, |a, c| a + c.amount);

	assert!(
		ledger.active >= Ring::minimum_balance()
			|| ledger.active_kton >= Kton::minimum_balance()
			|| (ledger.active == 0 && ledger.active_kton == 0),
		"{}: active ledger amount ({}/{}) must be greater than ED {}/{}",
		controller,
		ledger.active,
		ledger.active_kton,
		Ring::minimum_balance(),
		Kton::minimum_balance()
	);
}

pub fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
}

pub fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
}

fn bond(stash: AccountId, controller: AccountId, val: StakingBalanceT<Test>) {
	match val {
		StakingBalance::RingBalance(r) => {
			let _ = Ring::make_free_balance_be(&(stash), r);
			let _ = Ring::make_free_balance_be(&(controller), r);
		},
		StakingBalance::KtonBalance(k) => {
			let _ = Kton::make_free_balance_be(&(stash), k);
			let _ = Kton::make_free_balance_be(&(controller), k);
		},
	}
	assert_ok!(Staking::bond(
		Origin::signed(stash),
		controller,
		val,
		RewardDestination::Controller,
		0,
	));
}

pub fn bond_validator(stash: AccountId, controller: AccountId, val: StakingBalanceT<Test>) {
	bond(stash, controller, val);
	assert_ok!(Staking::validate(Origin::signed(controller), ValidatorPrefs::default()));
}

pub fn bond_nominator(
	stash: AccountId,
	controller: AccountId,
	val: StakingBalanceT<Test>,
	target: Vec<AccountId>,
) {
	bond(stash, controller, val);
	assert_ok!(Staking::nominate(Origin::signed(controller), target));
}

/// Progress to the given block, triggering session and era changes as we progress.
///
/// This will finalize the previous block, initialize up to the given block, essentially simulating
/// a block import/propose process where we first initialize the block, then execute some stuff (not
/// in the function), and then finalize the block.
pub fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		Staking::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

/// Progresses from the current block number (whatever that may be) to the `P * session_index + 1`.
pub fn start_session(session_index: SessionIndex) {
	let end: u64 = if Offset::get().is_zero() {
		(session_index as u64) * Period::get()
	} else {
		Offset::get() + (session_index.saturating_sub(1) as u64) * Period::get()
	};
	run_to_block(end);
	// session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

pub fn advance_session() {
	let current_index = Session::current_index();
	start_session(current_index + 1);
}

/// Progress until the given era.
pub fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era, so they must match
	// at this point.
	assert_eq!(current_era(), active_era());
}

pub fn current_total_payout_for_duration(duration: TsInMs) -> Balance {
	inflation::compute_total_payout::<Test>(
		duration,
		Staking::living_time(),
		<Test as Config>::Cap::get() - Ring::total_issuance(),
		Perbill::from_percent(50),
	)
	.0
}

pub fn maximum_payout_for_duration(duration: u64) -> Balance {
	inflation::compute_total_payout::<Test>(
		duration,
		Staking::living_time(),
		<Test as Config>::Cap::get() - Ring::total_issuance(),
		Perbill::from_percent(50),
	)
	.1
}

/// Time it takes to finish a session.
///
/// Note, if you see `time_per_session() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub fn time_per_session() -> u64 {
	Period::get() * BLOCK_TIME
}

/// Time it takes to finish an era.
///
/// Note, if you see `time_per_era() - BLOCK_TIME`, it is fine. This is because we set the
/// timestamp after on_initialize, so the timestamp is always one block old.
pub fn time_per_era() -> u64 {
	time_per_session() * SessionsPerEra::get() as u64
}

/// Time that will be calculated for the reward per era.
pub fn reward_time_per_era() -> u64 {
	time_per_era() - BLOCK_TIME
}

pub fn bonding_duration_in_blocks() -> BlockNumber {
	BondingDurationInEra::get() as BlockNumber * Period::get()
}

pub fn reward_all_elected() {
	let rewards = <Test as Config>::SessionInterface::validators().into_iter().map(|v| (v, 1));

	Staking::reward_by_ids(rewards)
}

pub fn validator_controllers() -> Vec<AccountId> {
	Session::validators()
		.into_iter()
		.map(|s| Staking::bonded(&s).expect("no controller for validator"))
		.collect()
}

pub fn on_offence_in_era(
	offenders: &[OffenceDetails<
		AccountId,
		pallet_session::historical::IdentificationTuple<Test>,
	>],
	slash_fraction: &[Perbill],
	era: EraIndex,
) {
	let bonded_eras = <BondedEras<Test>>::get();
	for &(bonded_era, start_session) in bonded_eras.iter() {
		if bonded_era == era {
			let _ = Staking::on_offence(offenders, slash_fraction, start_session);
			return;
		} else if bonded_era > era {
			break;
		}
	}

	if active_era() == era {
		Staking::on_offence(
			offenders,
			slash_fraction,
			Staking::eras_start_session_index(era).unwrap(),
		);
	} else {
		panic!("cannot slash in era {}", era);
	}
}

pub fn on_offence_now(
	offenders: &[OffenceDetails<
		AccountId,
		pallet_session::historical::IdentificationTuple<Test>,
	>],
	slash_fraction: &[Perbill],
) {
	let now = active_era();
	on_offence_in_era(offenders, slash_fraction, now)
}

pub fn add_slash(who: &AccountId) {
	on_offence_now(
		&[OffenceDetails {
			offender: (who.clone(), Staking::eras_stakers(active_era(), who.clone())),
			reporters: vec![],
		}],
		&[Perbill::from_percent(10)],
	);
}

/// Make all validator and nominator request their payment
pub fn make_all_reward_payment(era: EraIndex) {
	let validators_with_reward =
		<ErasRewardPoints<Test>>::get(era).individual.keys().cloned().collect::<Vec<_>>();

	// reward validators
	for validator_controller in validators_with_reward.iter().filter_map(Staking::bonded) {
		let ledger = <Ledger<Test>>::get(&validator_controller).unwrap();

		assert_ok!(Staking::payout_stakers(Origin::signed(1337), ledger.stash, era));
	}
}

pub fn staking_events() -> Vec<darwinia_staking::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| if let Event::Staking(inner) = e { Some(inner) } else { None })
		.collect()
}

pub fn ring_balances(who: &AccountId) -> (Balance, Balance) {
	(Ring::free_balance(who), Ring::reserved_balance(who))
}
pub fn kton_balances(who: &AccountId) -> (Balance, Balance) {
	(Kton::free_balance(who), Kton::reserved_balance(who))
}

pub fn ring_power(stake: Balance) -> Power {
	Staking::currency_to_power(stake, Staking::ring_pool())
}

#[macro_export]
macro_rules! assert_session_era {
	($session:expr, $era:expr) => {
		assert_eq!(
			Session::current_index(),
			$session,
			"wrong session {} != {}",
			Session::current_index(),
			$session,
		);
		assert_eq!(current_era(), $era, "wrong current era {} != {}", current_era(), $era,);
	};
}
