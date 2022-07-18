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
	migration::remove_storage_prefix(b"EthereumIssuing", b"MappingFactoryAddress", &[]);
	migration::remove_storage_prefix(b"EthereumIssuing", b"EthereumBackingAddress", &[]);
	migration::remove_storage_prefix(b"EthereumIssuing", b"VerifiedIssuingProof", &[]);
	migration::remove_storage_prefix(b"EthereumIssuing", b"BurnTokenEvents", &[]);

	migration::move_pallet(b"HeaderMMR", b"HeaderMmr");

	migration::move_pallet(b"Instance1DarwiniaRelayAuthorities", b"EcdsaRelayAuthority");

	RuntimeBlockWeights::get().max_block
	// 0
}
