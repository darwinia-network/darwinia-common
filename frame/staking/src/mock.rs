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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Test utilities

#![allow(unused)]

mod staking {
	pub use crate::Event;
}

// --- std ---
use std::{cell::RefCell, collections::HashSet};
// --- substrate ---
use frame_support::{
	assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
	storage::IterableStorageMap,
	traits::{Currency, FindAuthor, Get, OnFinalize, OnInitialize},
	weights::{constants::RocksDbWeight, Weight},
	StorageValue,
};
use sp_core::H256;
use sp_npos_elections::{reduce, StakedAssignment};
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::IdentityLookup,
	Perbill,
};
use sp_staking::{
	offence::{OffenceDetails, OnOffenceHandler},
	SessionIndex,
};
// --- darwinia ---
use crate::*;

pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub(crate) type Extrinsic = TestXt<Call, ()>;

pub(crate) type System = frame_system::Module<Test>;
pub(crate) type Session = pallet_session::Module<Test>;
pub(crate) type Timestamp = pallet_timestamp::Module<Test>;
pub(crate) type Ring = darwinia_balances::Module<Test, RingInstance>;
pub(crate) type Kton = darwinia_balances::Module<Test, KtonInstance>;
pub(crate) type Staking = Module<Test>;

pub(crate) type RingError = darwinia_balances::Error<Test, RingInstance>;
pub(crate) type StakingError = Error<Test>;

pub(crate) const NANO: Balance = 1;
pub(crate) const MICRO: Balance = 1_000 * NANO;
pub(crate) const MILLI: Balance = 1_000 * MICRO;
pub(crate) const COIN: Balance = 1_000 * MILLI;

pub(crate) const CAP: Balance = 10_000_000_000 * COIN;
pub(crate) const TOTAL_POWER: Power = 1_000_000_000;

pub const INIT_TIMESTAMP: TsInMs = 30_000;

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
	static SESSION_PER_ERA: RefCell<SessionIndex> = RefCell::new(3);
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
	static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
	static ELECTION_LOOKAHEAD: RefCell<BlockNumber> = RefCell::new(0);
	static PERIOD: RefCell<BlockNumber> = RefCell::new(1);
	static MAX_ITERATIONS: RefCell<u32> = RefCell::new(0);
	pub static RING_REWARD_REMAINDER_UNBALANCED: RefCell<Balance> = RefCell::new(0);
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		staking::Staking,
	}
}

impl_outer_event! {
	pub enum MetaEvent for Test {
		frame_system <T>,
		pallet_session,
		darwinia_balances Instance0<T>,
		darwinia_balances Instance1<T>,
		staking <T>,
	}
}

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

darwinia_support::impl_test_account_data! {}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;
impl pallet_session::OneSessionHandler<AccountId> for OtherSessionHandler {
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
		SESSION.with(|x| {
			*x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new())
		});
	}

	fn on_disabled(validator_index: usize) {
		SESSION.with(|d| {
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
	SESSION.with(|d| d.borrow().1.contains(&stash))
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const StakingModuleId: ModuleId = ModuleId(*b"da/staki");
	pub const BondingDurationInEra: EraIndex = 3;
	pub const BondingDurationInBlockNumber: BlockNumber = 9;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const UnsignedPriority: u64 = 1 << 20;
	pub const MinSolutionScoreBump: Perbill = Perbill::zero();
	pub const OffchainSolutionWeightLimit: Weight = MaximumBlockWeight::get();
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl Trait for Test {
	type Event = MetaEvent;
	type ModuleId = StakingModuleId;
	type UnixTime = SuppressUnixTimeWarning;
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = UnsignedPriority;
	type OffchainSolutionWeightLimit = OffchainSolutionWeightLimit;
	type RingCurrency = Ring;
	type RingRewardRemainder = RingRewardRemainderMock;
	type RingSlash = ();
	type RingReward = ();
	type KtonCurrency = Kton;
	type KtonSlash = ();
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = frame_support::weights::constants::WEIGHT_PER_SECOND * 2;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = AccountIndex;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = MetaEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = RocksDbWeight;
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

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
parameter_types! {
	pub const Offset: BlockNumber = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl pallet_session::Trait for Test {
	type Event = MetaEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf<Test>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = (OtherSessionHandler,);
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

parameter_types! {
	pub const UncleGenerations: u64 = 0;
}
impl pallet_authorship::Trait for Test {
	type FindAuthor = Author11;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = Module<Test>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxLocks: u32 = 1024;
}
impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = MetaEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = ();
	type WeightInfo = ();
}
impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = MetaEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = ();
	type WeightInfo = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

pub struct ExtBuilder {
	session_length: BlockNumber,
	election_lookahead: BlockNumber,
	session_per_era: SessionIndex,
	existential_deposit: Balance,
	validator_pool: bool,
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	slash_defer_duration: EraIndex,
	fair: bool,
	num_validators: Option<u32>,
	invulnerables: Vec<AccountId>,
	has_stakers: bool,
	max_offchain_iterations: u32,
	init_ring: bool,
	init_kton: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			session_length: 1,
			election_lookahead: 0,
			session_per_era: 3,
			existential_deposit: 1,
			validator_pool: false,
			nominate: true,
			validator_count: 2,
			minimum_validator_count: 0,
			slash_defer_duration: 0,
			fair: true,
			num_validators: None,
			invulnerables: vec![],
			has_stakers: true,
			max_offchain_iterations: 0,
			init_ring: true,
			init_kton: false,
		}
	}
}

impl ExtBuilder {
	pub fn session_per_era(mut self, length: SessionIndex) -> Self {
		self.session_per_era = length;
		self
	}
	pub fn election_lookahead(mut self, look: BlockNumber) -> Self {
		self.election_lookahead = look;
		self
	}
	pub fn session_length(mut self, length: BlockNumber) -> Self {
		self.session_length = length;
		self
	}
	pub fn existential_deposit(mut self, existential_deposit: Balance) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn validator_pool(mut self, validator_pool: bool) -> Self {
		self.validator_pool = validator_pool;
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
		self.slash_defer_duration = eras;
		self
	}
	pub fn fair(mut self, is_fair: bool) -> Self {
		self.fair = is_fair;
		self
	}
	pub fn num_validators(mut self, num_validators: u32) -> Self {
		self.num_validators = Some(num_validators);
		self
	}
	pub fn invulnerables(mut self, invulnerables: Vec<AccountId>) -> Self {
		self.invulnerables = invulnerables;
		self
	}
	pub fn has_stakers(mut self, has: bool) -> Self {
		self.has_stakers = has;
		self.init_ring = has;
		self
	}
	pub fn max_offchain_iterations(mut self, iterations: u32) -> Self {
		self.max_offchain_iterations = iterations;
		self
	}
	pub fn init_ring(mut self, init_ring: bool) -> Self {
		self.init_ring = init_ring;
		self.has_stakers = init_ring;
		self
	}
	pub fn init_kton(mut self, init_kton: bool) -> Self {
		self.init_kton = init_kton;
		self
	}
	pub fn offchain_election_ext(self) -> Self {
		self.session_per_era(4)
			.session_length(5)
			.election_lookahead(3)
	}
	pub fn set_associated_constants(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		SLASH_DEFER_DURATION.with(|v| *v.borrow_mut() = self.slash_defer_duration);
		SESSION_PER_ERA.with(|v| *v.borrow_mut() = self.session_per_era);
		ELECTION_LOOKAHEAD.with(|v| *v.borrow_mut() = self.election_lookahead);
		PERIOD.with(|v| *v.borrow_mut() = self.session_length);
		MAX_ITERATIONS.with(|v| *v.borrow_mut() = self.max_offchain_iterations);
	}
	pub fn build(self) -> sp_io::TestExternalities {
		sp_tracing::try_init_simple();
		self.set_associated_constants();
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

		let num_validators = self.num_validators.unwrap_or(self.validator_count);
		let validators = (0..num_validators)
			.map(|x| ((x + 1) * 10 + 1) as AccountId)
			.collect::<Vec<_>>();

		if self.init_ring {
			let _ = darwinia_balances::GenesisConfig::<Test, RingInstance> {
				balances: vec![
					(1, 10 * balance_factor),
					(2, 20 * balance_factor),
					(3, 300 * balance_factor),
					(4, 400 * balance_factor),
					(10, balance_factor),
					(11, balance_factor * 1000),
					(20, balance_factor),
					(21, balance_factor * 2000),
					(30, balance_factor),
					(31, balance_factor * 2000),
					(40, balance_factor),
					(41, balance_factor * 2000),
					(100, 2000 * balance_factor),
					(101, 2000 * balance_factor),
					// This allows us to have a total_payout different from 0.
					(999, 1_000_000_000_000),
				],
			}
			.assimilate_storage(&mut storage);
		}
		if self.init_kton {
			let _ = darwinia_balances::GenesisConfig::<Test, KtonInstance> {
				balances: vec![
					(1, 10 * balance_factor),
					(2, 20 * balance_factor),
					(3, 300 * balance_factor),
					(4, 400 * balance_factor),
					(10, balance_factor),
					(11, balance_factor * 1000),
					(20, balance_factor),
					(21, balance_factor * 2000),
					(30, balance_factor),
					(31, balance_factor * 2000),
					(40, balance_factor),
					(41, balance_factor * 2000),
					(100, 2000 * balance_factor),
					(101, 2000 * balance_factor),
					// This allows us to have a total_payout different from 0.
					(999, 1_000_000_000_000),
				],
			}
			.assimilate_storage(&mut storage);
		}

		let mut stakers = vec![];
		if self.has_stakers {
			let stake_21 = if self.fair { 1000 } else { 2000 };
			let stake_31 = if self.validator_pool {
				balance_factor * 1000
			} else {
				1
			};
			let status_41 = if self.validator_pool {
				StakerStatus::<AccountId>::Validator
			} else {
				StakerStatus::<AccountId>::Idle
			};
			let nominated = if self.nominate { vec![11, 21] } else { vec![] };
			stakers = vec![
				// (stash, controller, staked_amount, status)
				(
					11,
					10,
					balance_factor * 1000,
					StakerStatus::<AccountId>::Validator,
				),
				(21, 20, stake_21, StakerStatus::<AccountId>::Validator),
				(31, 30, stake_31, StakerStatus::<AccountId>::Validator),
				(41, 40, balance_factor * 1000, status_41),
				// nominator
				(
					101,
					100,
					balance_factor * 500,
					StakerStatus::<AccountId>::Nominator(nominated),
				),
			];
		}
		let _ = GenesisConfig::<Test> {
			history_depth: 84,
			stakers,
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			slash_reward_fraction: Perbill::from_percent(10),
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = pallet_session::GenesisConfig::<Test> {
			keys: validators
				.iter()
				.map(|x| {
					(
						*x,
						*x,
						SessionKeys {
							other: UintAuthorityId(*x as u64),
						},
					)
				})
				.collect(),
		}
		.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			let validators = Session::validators();
			SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
		});
		// We consider all test to start after timestamp is initialized
		// This must be ensured by having `timestamp::on_initialize` called before
		// `staking::on_initialize`
		ext.execute_with(|| {
			System::set_block_number(1);
			Timestamp::set_timestamp(INIT_TIMESTAMP);
		});

		ext
	}
	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		let mut ext = self.build();
		ext.execute_with(test);
		ext.execute_with(post_conditions);
	}
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
	fn get() -> Balance {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

pub struct SessionsPerEra;
impl Get<SessionIndex> for SessionsPerEra {
	fn get() -> SessionIndex {
		SESSION_PER_ERA.with(|v| *v.borrow())
	}
}
impl Get<BlockNumber> for SessionsPerEra {
	fn get() -> BlockNumber {
		SESSION_PER_ERA.with(|v| *v.borrow() as BlockNumber)
	}
}

pub struct ElectionLookahead;
impl Get<BlockNumber> for ElectionLookahead {
	fn get() -> BlockNumber {
		ELECTION_LOOKAHEAD.with(|v| *v.borrow())
	}
}

pub struct Period;
impl Get<BlockNumber> for Period {
	fn get() -> BlockNumber {
		PERIOD.with(|v| *v.borrow())
	}
}

pub struct SlashDeferDuration;
impl Get<EraIndex> for SlashDeferDuration {
	fn get() -> EraIndex {
		SLASH_DEFER_DURATION.with(|v| *v.borrow())
	}
}

pub struct MaxIterations;
impl Get<u32> for MaxIterations {
	fn get() -> u32 {
		MAX_ITERATIONS.with(|v| *v.borrow())
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
		core::time::Duration::from_millis(Timestamp::now().saturated_into::<u64>())
	}
}

fn post_conditions() {
	check_nominators();
	check_exposures();
	check_ledgers();
}

pub(crate) fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
}

pub(crate) fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
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
		.filter_map(|(nominator, nomination)| {
			if nomination.submitted_in > era {
				Some(nominator)
			} else {
				None
			}
		})
		.for_each(|nominator| {
			// must be bonded.
			assert_is_stash(nominator);
			let mut sum = 0;
			Session::validators()
				.iter()
				.map(|v| Staking::eras_stakers(era, v))
				.for_each(|e| {
					let individual = e
						.others
						.iter()
						.filter(|e| e.who == nominator)
						.collect::<Vec<_>>();
					let len = individual.len();
					match len {
						0 => { /* not supporting this validator at all. */ }
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
	assert_eq!(ledger.active_ring, ledger.ring_staking_lock.staking_amount);
	assert_eq!(ledger.active_kton, ledger.kton_staking_lock.staking_amount);
}

fn bond(stash: AccountId, controller: AccountId, val: StakingBalanceT<Test>) {
	match val {
		StakingBalance::RingBalance(r) => {
			let _ = Ring::make_free_balance_be(&(stash), r);
			let _ = Ring::make_free_balance_be(&(controller), r);
		}
		StakingBalance::KtonBalance(k) => {
			let _ = Kton::make_free_balance_be(&(stash), k);
			let _ = Kton::make_free_balance_be(&(controller), k);
		}
	}
	assert_ok!(Staking::bond(
		Origin::signed(stash),
		controller,
		val,
		RewardDestination::Controller,
		0,
	));
}

pub(crate) fn bond_validator(stash: AccountId, controller: AccountId, val: StakingBalanceT<Test>) {
	bond(stash, controller, val);
	assert_ok!(Staking::validate(
		Origin::signed(controller),
		ValidatorPrefs::default()
	));
}

pub(crate) fn bond_nominator(
	stash: AccountId,
	controller: AccountId,
	val: StakingBalanceT<Test>,
	target: Vec<AccountId>,
) {
	bond(stash, controller, val);
	assert_ok!(Staking::nominate(Origin::signed(controller), target));
}

pub(crate) fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		Staking::on_initialize(b);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

pub(crate) fn advance_session() {
	let current_index = Session::current_index();
	start_session(current_index + 1);
}

pub(crate) fn start_session(session_index: SessionIndex) {
	assert_eq!(
		<Period as Get<BlockNumber>>::get(),
		1,
		"start_session can only be used with session length 1."
	);
	for i in Session::current_index()..session_index {
		Staking::on_finalize(System::block_number());
		System::set_block_number((i + 1).into());
		Timestamp::set_timestamp(System::block_number() * 1000 + INIT_TIMESTAMP);
		Session::on_initialize(System::block_number());
		Staking::on_initialize(System::block_number());
	}

	assert_eq!(Session::current_index(), session_index);
}

pub(crate) fn start_era(era_index: EraIndex) {
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(Staking::current_era().unwrap(), era_index);
}

pub(crate) fn current_total_payout_for_duration(era_duration: TsInMs) -> Balance {
	inflation::compute_total_payout::<Test>(
		era_duration,
		Staking::living_time(),
		<Test as Trait>::Cap::get() - Ring::total_issuance(),
		Perbill::from_percent(50),
	)
	.0
}

pub(crate) fn reward_all_elected() {
	let rewards = <Test as Trait>::SessionInterface::validators()
		.into_iter()
		.map(|v| (v, 1));

	Staking::reward_by_ids(rewards)
}

pub(crate) fn validator_controllers() -> Vec<AccountId> {
	Session::validators()
		.into_iter()
		.map(|s| Staking::bonded(&s).expect("no controller for validator"))
		.collect()
}

pub(crate) fn on_offence_in_era(
	offenders: &[OffenceDetails<
		AccountId,
		pallet_session::historical::IdentificationTuple<Test>,
	>],
	slash_fraction: &[Perbill],
	era: EraIndex,
) {
	let bonded_eras = BondedEras::get();
	for &(bonded_era, start_session) in bonded_eras.iter() {
		if bonded_era == era {
			let _ = Staking::on_offence(offenders, slash_fraction, start_session).unwrap();
			return;
		} else if bonded_era > era {
			break;
		}
	}

	if Staking::active_era().unwrap().index == era {
		Staking::on_offence(
			offenders,
			slash_fraction,
			Staking::eras_start_session_index(era).unwrap(),
		)
		.unwrap();
	} else {
		panic!("cannot slash in era {}", era);
	}
}

pub(crate) fn on_offence_now(
	offenders: &[OffenceDetails<
		AccountId,
		pallet_session::historical::IdentificationTuple<Test>,
	>],
	slash_fraction: &[Perbill],
) {
	let now = Staking::active_era().unwrap().index;
	on_offence_in_era(offenders, slash_fraction, now)
}

pub(crate) fn add_slash(who: &AccountId) {
	on_offence_now(
		&[OffenceDetails {
			offender: (
				who.clone(),
				Staking::eras_stakers(Staking::active_era().unwrap().index, who.clone()),
			),
			reporters: vec![],
		}],
		&[Perbill::from_percent(10)],
	);
}

// winners will be chosen by simply their unweighted total backing stake. Nominator stake is
// distributed evenly.
pub(crate) fn horrible_npos_solution(
	do_reduce: bool,
) -> (CompactAssignments, Vec<ValidatorIndex>, ElectionScore) {
	let mut backing_stake_of: BTreeMap<AccountId, Balance> = BTreeMap::new();

	// self stake
	<Validators<Test>>::iter().for_each(|(who, _p)| {
		*backing_stake_of.entry(who).or_insert(0) += Staking::power_of(&who) as Balance
	});

	// add nominator stuff
	<Nominators<Test>>::iter().for_each(|(who, nomination)| {
		nomination.targets.iter().for_each(|v| {
			*backing_stake_of.entry(*v).or_insert(0) += Staking::power_of(&who) as Balance
		})
	});

	// elect winners
	let mut sorted: Vec<AccountId> = backing_stake_of.keys().cloned().collect();
	sorted.sort_by_key(|x| backing_stake_of.get(x).unwrap());
	let winners: Vec<AccountId> = sorted
		.iter()
		.cloned()
		.take(Staking::validator_count() as usize)
		.collect();

	// create assignments
	let mut staked_assignment: Vec<StakedAssignment<AccountId>> = Vec::new();
	<Nominators<Test>>::iter().for_each(|(who, nomination)| {
		let mut dist: Vec<(AccountId, ExtendedBalance)> = Vec::new();
		nomination.targets.iter().for_each(|v| {
			if winners.iter().find(|w| *w == v).is_some() {
				dist.push((*v, ExtendedBalance::zero()));
			}
		});

		if dist.len() == 0 {
			return;
		}

		// assign real stakes. just split the stake.
		let stake = Staking::power_of(&who) as ExtendedBalance;
		let mut sum: ExtendedBalance = Zero::zero();
		let dist_len = dist.len();
		{
			dist.iter_mut().for_each(|(_, w)| {
				let partial = stake / (dist_len as ExtendedBalance);
				*w = partial;
				sum += partial;
			});
		}

		// assign the leftover to last.
		{
			let leftover = stake - sum;
			let last = dist.last_mut().unwrap();
			last.1 += leftover;
		}

		staked_assignment.push(StakedAssignment {
			who,
			distribution: dist,
		});
	});

	// Ensure that this result is worse than seq-phragmen. Otherwise, it should not have been used
	// for testing.
	let score = {
		let (_, _, better_score) = prepare_submission_with(true, true, 0, |_| {});

		let support = build_support_map::<AccountId>(&winners, &staked_assignment).unwrap();
		let score = evaluate_support(&support);

		assert!(sp_npos_elections::is_score_better::<Perbill>(
			better_score,
			score,
			MinSolutionScoreBump::get(),
		));

		score
	};

	if do_reduce {
		reduce(&mut staked_assignment);
	}

	let snapshot_validators = Staking::snapshot_validators().unwrap();
	let snapshot_nominators = Staking::snapshot_nominators().unwrap();
	let nominator_index = |a: &AccountId| -> Option<NominatorIndex> {
		snapshot_nominators
			.iter()
			.position(|x| x == a)
			.map(|i| i as NominatorIndex)
	};
	let validator_index = |a: &AccountId| -> Option<ValidatorIndex> {
		snapshot_validators
			.iter()
			.position(|x| x == a)
			.map(|i| i as ValidatorIndex)
	};

	// convert back to ratio assignment. This takes less space.
	let assignments_reduced = sp_npos_elections::assignment_staked_to_ratio::<
		AccountId,
		OffchainAccuracy,
	>(staked_assignment);

	let compact =
		CompactAssignments::from_assignment(assignments_reduced, nominator_index, validator_index)
			.unwrap();

	// winner ids to index
	let winners = winners
		.into_iter()
		.map(|w| validator_index(&w).unwrap())
		.collect::<Vec<_>>();

	(compact, winners, score)
}

/// Note: this should always logically reproduce [`offchain_election::prepare_submission`], yet we
/// cannot do it since we want to have `tweak` injected into the process.
///
/// If the input is being tweaked in a way that the score cannot be compute accurately,
/// `compute_real_score` can be set to true. In this case a `Default` score is returned.
pub(crate) fn prepare_submission_with(
	compute_real_score: bool,
	do_reduce: bool,
	iterations: usize,
	tweak: impl FnOnce(&mut Vec<StakedAssignment<AccountId>>),
) -> (CompactAssignments, Vec<ValidatorIndex>, ElectionScore) {
	// run election on the default stuff.
	let sp_npos_elections::ElectionResult {
		winners,
		assignments,
	} = Staking::do_phragmen::<OffchainAccuracy>(iterations).unwrap();
	let winners = sp_npos_elections::to_without_backing(winners);

	let mut staked = sp_npos_elections::assignment_ratio_to_staked(assignments, |stash| {
		Staking::power_of(stash) as _
	});

	// apply custom tweaks. awesome for testing.
	tweak(&mut staked);

	if do_reduce {
		reduce(&mut staked);
	}

	// convert back to ratio assignment. This takes less space.
	let snapshot_validators = Staking::snapshot_validators().expect("snapshot not created.");
	let snapshot_nominators = Staking::snapshot_nominators().expect("snapshot not created.");
	let nominator_index = |a: &AccountId| -> Option<NominatorIndex> {
		snapshot_nominators.iter().position(|x| x == a).map_or_else(
			|| {
				println!("unable to find nominator index for {:?}", a);
				None
			},
			|i| Some(i as NominatorIndex),
		)
	};
	let validator_index = |a: &AccountId| -> Option<ValidatorIndex> {
		snapshot_validators.iter().position(|x| x == a).map_or_else(
			|| {
				println!("unable to find validator index for {:?}", a);
				None
			},
			|i| Some(i as ValidatorIndex),
		)
	};

	let assignments_reduced = sp_npos_elections::assignment_staked_to_ratio(staked);

	// re-compute score by converting, yet again, into staked type
	let score = if compute_real_score {
		let staked =
			sp_npos_elections::assignment_ratio_to_staked(assignments_reduced.clone(), |stash| {
				Staking::power_of(stash) as _
			});

		let support_map =
			build_support_map::<AccountId>(winners.as_slice(), staked.as_slice()).unwrap();
		evaluate_support::<AccountId>(&support_map)
	} else {
		Default::default()
	};

	let compact =
		CompactAssignments::from_assignment(assignments_reduced, nominator_index, validator_index)
			.map_err(|e| {
				println!("error in compact: {:?}", e);
				e
			})
			.expect("Failed to create compact");

	// winner ids to index
	let winners = winners
		.into_iter()
		.map(|w| validator_index(&w).unwrap())
		.collect::<Vec<_>>();

	(compact, winners, score)
}

/// Make all validator and nominator request their payment
pub(crate) fn make_all_reward_payment(era: EraIndex) {
	let validators_with_reward = <ErasRewardPoints<Test>>::get(era)
		.individual
		.keys()
		.cloned()
		.collect::<Vec<_>>();

	// reward validators
	for validator_controller in validators_with_reward.iter().filter_map(Staking::bonded) {
		let ledger = <Ledger<Test>>::get(&validator_controller).unwrap();

		assert_ok!(Staking::payout_stakers(
			Origin::signed(1337),
			ledger.stash,
			era
		));
	}
}

pub(crate) fn staking_events() -> Vec<Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let MetaEvent::staking(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect()
}

pub(crate) fn ring_balances(who: &AccountId) -> (Balance, Balance) {
	(Ring::free_balance(who), Ring::reserved_balance(who))
}
pub(crate) fn kton_balances(who: &AccountId) -> (Balance, Balance) {
	(Kton::free_balance(who), Kton::reserved_balance(who))
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
		assert_eq!(
			Staking::active_era().unwrap().index,
			$era,
			"wrong active era {} != {}",
			Staking::active_era().unwrap().index,
			$era,
		);
	};
}
