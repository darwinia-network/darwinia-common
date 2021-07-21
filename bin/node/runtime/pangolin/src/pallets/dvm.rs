// --- darwinia ---
use crate::*;
use dvm_ethereum::{Config, IntermediateStateRoot};

frame_support::parameter_types! {
	pub InternalTransactionGasLimit: U256 = U256::from(300_000_000);
}

impl Config for Runtime {
	type Event = Event;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type InternalTransactionGasLimit = InternalTransactionGasLimit;
}
