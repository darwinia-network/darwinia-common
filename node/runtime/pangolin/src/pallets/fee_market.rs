pub use darwinia_fee_market::{
	Instance1 as WithPangoroFeeMarket, Instance2 as WithPangolinParachainFeeMarket,
};

// --- core ---
use core::cmp;
// --- substrate ---
use frame_support::{traits::LockIdentifier, PalletId};
use sp_runtime::{traits::UniqueSaturatedInto, Permill};
// --- darwinia ---
use crate::*;
use darwinia_fee_market::{Config, RingBalance, Slasher};

pub struct FeeMarketSlasher;
impl<T: Config<I>, I: 'static> Slasher<T, I> for FeeMarketSlasher {
	fn slash(locked_collateral: RingBalance<T, I>, timeout: T::BlockNumber) -> RingBalance<T, I> {
		let slash_each_block = 2 * COIN;
		let slash_value = UniqueSaturatedInto::<Balance>::unique_saturated_into(timeout)
			.saturating_mul(UniqueSaturatedInto::<Balance>::unique_saturated_into(
				slash_each_block,
			))
			.unique_saturated_into();

		cmp::min(locked_collateral, slash_value)
	}
}

frame_support::parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");

	pub const WithPangoroFeeMarketId: PalletId = PalletId(*b"da/feemk");
	pub const WithParachainFeeMarketId: PalletId = PalletId(*b"da/feepa");

	pub const WithPangoroFeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const WithParachainFeeMarketLockId: LockIdentifier = *b"da/feepa";

	pub const MinimumRelayFee: Balance = 15 * COIN;
	pub const CollateralPerOrder: Balance = 50 * COIN;
	pub const Slot: BlockNumber = 600;

	pub const AssignedRelayersRewardRatio: Permill = Permill::from_percent(60);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
}

impl Config<WithPangoroFeeMarket> for Runtime {
	type TreasuryPalletId = TreasuryPalletId;

	type PalletId = WithPangoroFeeMarketId;
	type LockId = WithPangoroFeeMarketLockId;

	type MinimumRelayFee = MinimumRelayFee;
	type CollateralPerOrder = CollateralPerOrder;
	type Slot = Slot;

	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;

	type Slasher = FeeMarketSlasher;
	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}

impl Config<WithPangolinParachainFeeMarket> for Runtime {
	type TreasuryPalletId = TreasuryPalletId;

	type PalletId = WithParachainFeeMarketId;
	type LockId = WithParachainFeeMarketLockId;

	type MinimumRelayFee = MinimumRelayFee;
	type CollateralPerOrder = CollateralPerOrder;
	type Slot = Slot;

	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;

	type Slasher = FeeMarketSlasher;
	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}
