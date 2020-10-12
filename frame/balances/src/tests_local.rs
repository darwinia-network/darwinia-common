//! Test utilities

mod balances {
	pub use crate::{Event, Instance0, Instance1};
}

// --- std ---
use std::cell::RefCell;
// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	impl_outer_event, impl_outer_origin, parameter_types,
	traits::{Get, StorageMapShim},
	weights::{DispatchInfo, IdentityFee, Weight},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill, RuntimeDebug};
// --- darwinia ---
use crate::{tests::*, *};

type Balance = u64;

type RingInstance = Instance0;
type RingError = Error<Test, RingInstance>;
type Ring = Module<Test, RingInstance>;

type KtonInstance = Instance1;
type _KtonError = Error<Test, KtonInstance>;
type Kton = Module<Test, KtonInstance>;

thread_local! {
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
}

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum Event for Test {
		system<T>,
		balances Instance0<T>,
		balances Instance1<T>,
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

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
	fn get() -> Balance {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: Balance = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = CallWithDispatchInfo;
	type Index = Balance;
	type BlockNumber = Balance;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = Balance;
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
	type OnKilledAccount = Ring;
	type SystemWeightInfo = ();
}
parameter_types! {
	pub const TransactionByteFee: Balance = 1;
}
impl pallet_transaction_payment::Trait for Test {
	type Currency = Ring;
	type OnTransactionPayment = ();
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<u64>;
	type FeeMultiplierUpdate = ();
}
impl Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = StorageMapShim<
		Account<Test, RingInstance>,
		frame_system::CallOnCreatedAccount<Test>,
		frame_system::CallKillAccount<Test>,
		Balance,
		AccountData<Balance>,
	>;
	type MaxLocks = ();
	type OtherCurrencies = (Kton,);
	type WeightInfo = ();
}
impl Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = StorageMapShim<
		Account<Test, KtonInstance>,
		frame_system::CallOnCreatedAccount<Test>,
		frame_system::CallKillAccount<Test>,
		Balance,
		AccountData<Balance>,
	>;
	type MaxLocks = ();
	type OtherCurrencies = (Ring,);
	type WeightInfo = ();
}

pub struct ExtBuilder {
	existential_deposit: Balance,
	monied: bool,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 1,
			monied: false,
		}
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
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		GenesisConfig::<Test, RingInstance> {
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
	<ExtBuilder>::default()
		.existential_deposit(0)
		.build()
		.execute_with(|| {
			assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

			assert_eq!(
				events(),
				[
					Event::system(frame_system::RawEvent::NewAccount(1)),
					Event::balances_Instance0(RawEvent::Endowed(1, 100)),
					Event::balances_Instance0(RawEvent::BalanceSet(1, 100, 0)),
				]
			);

			let _ = Ring::slash(&1, 99);

			// no events
			assert_eq!(events(), []);

			assert_ok!(System::suicide(Origin::signed(1)));

			assert_eq!(
				events(),
				[
					Event::balances_Instance0(RawEvent::DustLost(1, 1)),
					Event::system(frame_system::RawEvent::KilledAccount(1))
				]
			);
		});
}

#[test]
fn dust_collector_should_work() {
	type AnotherBalance = Module<Test, Instance1>;

	<ExtBuilder>::default()
		.existential_deposit(100)
		.build()
		.execute_with(|| {
			assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

			assert_eq!(
				events(),
				[
					Event::system(system::RawEvent::NewAccount(1)),
					Event::balances_Instance0(RawEvent::Endowed(1, 100)),
					Event::balances_Instance0(RawEvent::BalanceSet(1, 100, 0)),
				]
			);

			let _ = Ring::slash(&1, 1);

			assert_eq!(
				events(),
				[
					Event::balances_Instance0(RawEvent::DustLost(1, 99)),
					Event::system(system::RawEvent::KilledAccount(1))
				]
			);

			// ---

			assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));
			assert_ok!(AnotherBalance::set_balance(
				RawOrigin::Root.into(),
				1,
				100,
				0
			));

			assert_eq!(
				events(),
				[
					Event::system(system::RawEvent::NewAccount(1)),
					Event::balances_Instance0(RawEvent::Endowed(1, 100)),
					Event::balances_Instance0(RawEvent::BalanceSet(1, 100, 0)),
					Event::system(system::RawEvent::NewAccount(1)),
					Event::balances_Instance1(RawEvent::Endowed(1, 100)),
					Event::balances_Instance1(RawEvent::BalanceSet(1, 100, 0)),
				]
			);

			let _ = Ring::slash(&1, 1);

			assert_eq!(events(), []);

			let _ = AnotherBalance::slash(&1, 1);

			assert_eq!(
				events(),
				[
					Event::balances_Instance1(RawEvent::DustLost(1, 99)),
					Event::balances_Instance0(RawEvent::DustLost(1, 99)),
					Event::system(system::RawEvent::KilledAccount(1)),
				]
			);
		});
}
