#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::Encode;
// --- paritytech ---
use beefy_primitives::{ConsensusLog, BEEFY_ENGINE_ID};
use pallet_mmr::primitives::{LeafDataProvider, OnNewRoot};
use sp_core::H256;
use sp_io::hashing;
use sp_runtime::generic::DigestItem;
use sp_std::{borrow::ToOwned, prelude::*};

#[derive(Encode)]
pub struct BeefyPayload {
	network: [u8; 32],
	mmr_root: H256,
	message_root: H256,
	encoded_next_authority_set: Vec<u8>,
}

pub struct DepositBeefyDigest<T>(PhantomData<T>);
impl<T> OnNewRoot<H256> for DepositBeefyDigest<T>
where
	T: pallet_mmr::Config<Hash = H256> + pallet_beefy::Config + pallet_beefy_mmr::Config,
{
	fn on_new_root(root: &<T as pallet_mmr::Config>::Hash) {
		let encoded_next_authority_set = <pallet_beefy_mmr::Pallet<T>>::leaf_data()
			.beefy_next_authority_set
			.encode();
		let encoded_payload = BeefyPayload {
			// TODO
			network: [0; 32],
			mmr_root: root.to_owned(),
			message_root: Default::default(),
			encoded_next_authority_set,
		}
		.encode();
		let payload_hash = hashing::keccak_256(&encoded_payload).into();

		<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
			BEEFY_ENGINE_ID,
			<ConsensusLog<<T as pallet_beefy::Config>::BeefyId>>::DarwiniaBeefyDigest(payload_hash)
				.encode(),
		));
	}
}
