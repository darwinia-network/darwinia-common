// --- paritytech ---
#[allow(unused)]
use frame_support::{migration, traits::OnRuntimeUpgrade, weights::Weight};
// --- darwinia-network ---
#[allow(unused)]
use crate::*;

pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		migrate()
	}
}

fn migrate() -> Weight {
	migration::move_pallet(b"BeefyGadget", b"MessageGadget");
	<darwinia_ecdsa_authority::Authorities<Runtime>>::put(
		frame_support::BoundedVec::try_from(vec![array_bytes::hex_into_unchecked(
			"0x68898db1012808808c903f390909c52d9f706749",
		)])
		.unwrap(),
	);

	RuntimeBlockWeights::get().max_block
	// 0
}
