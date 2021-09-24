// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;
use sp_runtime::Permill;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const MiniumLockValue: Balance = 2;
	pub const MinimumFee: Balance = 2;
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const SlotTimes: (BlockNumber, BlockNumber, BlockNumber) = (50, 50, 50);

	pub const ForAssignedRelayer: Permill = Permill::from_percent(60);
	pub const ForMessageRelayer: Permill = Permill::from_percent(80);
	pub const ForConfirmRelayer: Permill = Permill::from_percent(20);
	pub const SlashAssignRelayer: Balance = 2;
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type MiniumLockValue = MiniumLockValue;
	type MinimumFee = MinimumFee;
	type LockId = FeeMarketLockId;
	type SlotTimes = SlotTimes;

	type ForAssignedRelayer = ForAssignedRelayer;
	type ForMessageRelayer = ForMessageRelayer;
	type ForConfirmRelayer = ForConfirmRelayer;
	type SlashAssignRelayer = SlashAssignRelayer;

	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}
