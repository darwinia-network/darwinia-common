// --- substrate ---
use frame_support::traits::KeyOwnerProofSystem;
use pallet_babe::{AuthorityId, Config, EquivocationHandler, ExternalTrigger};
use sp_core::crypto::KeyTypeId;
// --- darwinia ---
use crate::*;

frame_support::parameter_types! {
	pub const EpochDuration: u64 = BLOCKS_PER_SESSION as _;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}
impl Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = ExternalTrigger;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, AuthorityId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		AuthorityId,
	)>>::IdentificationTuple;
	type HandleEquivocation = EquivocationHandler<Self::KeyOwnerIdentification, Offences>;
	type WeightInfo = ();
}
