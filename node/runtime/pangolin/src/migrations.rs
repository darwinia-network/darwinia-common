// --- paritytech ---
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
// --- darwinia-network ---
use crate::*;

fn migrate() -> Weight {
	// --- paritytech ---
	use frame_support::migration;

	migration::move_pallet(b"FeeMarket", b"PangoroFeeMarket");

	// 0
	RuntimeBlockWeights::get().max_block
}

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
