//! The crate's tests.

// --- std ---
use std::cell::RefCell;
// --- crates ---
use codec::Encode;
// --- substrate ---
use frame_support::{
	assert_noop, assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin,
	ord_parameter_types, parameter_types,
	traits::{Contains, Filter, OnInitialize},
	weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BadOrigin, BlakeTwo256, IdentityLookup},
	Perbill,
};
// --- darwinia ---
use super::*;
use darwinia_balances::Error as BalancesError;

mod cancellation;
mod decoders;
mod delegation;
mod external_proposing;
mod fast_tracking;
mod lock_voting;
mod preimage;
mod public_proposals;
mod scheduling;
mod voting;

type BlockNumber = u64;
type Balance = u64;

type RingInstance = darwinia_balances::Instance0;
type KtonInstance = darwinia_balances::Instance1;

const AYE: Vote = Vote {
	aye: true,
	conviction: Conviction::None,
};
const NAY: Vote = Vote {
	aye: false,
	conviction: Conviction::None,
};
const BIG_AYE: Vote = Vote {
	aye: true,
	conviction: Conviction::Locked1x,
};
const BIG_NAY: Vote = Vote {
	aye: false,
	conviction: Conviction::Locked1x,
};

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		frame_system::System,
		pallet_balances::Balances,
		democracy::Democracy,
	}
}

mod democracy {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum Event for Test {
		system<T>,
		darwinia_balances Instance0<T>,
		pallet_scheduler<T>,
		democracy<T>,
	}
}

darwinia_support::impl_account_data! {
	struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
		// other data
	}
}

// Test that a fitlered call can be dispatched.
pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
	fn filter(call: &Call) -> bool {
		!matches!(
			call,
			&Call::Balances(darwinia_balances::Call::set_balance(..))
		)
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1_000_000;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = BaseFilter;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}
parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
}
impl pallet_scheduler::Trait for Test {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<u64>;
	type WeightInfo = ();
}
parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = ();
	type WeightInfo = ();
}
parameter_types! {
	pub const LaunchPeriod: u64 = 2;
	pub const VotingPeriod: u64 = 2;
	pub const FastTrackVotingPeriod: u64 = 2;
	pub const MinimumDeposit: u64 = 1;
	pub const EnactmentPeriod: u64 = 2;
	pub const CooloffPeriod: u64 = 2;
	pub const MaxVotes: u32 = 100;
}
ord_parameter_types! {
	pub const One: u64 = 1;
	pub const Two: u64 = 2;
	pub const Three: u64 = 3;
	pub const Four: u64 = 4;
	pub const Five: u64 = 5;
	pub const Six: u64 = 6;
}
pub struct OneToFive;
impl Contains<u64> for OneToFive {
	fn sorted_members() -> Vec<u64> {
		vec![1, 2, 3, 4, 5]
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u64) {}
}
thread_local! {
	static PREIMAGE_BYTE_DEPOSIT: RefCell<u64> = RefCell::new(0);
	static INSTANT_ALLOWED: RefCell<bool> = RefCell::new(false);
}
pub struct PreimageByteDeposit;
impl Get<u64> for PreimageByteDeposit {
	fn get() -> u64 {
		PREIMAGE_BYTE_DEPOSIT.with(|v| *v.borrow())
	}
}
pub struct InstantAllowed;
impl Get<bool> for InstantAllowed {
	fn get() -> bool {
		INSTANT_ALLOWED.with(|v| *v.borrow())
	}
}
impl super::Trait for Test {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	type ExternalOrigin = EnsureSignedBy<Two, u64>;
	type ExternalMajorityOrigin = EnsureSignedBy<Three, u64>;
	type ExternalDefaultOrigin = EnsureSignedBy<One, u64>;
	type FastTrackOrigin = EnsureSignedBy<Five, u64>;
	type CancellationOrigin = EnsureSignedBy<Four, u64>;
	type VetoOrigin = EnsureSignedBy<OneToFive, u64>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type Slash = ();
	type InstantOrigin = EnsureSignedBy<Six, u64>;
	type InstantAllowed = InstantAllowed;
	type Scheduler = Scheduler;
	type MaxVotes = MaxVotes;
	type OperationalPreimageOrigin = EnsureSignedBy<Six, u64>;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	GenesisConfig::default().assimilate_storage(&mut t).unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Execute the function two times, with `true` and with `false`.
pub fn new_test_ext_execute_with_cond(execute: impl FnOnce(bool) -> () + Clone) {
	new_test_ext().execute_with(|| (execute.clone())(false));
	new_test_ext().execute_with(|| execute(true));
}

type System = frame_system::Module<Test>;
type Balances = darwinia_balances::Module<Test, RingInstance>;
type Scheduler = pallet_scheduler::Module<Test>;
type Democracy = Module<Test>;

#[test]
fn params_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Democracy::referendum_count(), 0);
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(Balances::total_issuance(), 210);
	});
}

fn set_balance_proposal(value: u64) -> Vec<u8> {
	Call::Balances(darwinia_balances::Call::set_balance(42, value, 0)).encode()
}

#[test]
fn set_balance_proposal_is_correctly_filtered_out() {
	for i in 0..10 {
		let call = Call::decode(&mut &set_balance_proposal(i)[..]).unwrap();
		assert!(!<Test as frame_system::Trait>::BaseCallFilter::filter(
			&call
		));
	}
}

fn set_balance_proposal_hash(value: u64) -> H256 {
	BlakeTwo256::hash(&set_balance_proposal(value)[..])
}

fn set_balance_proposal_hash_and_note(value: u64) -> H256 {
	let p = set_balance_proposal(value);
	let h = BlakeTwo256::hash(&p[..]);
	match Democracy::note_preimage(Origin::signed(6), p) {
		Ok(_) => (),
		Err(x) if x == Error::<Test>::DuplicatePreimage.into() => (),
		Err(x) => panic!(x),
	}
	h
}

fn propose_set_balance(who: u64, value: u64, delay: u64) -> DispatchResult {
	Democracy::propose(Origin::signed(who), set_balance_proposal_hash(value), delay)
}

fn propose_set_balance_and_note(who: u64, value: u64, delay: u64) -> DispatchResult {
	Democracy::propose(
		Origin::signed(who),
		set_balance_proposal_hash_and_note(value),
		delay,
	)
}

fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
	assert!(Democracy::begin_block(System::block_number()).is_ok());
}

fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

fn begin_referendum() -> ReferendumIndex {
	System::set_block_number(0);
	assert_ok!(propose_set_balance_and_note(1, 2, 1));
	fast_forward_to(2);
	0
}

fn aye(who: u64) -> AccountVote<u64> {
	AccountVote::Standard {
		vote: AYE,
		balance: Balances::free_balance(&who),
	}
}

fn nay(who: u64) -> AccountVote<u64> {
	AccountVote::Standard {
		vote: NAY,
		balance: Balances::free_balance(&who),
	}
}

fn big_aye(who: u64) -> AccountVote<u64> {
	AccountVote::Standard {
		vote: BIG_AYE,
		balance: Balances::free_balance(&who),
	}
}

fn big_nay(who: u64) -> AccountVote<u64> {
	AccountVote::Standard {
		vote: BIG_NAY,
		balance: Balances::free_balance(&who),
	}
}

fn tally(r: ReferendumIndex) -> Tally<u64> {
	Democracy::referendum_status(r).unwrap().tally
}
