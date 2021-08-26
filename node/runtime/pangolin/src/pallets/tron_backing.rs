// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_tron_backing::Config;

frame_support::parameter_types! {
	pub const TronBackingPalletId: PalletId = PalletId(*b"da/trobk");
}

impl Config for Runtime {
	type PalletId = TronBackingPalletId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type WeightInfo = ();
}
