// --- paritytech ---
use pallet_session::{Config, PeriodicSessions};
// --- darwinia-network ---
use crate::*;

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}

frame_support::parameter_types! {
	pub const Period: BlockNumber = pangoro_constants::SESSION_LENGTH as _;
	pub const Offset: BlockNumber = 0;
}

impl Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = PeriodicSessions<Period, Offset>;
	type NextSessionRotation = PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_shift_session_manager::Pallet<Runtime>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = ();
	type WeightInfo = ();
}
