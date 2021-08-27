// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_fee_market::Config;

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
}

impl Config for Runtime {
	type PalletId = FeeMarketPalletId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}
