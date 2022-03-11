use pallet_bridge_grandpa::Instance2 as RococoGrandpaInstance;


use crate::*;

pub type WithRococoParachainsInstance = ();

impl pallet_bridge_parachains::Config<WithRococoParachainsInstance> for Runtime {
    type BridgesGrandpaPalletInstance = RococoGrandpaInstance;
    type HeadsToKeep = HeadersToKeep;
}