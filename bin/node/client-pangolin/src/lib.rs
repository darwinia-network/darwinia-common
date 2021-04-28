use std::time::Duration;

// -- s2s bridger ---
use codec::Encode;
use sp_core::{Pair, storage::StorageKey};
use sp_runtime::{
	generic::SignedPayload,
	traits::IdentifyAccount,
};

use relay_substrate_client::{Chain, ChainBase, ChainWithBalances, TransactionSignScheme};

// --- darwinia ---
use pangolin_runtime::*;
use frame_support::traits::Len;

/// Pangolin header id.
pub type HeaderId = relay_utils::HeaderId<drml_primitives::Hash, drml_primitives::BlockNumber>;


/// Rialto signing params.
pub type SigningParams = sp_core::sr25519::Pair;

/// Rialto header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<drml_primitives::Header>;


/// Millau chain definition.
#[derive(Debug, Clone, Copy)]
pub struct PangolinRelayChain;

impl ChainBase for PangolinRelayChain {
	type BlockNumber = drml_primitives::BlockNumber;
	type Hash = drml_primitives::Hash;
	type Hasher = drml_primitives::Hashing;
	type Header = drml_primitives::Header;
}

impl Chain for PangolinRelayChain {
	const NAME: &'static str = "Pangolin";
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(5);

	type AccountId = drml_primitives::AccountId;
	type Index = drml_primitives::Nonce;
	type SignedBlock = pangolin_runtime::SignedBlock;
	type Call = pangolin_runtime::Call;
}

impl ChainWithBalances for PangolinRelayChain {
	type NativeBalance = drml_primitives::Balance;

	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		use frame_support::storage::generator::StorageMap;
		StorageKey(frame_system::Account::<pangolin_runtime::Runtime>::storage_map_final_key(
			account_id,
		))
	}
}


impl TransactionSignScheme for PangolinRelayChain {
	type Chain = PangolinRelayChain;
	type AccountKeyPair = sp_core::sr25519::Pair;
	type SignedTransaction = pangolin_runtime::UncheckedExtrinsic;

	fn sign_transaction(
		genesis_hash: <Self::Chain as ChainBase>::Hash,
		signer: &Self::AccountKeyPair,
		signer_nonce: <Self::Chain as Chain>::Index,
		call: <Self::Chain as Chain>::Call,
	) -> Self::SignedTransaction {
		let raw_payload = SignedPayload::from_raw(
			call,
			(
				frame_system::CheckSpecVersion::<pangolin_runtime::Runtime>::new(),
				frame_system::CheckTxVersion::<pangolin_runtime::Runtime>::new(),
				frame_system::CheckGenesis::<pangolin_runtime::Runtime>::new(),
				frame_system::CheckEra::<pangolin_runtime::Runtime>::from(sp_runtime::generic::Era::Immortal),
				frame_system::CheckNonce::<pangolin_runtime::Runtime>::from(signer_nonce),
				frame_system::CheckWeight::<pangolin_runtime::Runtime>::new(),
				pallet_transaction_payment::ChargeTransactionPayment::<pangolin_runtime::Runtime>::from(0),
			),
			(
				pangolin_runtime::VERSION.spec_version,
				pangolin_runtime::VERSION.transaction_version,
				genesis_hash,
				genesis_hash,
				(),
				(),
				(),
			),
		);
		let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
		let signer: sp_runtime::MultiSigner = signer.public().into();
		let (call, extra, _) = raw_payload.deconstruct();

		let s2s_extra = (
			extra.0,
			extra.1,
			extra.2,
			extra.3,
			extra.4,
			extra.5,
			extra.6,
			Default::default()
		);
		pangolin_runtime::UncheckedExtrinsic::new_signed(
			call,
			sp_runtime::MultiAddress::Id(signer.into_account()),
			signature.into(),
			s2s_extra,
		)
	}
}


