// --- paritytech ---
use frame_support::traits::{Get, LockIdentifier};
use sp_runtime::Perbill;
// --- darwinia-network ---
use crate::*;
use darwinia_relay_authority::{Config, EcdsaSign, OpCode};

pub enum MmrRoot {}
impl Get<Option<Hash>> for MmrRoot {
	fn get() -> Option<Hash> {
		HeaderMmr::get_root()
	}
}

// TODO
pub enum MessageRoot {}
impl Get<Hash> for MessageRoot {
	fn get() -> Hash {
		Default::default()
	}
}

frame_support::parameter_types! {
	pub const EcdsaRelayAuthoritiesLockId: LockIdentifier = *b"ethrauth";
	pub const EcdsaRelayAuthoritiesTermDuration: BlockNumber = 3 * MINUTES;
	pub const MaxMembers: u32 = 7;
	pub const OpCodes: (OpCode, OpCode) = (
		[71, 159, 189, 249],
		[180, 188, 244, 151]
	);
	pub const SignThreshold: Perbill = Perbill::from_percent(60);
	pub const SubmitDuration: BlockNumber = 30;
	pub const MaxSchedules: u32 = 10;
}

impl Config for Runtime {
	type AddOrigin = RootOrAtLeastThreeFifth<CouncilCollective>;
	type Currency = Ring;
	type Event = Event;
	type LockId = EcdsaRelayAuthoritiesLockId;
	type MaxMembers = MaxMembers;
	type MaxSchedules = MaxSchedules;
	type MmrRoot = MmrRoot;
	type MmrRootT = Self::Hash;
	type OpCodes = OpCodes;
	type RemoveOrigin = RootOrAtLeastThreeFifth<CouncilCollective>;
	type ResetOrigin = RootOrAtLeastThreeFifth<CouncilCollective>;
	type Sign = EcdsaSign;
	type SignThreshold = SignThreshold;
	type SubmitDuration = SubmitDuration;
	type TermDuration = EcdsaRelayAuthoritiesTermDuration;
	type WeightInfo = ();
}
