// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;
use sp_runtime::Permill;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const AssignedRelayersNumber: u64 = 1;
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";

	pub const MinimumRelayFee: Balance = 15 * COIN;
	pub const Slot: BlockNumber = 50;
	pub const SlashForEachBlock: Balance = 2 * COIN;
	pub const CollateralEachOrder: Balance = 100 * COIN;

	pub const AssignedRelayersRewardRatio: Permill = Permill::from_percent(60);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type LockId = FeeMarketLockId;

	type AssignedRelayersNumber = AssignedRelayersNumber;
	type MinimumRelayFee = MinimumRelayFee;
	type SlashForEachBlock = SlashForEachBlock;
	type CollateralEachOrder = CollateralEachOrder;
	type Slot = Slot;

	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;

	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}
