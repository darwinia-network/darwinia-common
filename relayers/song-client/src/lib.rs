//! Types used to connect to the Song-Substrate chain.

use codec::Encode;
use relay_substrate_client::{Chain, ChainBase, ChainWithBalances, Client, TransactionSignScheme};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount};
use std::time::Duration;

pub use song_node_runtime::BridgeTangCall;

/// Song header id.
pub type HeaderId = relay_utils::HeaderId<song_node_runtime::Hash, song_node_runtime::BlockNumber>;

/// Song chain definition
#[derive(Debug, Clone, Copy)]
pub struct Song;

impl ChainBase for Song {
	type BlockNumber = song_node_runtime::BlockNumber;
	type Hash = song_node_runtime::Hash;
	type Hasher = song_node_runtime::Hashing;
	type Header = song_node_runtime::Header;
}

impl Chain for Song {
	const NAME: &'static str = "Song";
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(6);

	type AccountId = song_node_runtime::AccountId;
	type Index = song_node_runtime::Index;
	type SignedBlock = song_node_runtime::SignedBlock;
	type Call = song_node_runtime::Call;
}

impl ChainWithBalances for Song {
	type NativeBalance = song_node_runtime::Balance;

	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		use frame_support::storage::generator::StorageMap;
		StorageKey(
			frame_system::Account::<song_node_runtime::Runtime>::storage_map_final_key(account_id),
		)
	}
}

impl TransactionSignScheme for Song {
	type Chain = Song;
	type AccountKeyPair = sp_core::sr25519::Pair;
	type SignedTransaction = song_node_runtime::UncheckedExtrinsic;

	fn sign_transaction(
		client: &Client<Self>,
		signer: &Self::AccountKeyPair,
		signer_nonce: <Self::Chain as Chain>::Index,
		call: <Self::Chain as Chain>::Call,
	) -> Self::SignedTransaction {
		let raw_payload =
			SignedPayload::from_raw(
				call,
				(
					frame_system::CheckSpecVersion::<song_node_runtime::Runtime>::new(),
					frame_system::CheckTxVersion::<song_node_runtime::Runtime>::new(),
					frame_system::CheckGenesis::<song_node_runtime::Runtime>::new(),
					frame_system::CheckEra::<song_node_runtime::Runtime>::from(
						sp_runtime::generic::Era::Immortal,
					),
					frame_system::CheckNonce::<song_node_runtime::Runtime>::from(signer_nonce),
					frame_system::CheckWeight::<song_node_runtime::Runtime>::new(),
					pallet_transaction_payment::ChargeTransactionPayment::<
						song_node_runtime::Runtime,
					>::from(0),
					darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel::<
						song_node_runtime::Runtime,
					>::new(),
				),
				(
					song_node_runtime::VERSION.spec_version,
					song_node_runtime::VERSION.transaction_version,
					*client.genesis_hash(),
					*client.genesis_hash(),
					(),
					(),
					(),
					(),
				),
			);
		let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
		let signer: sp_runtime::MultiSigner = signer.public().into();
		let (call, extra, _) = raw_payload.deconstruct();

		song_node_runtime::UncheckedExtrinsic::new_signed(
			call,
			signer.into_account(),
			signature.into(),
			extra,
		)
	}
}

/// Song signing params.
#[derive(Clone)]
pub struct SigningParams {
	/// Substrate transactions signer.
	pub signer: sp_core::sr25519::Pair,
}

impl SigningParams {
	/// Create signing params from SURI and password.
	pub fn from_suri(
		suri: &str,
		password: Option<&str>,
	) -> Result<Self, sp_core::crypto::SecretStringError> {
		Ok(SigningParams {
			signer: sp_core::sr25519::Pair::from_string(suri, password)?,
		})
	}
}

impl std::fmt::Debug for SigningParams {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.signer.public())
	}
}

impl Default for SigningParams {
	fn default() -> Self {
		SigningParams {
			signer: sp_keyring::AccountKeyring::Alice.pair(),
		}
	}
}

/// Song header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<song_node_runtime::Header>;
