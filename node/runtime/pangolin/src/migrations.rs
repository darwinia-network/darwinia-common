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

	removed_items.iter().for_each(|(module, items)| {
		items.iter().for_each(|item| migration::remove_storage_prefix(module, item, &[]));
	});

	let grandpas: &[&[u8]] =
		&[b"BridgePangoroGrandpa", b"BridgeRococoGrandpa", b"BridgeMoonbaseRelayGrandpa"];

	grandpas
		.iter()
		.for_each(|grandpa| bridge_runtime_common::migrate_pallet_operation_mode(grandpa));

	macro_rules! migrate_best_finalized {
		($c:ty, $i:ty, $n:expr) => {
			if let Some(hash) = migration::take_storage_value::<<$c as bp_runtime::Chain>::Hash>(
				$n,
				b"BestFinalized",
				&[],
			) {
				if let Some(header) =
					<pallet_bridge_grandpa::ImportedHeaders<Runtime, $i>>::get(hash)
				{
					<pallet_bridge_grandpa::BestFinalized<Runtime, $i>>::put((header.number, hash));
				}
			}
		};
	}

	migrate_best_finalized!(Pangoro, WithPangoroGrandpa, b"BridgePangoroGrandpa");
	migrate_best_finalized!(Rococo, WithRococoGrandpa, b"BridgeRococoGrandpa");
	migrate_best_finalized!(MoonbaseRelay, WithMoonbaseRelayGrandpa, b"BridgeMoonbaseRelayGrandpa");

	let messages: &[&[u8]] = &[
		b"BridgePangoroMessages",
		b"BridgePangolinParachainMessages",
		b"BridgePangolinParachainAlphaMessages",
	];

	messages
		.iter()
		.for_each(|message| bridge_runtime_common::migrate_message_pallet_operation_mode(message));

	let parachains_modules: &[&[u8]] = &[b"BridgeRococoParachain", b"BridgeMoonbaseRelayParachain"];

	parachains_modules.iter().for_each(|module| {
		bridge_runtime_common::put_pallet_operation_mode(
			module,
			bp_runtime::BasicOperatingMode::Normal,
		);
	});

	// 0
	RuntimeBlockWeights::get().max_block
}
