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
	migration::move_pallet(b"BridgeRococoParachains", b"BridgeRococoParachain");
	migration::move_pallet(b"BridgeMoonbaseRelayParachains", b"BridgeMoonbaseRelayParachain");

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

	use bridge_runtime_common::{
		migrate_message_pallet_operation_mode, migrate_pallet_operation_mode,
	};
	let grandpa_modules =
		vec![b"BridgePangoroGrandpa", b"BridgeRococoGrandpa", b"BridgeMoonbaseRelayGrandpa"];
	for module in grandpa_modules {
		migrate_pallet_operation_mode(module);
		// TODO, migrate BestFinalized
	}

	let message_modules = vec![
		b"BridgePangoroMessages",
		b"BridgePangolinParachainMessages",
		b"BridgePangolinParachainAlphaMessages",
	];
	for module in message_modules {
		migrate_message_pallet_operation_mode(module);
	}

	// 0
	RuntimeBlockWeights::get().max_block
}
