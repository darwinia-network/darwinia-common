// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Types used to connect to the Tang-Substrate chain.

use codec::Encode;
use relay_substrate_client::{Chain, ChainBase, ChainWithBalances, Client, TransactionSignScheme};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount};
use std::time::Duration;

pub use tang_node_runtime::BridgeSongCall;

/// Tang header id.
pub type HeaderId = relay_utils::HeaderId<tang_node_runtime::Hash, tang_node_runtime::BlockNumber>;

/// Tang chain definition.
#[derive(Debug, Clone, Copy)]
pub struct Tang;

impl ChainBase for Tang {
	type BlockNumber = tang_node_runtime::BlockNumber;
	type Hash = tang_node_runtime::Hash;
	type Hasher = tang_node_runtime::Hashing;
	type Header = tang_node_runtime::Header;
}

impl Chain for Tang {
	const NAME: &'static str = "Tang";
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(5);

	type AccountId = tang_node_runtime::AccountId;
	type Index = tang_node_runtime::Index;
	type SignedBlock = tang_node_runtime::SignedBlock;
	type Call = tang_node_runtime::Call;
}

impl ChainWithBalances for Tang {
	type NativeBalance = tang_node_runtime::Balance;

	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		use frame_support::storage::generator::StorageMap;
		StorageKey(
			frame_system::Account::<tang_node_runtime::Runtime>::storage_map_final_key(account_id),
		)
	}
}

impl TransactionSignScheme for Tang {
	type Chain = Tang;
	type AccountKeyPair = sp_core::sr25519::Pair;
	type SignedTransaction = tang_node_runtime::UncheckedExtrinsic;

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
					frame_system::CheckSpecVersion::<tang_node_runtime::Runtime>::new(),
					frame_system::CheckTxVersion::<tang_node_runtime::Runtime>::new(),
					frame_system::CheckGenesis::<tang_node_runtime::Runtime>::new(),
					frame_system::CheckEra::<tang_node_runtime::Runtime>::from(
						sp_runtime::generic::Era::Immortal,
					),
					frame_system::CheckNonce::<tang_node_runtime::Runtime>::from(signer_nonce),
					frame_system::CheckWeight::<tang_node_runtime::Runtime>::new(),
					pallet_transaction_payment::ChargeTransactionPayment::<
						tang_node_runtime::Runtime,
					>::from(0),
					darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel::<tang_node_runtime::Runtime>::new(),
				),
				(
					tang_node_runtime::VERSION.spec_version,
					tang_node_runtime::VERSION.transaction_version,
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

		tang_node_runtime::UncheckedExtrinsic::new_signed(
			call,
			signer.into_account(),
			signature.into(),
			extra,
		)
	}
}

/// Tang signing params.
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

/// Tang header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<tang_node_runtime::Header>;
