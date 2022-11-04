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
	migration::remove_storage_prefix(b"TransactionPause", b"PausedTransactions", &[]);

	let grandpa = b"BridgePangolinGrandpa";

	bridge_runtime_common::migrate_pallet_operation_mode(grandpa);

	if let Some(hash) = migration::take_storage_value::<<Pangolin as bp_runtime::Chain>::Hash>(
		grandpa,
		b"BestFinalized",
		&[],
	) {
		if let Some(header) =
			<pallet_bridge_grandpa::ImportedHeaders<Runtime, WithPangolinGrandpa>>::get(hash)
		{
			<pallet_bridge_grandpa::BestFinalized<Runtime, WithPangolinGrandpa>>::put((
				header.number,
				hash,
			));
		}
	}

	bridge_runtime_common::migrate_message_pallet_operation_mode(b"BridgePangolinMessages");

	// 0
	RuntimeBlockWeights::get().max_block
}
