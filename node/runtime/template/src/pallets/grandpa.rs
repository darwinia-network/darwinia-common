pub use pallet_grandpa::AuthorityId as GrandpaId;

// --- paritytech ---
use pallet_grandpa::Config;
// --- darwinia-network ---
use crate::*;

impl Config for Runtime {
	type Call = Call;
	type Event = Event;
	type HandleEquivocation = ();
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerProofSystem = ();
	type MaxAuthorities = MaxAuthorities;
	type WeightInfo = ();
}
