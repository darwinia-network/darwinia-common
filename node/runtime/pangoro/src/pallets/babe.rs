// --- paritytech ---
use frame_support::traits::KeyOwnerProofSystem;
use pallet_babe::{AuthorityId, Config, EquivocationHandler, ExternalTrigger};
use sp_core::crypto::KeyTypeId;
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = PANGORO_BLOCKS_PER_SESSION as _;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDurationInEra::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = ExternalTrigger;
	type DisabledValidators = Session;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, AuthorityId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		AuthorityId,
	)>>::IdentificationTuple;
	type HandleEquivocation =
		EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}
