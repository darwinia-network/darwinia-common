// --- substrate ---
use frame_support::{traits::FindAuthor, ConsensusEngineId};
use sp_core::{crypto::Public, H160};
// --- darwinia ---
use crate::*;
use dvm_ethereum::{Config, IntermediateStateRoot};

impl Config for Runtime {
	type Event = Event;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
}
