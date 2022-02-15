// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet {
	// --- paritytech ---
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::H160;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::getter(fn commitment_contract)]
	pub type CommitmentContract<T> = StorageValue<_, H160, ValueQuery>;

	#[cfg_attr(feature = "std", derive(Default))]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub commitment_contract: H160,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<CommitmentContract<T>>::put(H160::default());
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_commitment_contract(
			origin: OriginFor<T>,
			commitment_contract: H160,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<CommitmentContract<T>>::put(commitment_contract);

			Ok(().into())
		}
	}
}
pub use pallet::*;

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::Encode;
// --- paritytech ---
use beefy_primitives::{ConsensusLog, BEEFY_ENGINE_ID};
use frame_support::log;
use pallet_mmr::primitives::{LeafDataProvider, OnNewRoot};
use sp_core::H256;
use sp_io::hashing;
use sp_runtime::generic::DigestItem;
use sp_std::{borrow::ToOwned, prelude::*};
// --- darwinia-network ---
use darwinia_beefy_primitives::network_ids::AsciiId;
use dp_contract::beefy;
use dvm_ethereum::InternalTransactHandler;

pub const LOG_TARGET: &str = "runtime::beefy-gadget";

#[derive(Encode)]
pub struct BeefyPayload {
	network_id: [u8; 32],
	mmr_root: H256,
	message_root: H256,
	encoded_next_authority_set: Vec<u8>,
}

pub struct DepositBeefyDigest<T, A>(PhantomData<(T, A)>);
impl<T, A> OnNewRoot<H256> for DepositBeefyDigest<T, A>
where
	T: Config
		+ pallet_mmr::Config<Hash = H256>
		+ pallet_beefy::Config
		+ pallet_beefy_mmr::Config
		+ dvm_ethereum::Config,
	A: AsciiId,
{
	fn on_new_root(root: &<T as pallet_mmr::Config>::Hash) {
		macro_rules! unwrap_or_exit {
			($r:expr, $err_msg:expr) => {
				if let Ok(r) = $r {
					r
				} else {
					log::error!(target: LOG_TARGET, "{}", $err_msg);

					return;
				}
			};
		}

		let raw_message_root = unwrap_or_exit!(
			<dvm_ethereum::Pallet<T>>::read_only_call(
				<CommitmentContract<T>>::get(),
				unwrap_or_exit!(
					beefy::commitment(),
					"Fail to encode `commitment` ABI, exit."
				)
			),
			"Fail to read message root from DVM, exit."
		);

		if raw_message_root.len() != 32 {
			log::error!(
				target: LOG_TARGET,
				"Invalid raw message root: {:?}, exit.",
				raw_message_root
			);

			return;
		}

		let network_id = A::ascii_id();
		let mmr_root = root.to_owned();
		let message_root = H256::from_slice(&raw_message_root);
		let encoded_next_authority_set = <pallet_beefy_mmr::Pallet<T>>::leaf_data()
			.beefy_next_authority_set
			.encode();

		log::debug!(
			target: LOG_TARGET,
			"\
			🥩 network_id: {:?}\
			🥩 mmr_root: {:?}\
			🥩 message_root: {:?}\
			🥩 encoded_next_authority_set: {:?}\
			",
			network_id,
			mmr_root,
			message_root,
			encoded_next_authority_set
		);

		let encoded_payload = BeefyPayload {
			network_id,
			mmr_root,
			message_root,
			encoded_next_authority_set,
		}
		.encode();
		let payload_hash = hashing::keccak_256(&encoded_payload).into();

		log::debug!(
			target: LOG_TARGET,
			"\
			🥩 encoded_payload: {:?}\
			🥩 payload_hash: {:?}\
			",
			encoded_payload,
			payload_hash
		);

		<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
			BEEFY_ENGINE_ID,
			<ConsensusLog<<T as pallet_beefy::Config>::BeefyId>>::DarwiniaBeefyDigest(payload_hash)
				.encode(),
		));
	}
}
