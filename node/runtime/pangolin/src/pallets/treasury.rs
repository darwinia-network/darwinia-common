// --- substrate ---
use frame_support::PalletId;
use sp_runtime::Permill;
// --- darwinia ---
use crate::*;
use pallet_treasury::{Config, Instance2 as KtonTreasuryInstance};

frame_support::parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 20 * COIN;
	pub const SpendPeriod: BlockNumber = 3 * MINUTES;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxApprovals: u32 = 100;
}

// In order to use `Tips`, which bounded by `pallet_treasury::Config` rather `pallet_treasury::Config<I>`
// Still use `DefaultInstance` here instead `Instance1`
impl Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Ring;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = EnsureRootOrMoreThanHalfCouncil;
	type Event = Event;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = Society;
	type WeightInfo = ();
	type SpendFunds = Bounties;
	type MaxApprovals = MaxApprovals;
}
impl Config<KtonTreasuryInstance> for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Kton;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = EnsureRootOrMoreThanHalfCouncil;
	type Event = Event;
	type OnSlash = KtonTreasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type WeightInfo = ();
	type SpendFunds = ();
	type MaxApprovals = MaxApprovals;
}
