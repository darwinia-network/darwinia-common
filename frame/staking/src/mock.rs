//! Test utilities

// --- std ---
use std::{
	cell::RefCell,
	collections::{HashMap, HashSet},
};
// --- substrate ---
use frame_support::{
	assert_ok, impl_outer_origin, parameter_types,
	storage::IterableStorageMap,
	traits::{Currency, FindAuthor, Get},
	weights::Weight,
	StorageValue,
};
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
	testing::{Header, UintAuthorityId},
	traits::{IdentityLookup, OnFinalize, OnInitialize, OpaqueKeys},
	{KeyTypeId, Perbill},
};
use sp_staking::{
	offence::{OffenceDetails, OnOffenceHandler},
	SessionIndex,
};
// --- darwinia ---
use crate::*;

pub type AccountId = u64;
pub type Balance = u128;
type BlockNumber = u64;

pub type RingInstance = darwinia_balances::Instance0;
pub type RingError = darwinia_balances::Error<Test, RingInstance>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

pub type KtonInstance = darwinia_balances::Instance1;
pub type _KtonError = darwinia_balances::Error<Test, KtonInstance>;
pub type Kton = darwinia_balances::Module<Test, KtonInstance>;

pub type System = frame_system::Module<Test>;
pub type Session = pallet_session::Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;

pub type StakingError = Error<Test>;
pub type Staking = Module<Test>;

darwinia_support::impl_account_data! {
	pub struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
		// other data
	}
}

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const CAP: Balance = 10_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
	static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		_changed: bool,
		validators: &[(AccountId, Ks)],
		_queued_validators: &[(AccountId, Ks)],
	) {
		SESSION.with(|x| {
			*x.borrow_mut() = (
				validators.iter().map(|x| x.0.clone()).collect(),
				HashSet::new(),
			)
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

pub fn is_disabled(controller: AccountId) -> bool {
	let stash = Staking::ledger(&controller).unwrap().stash;
	SESSION.with(|d| d.borrow().1.contains(&stash))
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
	fn get() -> Balance {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

pub struct SlashDeferDuration;
impl Get<EraIndex> for SlashDeferDuration {
	fn get() -> EraIndex {
		SLASH_DEFER_DURATION.with(|v| *v.borrow())
	}
}

impl_outer_origin! {
	pub enum Origin for Test  where system = system {}
}

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<u64> for Author11 {
	fn find_author<'a, I>(_digests: I) -> Option<u64>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some(11)
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type MigrateAccount = ();
}

parameter_types! {
	pub const Period: BlockNumber = 1;
	pub const Offset: BlockNumber = 0;
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl pallet_session::Trait for Test {
	type Event = ();
	type ValidatorId = AccountId;
	type ValidatorIdOf = crate::StashOf<Test>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = TestSessionHandler;
	type Keys = UintAuthorityId;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
}

impl pallet_session::historical::Trait for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
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
}

impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Kton,);
}
impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Ring,);
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 3;
	pub const BondingDurationInEra: EraIndex = 3;
	pub const BondingDurationInBlockNumber: BlockNumber = 9;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;

	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl Trait for Test {
	type Time = pallet_timestamp::Module<Self>;
	type Event = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	type SlashCancelOrigin = system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type RingCurrency = Ring;
	type RingRewardRemainder = ();
	type RingSlash = ();
	type RingReward = ();
	type KtonCurrency = Kton;
	type KtonSlash = ();
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
}

pub struct ExtBuilder {
	existential_deposit: Balance,
	validator_pool: bool,
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	slash_defer_duration: EraIndex,
	fair: bool,
	num_validators: Option<u32>,
	invulnerables: Vec<AccountId>,
	init_ring: bool,
	init_kton: bool,
	init_staker: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 1,
			validator_pool: false,
			nominate: true,
			validator_count: 2,
			minimum_validator_count: 0,
			slash_defer_duration: 0,
			fair: true,
			num_validators: None,
			invulnerables: vec![],
			init_ring: true,
			init_kton: false,
			init_staker: true,
		}
	}
}

impl ExtBuilder {
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
	pub fn init_ring(mut self, init_ring: bool) -> Self {
		self.init_ring = init_ring;
		self
	}
	pub fn init_kton(mut self, init_kton: bool) -> Self {
		self.init_kton = init_kton;
		self
	}
	pub fn init_staker(mut self, init_staker: bool) -> Self {
		self.init_staker = init_staker;
		self
	}
	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		SLASH_DEFER_DURATION.with(|v| *v.borrow_mut() = self.slash_defer_duration);
	}
	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_consts();
		let mut storage = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

		let num_validators = self.num_validators.unwrap_or(self.validator_count);
		let validators = (0..num_validators)
			.map(|x| ((x + 1) * 10 + 1) as u64)
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
					// This allow us to have a total_payout different from 0.
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
					// This allow us to have a total_payout different from 0.
					(999, 1_000_000_000_000),
				],
			}
			.assimilate_storage(&mut storage);
		}

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
		let _ = GenesisConfig::<Test> {
			stakers: if self.init_staker {
				vec![
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
				]
			} else {
				vec![]
			},
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			slash_reward_fraction: Perbill::from_percent(10),
			// --- custom ---
			payout_fraction: Perbill::from_percent(50),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = pallet_session::GenesisConfig::<Test> {
			keys: validators
				.iter()
				.map(|x| (*x, *x, UintAuthorityId(*x)))
				.collect(),
		}
		.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			let validators = Session::validators();
			SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
		});
		ext
	}
}

pub fn check_exposure_all(era: EraIndex) {
	<ErasStakers<Test>>::iter_prefix(era).for_each(check_exposure)
}

pub fn check_nominator_all(era: EraIndex) {
	<Nominators<Test>>::iter().for_each(|(acc, _)| check_nominator_exposure(era, acc));
}

/// Check for each selected validator: expo.total = Sum(expo.other) + expo.own
pub fn check_exposure(expo: Exposure<AccountId, Balance, Balance>) {
	assert_eq!(
		expo.total_power,
		expo.own_power + expo.others.iter().map(|e| e.power).sum::<Power>(),
		"wrong total exposure {:?}",
		expo,
	);
}

/// Check that for each nominator: slashable_balance > sum(used_balance)
/// Note: we might not consume all of a nominator's balance, but we MUST NOT over spend it.
pub fn check_nominator_exposure(era: EraIndex, stash: AccountId) {
	assert_is_stash(stash);
	let mut sum = 0;
	<ErasStakers<Test>>::iter_prefix(era).for_each(|exposure| {
		exposure
			.others
			.iter()
			.filter(|i| i.who == stash)
			.for_each(|i| sum += i.power)
	});
	let nominator_power = Staking::power_of(&stash);
	// a nominator cannot over-spend.
	assert!(
		nominator_power >= sum,
		"failed: Nominator({}) stake({}) >= sum divided({})",
		stash,
		nominator_power,
		sum,
	);
}

pub fn assert_is_stash(acc: AccountId) {
	assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

pub fn assert_ledger_consistent(stash: AccountId) {
	assert_is_stash(stash);
	let ledger = Staking::ledger(stash - 1).unwrap();

	assert_eq!(ledger.active_ring, ledger.ring_staking_lock.staking_amount);
	assert_eq!(ledger.active_kton, ledger.kton_staking_lock.staking_amount);
}

pub fn bond(acc: AccountId, val: StakingBalanceT<Test>) {
	// a = controller
	// a + 1 = stash
	match val {
		StakingBalance::RingBalance(r) => {
			let _ = Ring::make_free_balance_be(&(acc + 1), r);
		}
		StakingBalance::KtonBalance(k) => {
			let _ = Kton::make_free_balance_be(&(acc + 1), k);
		}
	}
	assert_ok!(Staking::bond(
		Origin::signed(acc + 1),
		acc,
		val,
		RewardDestination::Controller,
		0,
	));
}

pub fn bond_validator(acc: AccountId, val: StakingBalanceT<Test>) {
	bond(acc, val);
	assert_ok!(Staking::validate(
		Origin::signed(acc),
		ValidatorPrefs::default()
	));
}

pub fn bond_nominator(acc: AccountId, val: StakingBalanceT<Test>, target: Vec<AccountId>) {
	bond(acc, val);
	assert_ok!(Staking::nominate(Origin::signed(acc), target));
}

pub fn advance_session() {
	let current_index = Session::current_index();
	start_session(current_index + 1);
}

pub fn start_session(session_index: SessionIndex) {
	for i in Session::current_index()..session_index {
		Staking::on_finalize(System::block_number());
		System::set_block_number((i + 1).into());
		Timestamp::set_timestamp(System::block_number() * 1000);
		Session::on_initialize(System::block_number());
	}

	assert_eq!(Session::current_index(), session_index);
}

pub fn start_era(era_index: EraIndex) {
	start_session((era_index * 3).into());
	assert_eq!(Staking::active_era().unwrap().index, era_index);
}

pub fn current_total_payout_for_duration(era_duration: Moment) -> Balance {
	inflation::compute_total_payout::<Test>(
		era_duration,
		<Module<Test>>::living_time(),
		<Test as Trait>::Cap::get() - Ring::total_issuance(),
		Perbill::from_percent(50),
	)
	.0
}

pub fn reward_all_elected() {
	let rewards = <Test as Trait>::SessionInterface::validators()
		.into_iter()
		.map(|v| (v, 1));

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
	let bonded_eras = BondedEras::get();
	for &(bonded_era, start_session) in bonded_eras.iter() {
		if bonded_era == era {
			Staking::on_offence(offenders, slash_fraction, start_session);
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
	let now = Staking::active_era().unwrap().index;
	on_offence_in_era(offenders, slash_fraction, now)
}

/// Make all validator and nominator request their payment
pub fn make_all_reward_payment(era: EraIndex) {
	let validators_with_reward = <ErasRewardPoints<Test>>::get(era)
		.individual
		.keys()
		.cloned()
		.collect::<Vec<_>>();

	// reward nominators
	let mut nominator_controllers = HashMap::new();
	for validator in Staking::eras_reward_points(era).individual.keys() {
		let validator_exposure = Staking::eras_stakers_clipped(era, validator);
		for (nom_index, nom) in validator_exposure.others.iter().enumerate() {
			if let Some(nom_ctrl) = Staking::bonded(nom.who) {
				nominator_controllers
					.entry(nom_ctrl)
					.or_insert(vec![])
					.push((validator.clone(), nom_index as u32));
			}
		}
	}
	for (nominator_controller, validators_with_nom_index) in nominator_controllers {
		assert_ok!(Staking::payout_nominator(
			Origin::signed(nominator_controller),
			era,
			validators_with_nom_index,
		));
	}

	// reward validators
	for validator_controller in validators_with_reward.iter().filter_map(Staking::bonded) {
		assert_ok!(Staking::payout_validator(
			Origin::signed(validator_controller),
			era
		));
	}
}
