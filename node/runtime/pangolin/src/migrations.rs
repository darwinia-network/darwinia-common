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
		Scheduler::pre_migrate_to_v3()?;

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Scheduler::post_migrate_to_v3()?;

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		migrate()
	}
}

fn migrate() -> Weight {
	Scheduler::migrate_v2_to_v3();

	for precompile in PangolinPrecompiles::<Runtime>::used_addresses() {
		EVM::create_account(&precompile, vec![0x60_u8, 0x00, 0x60, 0x00, 0xFD]);
	}

	let storages: &[(&[u8], &[&[u8]])] = &[
		(
			b"EcdsaRelayAuthority",
			&[
				b"Candidates",
				b"Authorities",
				b"NextAuthorities",
				b"NextTerm",
				b"AuthoritiesToSign",
				b"MmrRootsToSignKeys",
				b"MmrRootsToSign",
				b"SubmitDuration",
			],
		),
		(
			// FrameV1 name.
			b"DarwiniaEthereumRelay",
			&[
				b"ConfirmedHeaderParcels",
				b"ConfirmedBlockNumbers",
				b"BestConfirmedBlockNumber",
				b"ConfirmedDepth",
				b"DagsMerkleRoots",
				b"ReceiptVerifyFee",
				b"PendingRelayHeaderParcels",
			],
		),
		(
			// FrameV1 name.
			b"Instance1DarwiniaRelayerGame",
			&[
				b"RelayHeaderParcelToResolve",
				b"Affirmations",
				b"BestConfirmedHeaderId",
				b"RoundCounts",
				b"AffirmTime",
				b"GamesToUpdate",
				b"Stakes",
				b"GameSamplePoints",
			],
		),
		(
			b"EthereumBacking",
			&[
				b"TokenRedeemAddress",
				b"DepositRedeemAddress",
				b"SetAuthoritiesAddress",
				b"RingTokenAddress",
				b"KtonTokenAddress",
				b"RedeemStatus",
				b"LockAssetEvents",
			],
		),
	];
	storages.iter().for_each(|(module, items)| {
		items.iter().for_each(|item| migration::remove_storage_prefix(module, item, &[]))
	});


	// 0
	RuntimeBlockWeights::get().max_block
}
