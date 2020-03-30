//! Mock file for eth-backing.

use std::cell::RefCell;

use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use hex_literal::hex;
use sp_core::{crypto::key_types, H256};
use sp_io;
use sp_runtime::{
	testing::{Header, UintAuthorityId},
	traits::{IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
	{KeyTypeId, MultiSignature, Perbill},
};
use sp_staking::SessionIndex;

use pallet_staking::{EraIndex, Exposure, ExposureOf};

use crate::*;

// --- custom ---
pub type KtonInstance = pallet_balances::Instance1;
pub type RingInstance = pallet_balances::Instance2;
pub type Kton = pallet_balances::Module<Test, KtonInstance>;
pub type Ring = pallet_balances::Module<Test, RingInstance>;
pub type Staking = pallet_staking::Module<Test>;
pub type EthRelay = darwinia_eth_relay::Module<Test>;

// --- current ---
pub type EthBacking = Module<Test>;

// --- substrate ---
type System = frame_system::Module<Test>;
type Timestamp = pallet_timestamp::Module<Test>;

type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
type Signature = MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.

type Balance = u128;
type BlockNumber = u64;
type Power = u32;

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const CAP: Balance = 10_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free_ring: Balance,
	pub free_kton: Balance,
	pub reserved_ring: Balance,
	pub reserved_kton: Balance,
}

impl darwinia_support::balance::BalanceInfo<Balance, KtonInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_kton
	}

	fn reserved(&self) -> Balance {
		self.reserved_kton
	}

	fn set_free(&mut self, new_free: Balance) {
		self.free_kton = new_free;
	}

	fn set_reserved(&mut self, new_reserved: Balance) {
		self.reserved_kton = new_reserved;
	}

	fn usable(
		&self,
		reasons: darwinia_support::balance::lock::LockReasons,
		frozen_balance: darwinia_support::balance::FrozenBalance<Balance>,
	) -> Balance {
		self.free_kton
			.saturating_sub(frozen_balance.frozen_for(reasons))
	}

	fn total(&self) -> Balance {
		self.free_kton.saturating_add(self.reserved_kton)
	}
}

impl darwinia_support::balance::BalanceInfo<Balance, RingInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_ring
	}

	fn reserved(&self) -> Balance {
		self.reserved_ring
	}

	fn set_free(&mut self, new_free: Balance) {
		self.free_ring = new_free;
	}

	fn set_reserved(&mut self, new_reserved: Balance) {
		self.reserved_ring = new_reserved;
	}

	fn usable(
		&self,
		reasons: darwinia_support::balance::lock::LockReasons,
		frozen_balance: darwinia_support::balance::FrozenBalance<Balance>,
	) -> Balance {
		self.free_ring
			.saturating_sub(frozen_balance.frozen_for(reasons))
	}

	fn total(&self) -> Balance {
		self.free_ring.saturating_add(self.reserved_ring)
	}
}

thread_local! {
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
	static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId, Ks)],
		_queued_validators: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_validator_index: usize) {}
}

impl_outer_origin! {
	pub enum Origin for Test  where system = system {}
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
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
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
	type ValidatorIdOf = ();
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = TestSessionHandler;
	type Keys = UintAuthorityId;
	type DisabledValidatorsThreshold = ();
}

impl pallet_session::historical::Trait for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

// --- custom ---

parameter_types! {
	pub const EthRopsten: u64 = 1;
}
impl darwinia_eth_relay::Trait for Test {
	type Event = ();
	type EthNetwork = EthRopsten;
}

impl pallet_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type TryDropOther = ();
}
impl pallet_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type TryDropOther = ();
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 3;
	pub const BondingDurationInEra: EraIndex = 3;
	// assume 60 blocks per session
	pub const BondingDurationInBlockNumber: BlockNumber = 3 * 3 * 60;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;

	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl pallet_staking::Trait for Test {
	type Time = Timestamp;
	type Event = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = ();
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

impl Trait for Test {
	type Event = ();
	type Time = Timestamp;
	type DetermineAccountId = AccountIdDeterminator<Test>;
	type EthRelay = EthRelay;
	type OnDepositRedeem = Staking;
	type Ring = Ring;
	type RingReward = ();
	type Kton = Kton;
	type KtonReward = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
	fn default() -> Self {
		Self
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		GenesisConfig::<Test> {
			ring_redeem_address: hex!["dbc888d701167cbfb86486c516aafbefc3a4de6e"].into(),
			kton_redeem_address: hex!["dbc888d701167cbfb86486c516aafbefc3a4de6e"].into(),
			deposit_redeem_address: hex!["6ef538314829efa8386fc43386cb13b4e0a67d1e"].into(),
			ring_locked: 20000000000000,
			kton_locked: 5000000000000,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
