// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use darwinia_ethereum::{Config, IntermediateStateRoot};

frame_support::parameter_types! {
	pub const DvmPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl Config for Runtime {
	type Event = Event;
	// type KtonCurrency = Kton;
	type PalletId = DvmPalletId;
	// type RingCurrency = Ring;
	type StateRoot = IntermediateStateRoot;
}
