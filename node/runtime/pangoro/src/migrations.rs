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
	// paritytech
	use sp_api::HeaderT;
	// darwinia-network
	use bridge_runtime_common::{
		migrate_message_pallet_operation_mode, migrate_pallet_operation_mode,
	};
	use frame_support::StorageHasher;

	// Removed pallets
	migration::remove_storage_prefix(b"TransactionPause", b"PausedTransactions", &[]);

	// Grandpa
	let module = b"BridgePangolinGrandpa";
	let item = b"BestFinalized";
	let hash = &[];
	if let Some(block_hash) = migration::take_storage_value::<Hash>(module, item, hash) {
		let imported_header_item = b"ImportedHeaders";
		let imported_header_hash = frame_support::Identity::hash(&block_hash.encode());
		if let Some(header) = migration::get_storage_value::<Header>(
			module,
			imported_header_item,
			&imported_header_hash,
		) {
			migration::put_storage_value(module, item, hash, (header.number(), header.hash()));
		}
	}
	migrate_pallet_operation_mode(module);

	// Message
	migrate_message_pallet_operation_mode(b"BridgePangolinMessages");

	// 0
	RuntimeBlockWeights::get().max_block
}
