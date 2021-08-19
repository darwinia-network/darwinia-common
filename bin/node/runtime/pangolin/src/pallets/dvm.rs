// --- darwinia ---
use crate::*;
use dvm_ethereum::{Config, IntermediateStateRoot};

frame_support::parameter_types! {
	pub const PalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl Config for Runtime {
	type PalletId = PalletId;
	type Event = Event;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
}
