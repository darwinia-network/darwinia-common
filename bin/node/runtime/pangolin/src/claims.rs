// --- substrate ---
use sp_runtime::ModuleId;
// --- darwinia ---
use crate::*;
use darwinia_claims::Config;

frame_support::parameter_types! {
	pub const ClaimsModuleId: ModuleId = ModuleId(*b"da/claim");
	pub Prefix: &'static [u8] = b"Pay PRINGs to the Pangolin account:";
}
impl Config for Runtime {
	type Event = Event;
	type ModuleId = ClaimsModuleId;
	type Prefix = Prefix;
	type RingCurrency = Ring;
	type MoveClaimOrigin = EnsureRootOrMoreThanHalfCouncil;
}
