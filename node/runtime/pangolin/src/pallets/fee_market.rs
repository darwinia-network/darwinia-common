// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const MiniumLockValue: Balance = 2;
	pub const MinimumPrice: u64 = 2;
	pub const PriorRelayersNumber: u64 = 3;
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type Event = Event;
	type MiniumLockValue = MiniumLockValue;
	type MinimumPrice = MinimumPrice;
	type PriorRelayersNumber = PriorRelayersNumber;
	type LockId = FeeMarketLockId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}
