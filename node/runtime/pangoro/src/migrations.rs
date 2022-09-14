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
	let module = b"Substrate2SubstrateBacking";
	migration::remove_storage_prefix(module, b"SecureLimitedPeriod", &[]);
	migration::remove_storage_prefix(module, b"SecureLimitedRingAmount", &[]);
	migration::remove_storage_prefix(module, b"TransactionInfos", &[]);
	migration::remove_storage_prefix(module, b"RemoteMappingTokenFactoryAccount", &[]);

	// 0
	RuntimeBlockWeights::get().max_block
}
