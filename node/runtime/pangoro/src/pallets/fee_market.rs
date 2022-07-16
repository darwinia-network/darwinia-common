pub use pallet_fee_market::Instance1 as WithPangolinFeeMarket;

// --- core ---
use core::cmp;
// --- substrate ---
use frame_support::traits::LockIdentifier;
use sp_runtime::{traits::UniqueSaturatedInto, Permill};
// --- darwinia ---
use crate::{weights::pallet_fee_market::WeightInfo, *};
use pallet_fee_market::{BalanceOf, Config, Slasher};

pub struct FeeMarketSlasher;
impl<T, I> Slasher<T, I> for FeeMarketSlasher
where
	T: Config<I>,
	I: 'static,
{
	fn slash(locked_collateral: BalanceOf<T, I>, timeout: T::BlockNumber) -> BalanceOf<T, I> {
		let slash_each_block = 2 * COIN;
		let slash_value = UniqueSaturatedInto::<Balance>::unique_saturated_into(timeout)
			.saturating_mul(UniqueSaturatedInto::<Balance>::unique_saturated_into(slash_each_block))
			.unique_saturated_into();

		cmp::min(locked_collateral, slash_value)
	}
}

frame_support::parameter_types! {
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";

	pub const MinimumRelayFee: Balance = 15 * COIN;
	pub const CollateralPerOrder: Balance = 50 * COIN;
	pub const Slot: BlockNumber = 300;

	pub const AssignedRelayersRewardRatio: Permill = Permill::from_percent(60);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
}

impl Config<WithPangolinFeeMarket> for Runtime {
	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Currency = Ring;
	type Event = Event;
	type LockId = FeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = WeightInfo<Runtime>;
}
