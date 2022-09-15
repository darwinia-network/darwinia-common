// --- paritytech ---
use frame_support::PalletId;
use sp_runtime::Permill;
// --- darwinia-network ---
use crate::{weights::pallet_treasury::WeightInfo, *};
use pallet_treasury::Config;

frame_support::parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 1_000 * COIN;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const MaxApprovals: u32 = 100;
}

// In order to use `Tips`, which bounded by `pallet_treasury::Config` rather
// `pallet_treasury::Config<I>` Still use `DefaultInstance` here instead `Instance1`
impl Config for Runtime {
	type ApproveOrigin = Root;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Ring;
	type Event = Event;
	type MaxApprovals = MaxApprovals;
	type OnSlash = Treasury;
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = RingProposalBondMinimum;
	type RejectOrigin = Root;
	type SpendFunds = ();
	type SpendPeriod = SpendPeriod;
	type WeightInfo = WeightInfo<Self>;
}
