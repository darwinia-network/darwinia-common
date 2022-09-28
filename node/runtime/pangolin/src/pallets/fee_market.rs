pub use pallet_fee_market::{
	Instance1 as WithPangoroFeeMarket, Instance2 as WithPangolinParachainFeeMarket, Instance3 as WithPangolinParachainAlphaFeeMarket
};

// --- core ---
use core::cmp;
// --- substrate ---
use frame_support::traits::LockIdentifier;
use sp_runtime::{Permill, SaturatedConversion};
// --- darwinia ---
use crate::{
	weights::{
		pallet_fee_market_pangolin_parachain_fee_market::WeightInfo as PangolinParachainWeightInfo,
		pallet_fee_market_pangoro_fee_market::WeightInfo as PangoroWeightInfo,
	},
	*,
};
use pallet_fee_market::{BalanceOf, Config, Slasher};

pub struct FeeMarketSlasher;
impl<T, I> Slasher<T, I> for FeeMarketSlasher
where
	T: Config<I>,
	I: 'static,
{
	fn cal_slash_amount(
		collateral_per_order: BalanceOf<T, I>,
		timeout: T::BlockNumber,
	) -> BalanceOf<T, I> {
		const SLASH_PER_BLOCK: Balance = 2 * COIN;

		let collateral_per_order = collateral_per_order.saturated_into::<Balance>();
		let timeout = timeout.saturated_into::<Balance>();
		let slash_value = timeout.saturating_mul(SLASH_PER_BLOCK);

		cmp::min(collateral_per_order, slash_value).saturated_into()
	}
}

frame_support::parameter_types! {
	// Shared configurations.
	pub const MinimumRelayFee: Balance = 15 * COIN;
	pub const CollateralPerOrder: Balance = 50 * COIN;
	pub const Slot: BlockNumber = 300;
	pub const GuardRelayersRewardRatio: Permill = Permill::from_percent(20);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
	pub const AssignedRelayerSlashRatio: Permill = Permill::from_percent(20);
	// Pangoro configurations.
	pub const PangoroFeeMarketLockId: LockIdentifier = *b"da/feelf";
	// Pangolin Parachain configurations.
	pub const PangolinParachainFeeMarketLockId: LockIdentifier = *b"da/feepa";
	// Pangolin Parachain Alpha configurations.
	pub const PangolinParachainAlphaFeeMarketLockId: LockIdentifier = *b"da/feeph";
}

impl Config<WithPangoroFeeMarket> for Runtime {
	type AssignedRelayerSlashRatio = AssignedRelayerSlashRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Currency = Ring;
	type Event = Event;
	type GuardRelayersRewardRatio = GuardRelayersRewardRatio;
	type LockId = PangoroFeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = PangoroWeightInfo<Self>;
}
impl Config<WithPangolinParachainFeeMarket> for Runtime {
	type AssignedRelayerSlashRatio = AssignedRelayerSlashRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Currency = Ring;
	type Event = Event;
	type GuardRelayersRewardRatio = GuardRelayersRewardRatio;
	type LockId = PangolinParachainFeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = PangolinParachainWeightInfo<Self>;
}
impl Config<WithPangolinParachainAlphaFeeMarket> for Runtime {
	type AssignedRelayerSlashRatio = AssignedRelayerSlashRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Currency = Ring;
	type Event = Event;
	type GuardRelayersRewardRatio = GuardRelayersRewardRatio;
	type LockId = PangolinParachainAlphaFeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = PangolinParachainWeightInfo<Self>;
}
