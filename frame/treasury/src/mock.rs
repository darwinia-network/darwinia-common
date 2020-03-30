//! Mock file for treasury.

use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use crate::*;

// --- substrate ---
pub type System = frame_system::Module<Test>;

// --- custom ---
pub type KtonInstance = pallet_balances::Instance1;
pub type RingInstance = pallet_balances::Instance2;
pub type Kton = pallet_balances::Module<Test, KtonInstance>;
pub type Ring = pallet_balances::Module<Test, RingInstance>;
pub type Balance = u64;

// --- current ---
pub type Treasury = Module<Test>;

impl_outer_origin! {
	pub enum Origin for Test  where system = frame_system {}
}

#[derive(Clone, Eq, PartialEq)]
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
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
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

pub struct TenToFourteen;
impl Contains<u64> for TenToFourteen {
	fn contains(n: &u64) -> bool {
		*n >= 10 && *n <= 14
	}
	fn sorted_members() -> Vec<u64> {
		vec![10, 11, 12, 13, 14]
	}
}

parameter_types! {
		pub const ExistentialDeposit: u64 = 1;
}


#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free_ring: Balance,
	pub free_kton: Balance,
	pub reserved_ring: Balance,
	pub reserved_kton: Balance,
}

impl darwinia_support::balance::BalanceInfo<Balance, KtonInstance> for AccountData<Balance> {
	fn free(&self) -> Balance{
		self.free_kton
	}

	fn reserved(&self) -> Balance {
		self.reserved_kton
	}

	fn mutate_free(&mut self, new_free: Balance) {
		self.free_kton = new_free;
	}

	fn mutate_reserved(&mut self, new_reserved: Balance) {
		self.reserved_kton = new_reserved;
	}

	fn usable(&self, reasons: darwinia_support::balance::lock::LockReasons, frozen_balance: darwinia_support::balance::FrozenBalance<Balance>) -> Balance {
		self.free_kton
			.saturating_sub(darwinia_support::balance::FrozenBalance::frozen_for(reasons, frozen_balance))
	}

	fn total(&self) -> Balance {
		self.free_kton.saturating_add(self.reserved_kton)
	}
}

impl darwinia_support::balance::BalanceInfo<Balance, RingInstance> for AccountData<Balance> {
	fn free(&self) -> Balance{
		self.free_ring
	}

	fn reserved(&self) -> Balance {
		self.reserved_ring
	}

	fn mutate_free(&mut self, new_free: Balance) {
		self.free_ring = new_free;
	}

	fn mutate_reserved(&mut self, new_reserved: Balance) {
		self.reserved_ring = new_reserved;
	}

	fn usable(&self, reasons: darwinia_support::balance::lock::LockReasons, frozen_balance: darwinia_support::balance::FrozenBalance<Balance>) -> Balance {
		self.free_ring
			.saturating_sub(darwinia_support::balance::FrozenBalance::frozen_for(reasons, frozen_balance))
	}

	fn total(&self) -> Balance {
		self.free_ring.saturating_add(self.reserved_ring)
	}
}

impl pallet_balances::Trait<KtonInstance> for Test {
	type Balance = u64;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type TryDropOther = ();
}
impl pallet_balances::Trait<RingInstance> for Test {
	type Balance = u64;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type TryDropOther = ();
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: u64 = 1;
	pub const KtonProposalBondMinimum: u64 = 1;
	pub const SpendPeriod: u64 = 2;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TipCountdown: u64 = 1;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: u64 = 1;
	pub const TipReportDepositPerByte: u64 = 1;
}
impl Trait for Test {
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin = frame_system::EnsureRoot<u64>;
	type RejectOrigin = frame_system::EnsureRoot<u64>;
	type Tippers = TenToFourteen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = ();
	type RingProposalRejection = ();
	type KtonProposalRejection = ();
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	pallet_balances::GenesisConfig::<Test, RingInstance> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(0, 100), (1, 98), (2, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	pallet_balances::GenesisConfig::<Test, KtonInstance> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(0, 100), (1, 98), (2, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	GenesisConfig::default()
		.assimilate_storage::<Test>(&mut t)
		.unwrap();

	t.into()
}
