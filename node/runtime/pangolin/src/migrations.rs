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
	use codec::Decode;
	use frame_support::{storage::migration::storage_key_iter, Blake2_128Concat, StorageHasher};
	// darwinia-network
	use bp_parachains::{BestParaHeadHash, ParaInfo};
	use bp_polkadot_core::parachains::{ParaHash, ParaId};
	use bp_runtime::BasicOperatingMode;
	use bridge_runtime_common::{
		migrate_message_pallet_operation_mode, migrate_pallet_operation_mode,
		put_pallet_operation_mode,
	};
	use pallet_bridge_parachains::RelayBlockNumber;
	use sp_api::HeaderT;

	migration::move_pallet(b"BridgeRococoParachains", b"BridgeRococoParachain");
	migration::move_pallet(b"BridgeMoonbaseRelayParachains", b"BridgeMoonbaseRelayParachain");

	// Removed pallets
	let removed_items: &[(&[u8], &[&[u8]])] = &[
		(
			b"ToPangolinParachainBacking",
			&[b"SecureLimitedPeriod", b"TransactionInfos", b"RemoteMappingTokenFactoryAccount"],
		),
		(b"TransactionPause", &[b"PausedTransactions"]),
	];
	let hash = &[];

	removed_items.iter().for_each(|(module, items)| {
		items.iter().for_each(|item| migration::remove_storage_prefix(module, item, hash));
	});

	// Grandpa pallets
	let grandpa_modules: Vec<&[u8]> =
		vec![b"BridgePangoroGrandpa", b"BridgeRococoGrandpa", b"BridgeMoonbaseRelayGrandpa"];
	for module in grandpa_modules {
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
	}

	//  Message pallets
	let message_modules: Vec<&[u8]> = vec![
		b"BridgePangoroMessages",
		b"BridgePangolinParachainMessages",
		b"BridgePangolinParachainAlphaMessages",
	];
	for module in message_modules {
		migrate_message_pallet_operation_mode(module);
	}

	// Parachains pallets
	#[derive(Encode, Decode)]
	pub struct BestParaHead {
		pub at_relay_block_number: RelayBlockNumber,
		pub head_hash: ParaHash,
		pub next_imported_hash_position: u32,
	}
	let old_item = b"BestParaHeads";
	let new_item = b"ParasInfo";

	let parachains_modules: Vec<&[u8]> =
		vec![b"BridgeRococoParachain", b"BridgeMoonbaseRelayParachain"];
	for module in parachains_modules {
		for (para_id, best_para_head) in
			storage_key_iter::<ParaId, BestParaHead, Blake2_128Concat>(module, old_item).drain()
		{
			let para_info = ParaInfo {
				best_head_hash: BestParaHeadHash {
					at_relay_block_number: best_para_head.at_relay_block_number,
					head_hash: best_para_head.head_hash,
				},
				next_imported_hash_position: best_para_head.next_imported_hash_position,
			};

			migration::put_storage_value(
				module,
				new_item,
				&Blake2_128Concat::hash(&para_id.encode()),
				para_info,
			);
		}

		put_pallet_operation_mode(module, BasicOperatingMode::Normal);
	}

	// 0
	RuntimeBlockWeights::get().max_block
}
