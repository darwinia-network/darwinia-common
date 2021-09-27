// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;
use sp_runtime::Permill;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const MiniumLockCollateral: Balance = 3000;
	pub const MinimumRelayFee: Balance = 15;
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const SlotTimes: (BlockNumber, BlockNumber, BlockNumber) = (20, 20, 20);

	pub const ForAssignedRelayers: Permill = Permill::from_percent(60);
	pub const ForMessageRelayer: Permill = Permill::from_percent(80);
	pub const ForConfirmRelayer: Permill = Permill::from_percent(20);
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type MiniumLockCollateral = MiniumLockCollateral;
	type MinimumRelayFee = MinimumRelayFee;
	type LockId = FeeMarketLockId;
	type SlotTimes = SlotTimes;

	type ForAssignedRelayers = ForAssignedRelayers;
	type ForMessageRelayer = ForMessageRelayer;
	type ForConfirmRelayer = ForConfirmRelayer;
	type AssignedRelayersAbsentSlash = ();

	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}
