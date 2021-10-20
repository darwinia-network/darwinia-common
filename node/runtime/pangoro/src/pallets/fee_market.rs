// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;
use sp_runtime::Permill;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const MiniumLockCollateral: Balance = 3000 * COIN;
	pub const MinimumRelayFee: Balance = 15 * COIN;
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const SlotTime: (BlockNumber, BlockNumber, BlockNumber) = (50, 50, 50);

	pub const AssignedRelayersRewardRatio: Permill = Permill::from_percent(60);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type MiniumLockCollateral = MiniumLockCollateral;
	type MinimumRelayFee = MinimumRelayFee;
	type LockId = FeeMarketLockId;
	type SlotTime = SlotTime;

	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Slasher = ();

	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}
