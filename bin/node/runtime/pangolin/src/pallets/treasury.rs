// --- substrate ---
use frame_support::PalletId;
use sp_runtime::{Percent, Permill};
// --- darwinia ---
use crate::*;
use darwinia_treasury::{weights::SubstrateWeight, Config};

frame_support::parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 20 * COIN;
	pub const KtonProposalBondMinimum: Balance = 20 * COIN;
	pub const SpendPeriod: BlockNumber = 3 * MINUTES;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = 3 * MINUTES;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * COIN;
	pub const DataDepositPerByte: Balance = 1 * MILLI;
	pub const BountyDepositBase: Balance = 1 * COIN;
	pub const BountyDepositPayoutDelay: BlockNumber = 3 * MINUTES;
	pub const BountyUpdatePeriod: BlockNumber = 3 * MINUTES;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 2 * COIN;
}
impl Config for Runtime {
	type PalletId = TreasuryPalletId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = EnsureRootOrMoreThanHalfCouncil;
	type Tippers = ElectionsPhragmen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type OnSlashRing = Treasury;
	type OnSlashKton = Treasury;
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type MaximumReasonLength = MaximumReasonLength;
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyValueMinimum = BountyValueMinimum;
	type RingBurnDestination = ();
	type KtonBurnDestination = ();
	type WeightInfo = SubstrateWeight<Runtime>;
}
