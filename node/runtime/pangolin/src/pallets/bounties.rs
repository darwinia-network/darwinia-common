// --- substrate ---
use sp_runtime::Permill;
// --- darwinia ---
use crate::*;
use pallet_bounties::Config;

frame_support::parameter_types! {
	pub const BountyDepositBase: Balance = 1 * COIN;
	pub const BountyDepositPayoutDelay: BlockNumber = 3 * MINUTES;
	pub const BountyUpdatePeriod: BlockNumber = 3 * MINUTES;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 2 * COIN;
}

impl Config for Runtime {
	type Event = Event;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = ();
}
