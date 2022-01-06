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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! # Ethereum BSC(Binance Smart Chain) Pallet
//!
//! The darwinia-bridge-bsc pallet provides functionality for verifying headers which submitted by relayer and finalized
//! authority set
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! The darwinia-bridge-bsc pallet provides functions for:
//!
//! - Verify bsc headers and finalize authority set
//! - Verify a single bsc header
//!
//! ### Terminology
//!
//! - [`BSCHeader`]: The header structure of Binance Smart Chain.
//!
//! - `genesis_header` The initial header which set to this pallet before it accepts the headers submitted by relayers.
//!   We extract the initial authority set from this header and verify the headers submitted later with the extracted initial
//!   authority set. So the genesis_header must be verified manually.
//!
//!
//! - `checkpoint` checkpoint is the block that fulfill block number % epoch_length == 0. This concept comes from the implementation of
//!   Proof of Authority consensus algorithm
//!
//! ### Implementations
//! If you want to review the code, you may need to read about Authority Round and Proof of Authority consensus algorithms first. Then you may
//! look into the go implementation of bsc source code and probably focus on the consensus algorithm that bsc is using. Read the bsc official docs if you need them.
//! For this pallet:
//! The first thing you should care about is the configuration parameters of this pallet. Check the bsc official docs even the go source code to make sure you set them
//! correctly
//! In bsc explorer, choose a checkpoint block's header to set as the genesis header of this pallet. It's not important which block you take, but it's important
//! that the relayers should submit headers from `genesis_header.number + epoch_length`
//! Probably you need a tool to finish this, like POSTMAN
//! For bsc testnet, you can access API https://data-seed-prebsc-1-s1.binance.org:8545 with
//! following body input to get the header content.
//! ```json
//! {
//!    "jsonrpc": "2.0",
//!    "method": "eth_getBlockByNumber",
//!    "params": [
//!        "0x913640",
//!        false
//!    ],
//!    "id": 83
//! }
//!```
//! If you only want to verify a single header, use verify_header fn is enough. The important tip is the header's number you want verify should greater
//! than genesis header, or the answer will be NO.
//! According to the official doc of Binance Smart Chain, when the authority set changed at checkpoint header, the new authority set is not taken as finalized immediately.
//! We will wait(accept and verify) N / 2 blocks(only headers) to make sure it's safe to finalize the new authority set. N is the authority set size.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	// --- paritytech ---
	use frame_support::{pallet_prelude::*, traits::UnixTime};
	use frame_system::pallet_prelude::*;
	use sp_core::U256;
	use sp_io::crypto;
	use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};
	#[cfg(not(feature = "std"))]
	use sp_std::borrow::ToOwned;
	use sp_std::{collections::btree_set::BTreeSet, prelude::*};
	// --- darwinia-network ---
	use bsc_primitives::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type UnixTime: UnixTime;
		/// BSC configuration.
		type BSCConfiguration: Get<BSCConfiguration>;
		/// Handler for headers submission result.
		type OnHeadersSubmitted: OnHeadersSubmitted<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The size of submitted headers is not N/2+1
		InvalidHeadersSize,
		/// Block number isn't sensible
		RidiculousNumber,
		/// This header is not checkpoint
		NotCheckpoint,
		/// Invalid signer
		InvalidSigner,
		/// Signed recently
		SignedRecently,
		/// Submitted headers not enough
		HeadersNotEnough,

		/// Non empty nonce
		/// InvalidNonce is returned if a block header nonce is non-empty
		InvalidNonce,
		/// Gas limit header field is invalid.
		InvalidGasLimit,
		/// Block has too much gas used.
		TooMuchGasUsed,
		/// Non empty uncle hash
		/// InvalidUncleHash is returned if a block contains an non-empty uncle list
		InvalidUncleHash,
		/// Difficulty header field is invalid.
		InvalidDifficulty,
		/// Non-zero mix digest
		/// InvalidMixDigest is returned if a block's mix digest is non-zero
		InvalidMixDigest,
		/// Header timestamp is ahead of on-chain timestamp
		HeaderTimestampIsAhead,
		/// Extra-data 32 byte vanity prefix missing
		/// MissingVanity is returned if a block's extra-data section is shorter than
		/// 32 bytes, which is required to store the validator(signer) vanity
		///
		/// Extra-data 32 byte vanity prefix missing
		MissingVanity,
		/// Extra-data 65 byte signature suffix missing
		/// MissingSignature is returned if a block's extra-data section doesn't seem
		/// to contain a 65 byte secp256k1 signature
		MissingSignature,
		/// Invalid validator list on checkpoint block
		/// errInvalidCheckpointValidators is returned if a checkpoint block contains an
		/// invalid list of validators (i.e. non divisible by 20 bytes)
		InvalidCheckpointValidators,
		/// Non-checkpoint block contains extra validator list
		/// ExtraValidators is returned if non-checkpoint block contain validator data in
		/// their extra-data fields
		ExtraValidators,

		/// UnknownAncestor is returned when validating a block requires an ancestor that is unknown.
		UnknownAncestor,
		/// Header timestamp too close while header timestamp is too close with parent's
		HeaderTimestampTooClose,

		/// Missing signers
		CheckpointNoSigner,
		/// List of signers is invalid
		CheckpointInvalidSigners,

		/// EC_RECOVER error
		///
		/// Recover pubkey from signature error
		RecoverPubkeyFail,
	}

	#[pallet::storage]
	#[pallet::getter(fn finalized_authority)]
	pub type FinalizedAuthority<T> = StorageValue<_, Vec<Address>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn finalized_checkpoint)]
	pub type FinalizedCheckpoint<T> = StorageValue<_, BSCHeader, ValueQuery>;

	/// [`Authorities`] is the set of qualified authorities that currently active or activated in previous rounds
	/// this was added to track the older qualified authorities, to make sure we can verify a older header
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T> = StorageValue<_, Vec<Address>, ValueQuery>;

	/// [`AuthoritiesOfRound`] use a `Map<u64, Vec<u32>>` structure to track the active authorities in every epoch
	/// the key is `checkpoint.number / epoch_length`
	/// the value is the index of authorities which extracted from checkpoint block header
	/// So the the order of authorities vector **MUST** be stable.
	#[pallet::storage]
	#[pallet::getter(fn authorities_of_round)]
	pub type AuthoritiesOfRound<T> = StorageMap<_, Blake2_128Concat, u64, Vec<u32>, ValueQuery>;

	#[cfg_attr(feature = "std", derive(Default))]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub genesis_header: BSCHeader,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let initial_authority_set =
				<Pallet<T>>::extract_authorities(&self.genesis_header).unwrap();

			<Authorities<T>>::put(&initial_authority_set);
			<FinalizedAuthority<T>>::put(&initial_authority_set);
			<FinalizedCheckpoint<T>>::put(&self.genesis_header);
			<AuthoritiesOfRound<T>>::insert(
				&self.genesis_header.number / T::BSCConfiguration::get().epoch_length,
				(0u32..initial_authority_set.len() as u32).collect::<Vec<u32>>(),
			);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Verify unsigned relayed headers and finalize authority set
		#[pallet::weight(0)]
		pub fn verify_and_update_authority_set_unsigned(
			origin: OriginFor<T>,
			headers: Vec<BSCHeader>,
		) -> DispatchResultWithPostInfo {
			// ensure not signed
			frame_system::ensure_none(origin)?;

			Self::verify_and_update_authority_set(&headers)?;

			Ok(().into())
		}

		/// Verify signed relayed headers and finalize authority set
		#[pallet::weight(0)]
		pub fn verify_and_update_authority_set_signed(
			origin: OriginFor<T>,
			headers: Vec<BSCHeader>,
		) -> DispatchResultWithPostInfo {
			let submitter = frame_system::ensure_signed(origin)?;

			match Self::verify_and_update_authority_set(&headers) {
				Ok(new_authority_set) => {
					T::OnHeadersSubmitted::on_valid_authority_finalized(
						submitter,
						&new_authority_set,
					);
				}
				e => {
					T::OnHeadersSubmitted::on_invalid_headers_submitted(submitter);

					e?;
				}
			}

			Ok(().into())
		}
	}
	impl<T: Config> Pallet<T> {
		/// Return true if the header's timestamp greater or equal than current on-chain time
		pub fn is_timestamp_ahead(timestamp: u64) -> bool {
			T::UnixTime::now().as_millis() as u64 <= timestamp
		}

		/// Perform basic checks that only require header itself.
		pub fn contextless_checks(config: &BSCConfiguration, header: &BSCHeader) -> DispatchResult {
			// he genesis block is the always valid dead-end
			if header.number == 0 {
				return Ok(());
			}

			// Ensure that nonce is empty
			ensure!(header.nonce.as_slice() == [0; 8], <Error<T>>::InvalidNonce);

			// This comes from the go version of BSC header verification code
			ensure!(
				header.gas_limit >= config.min_gas_limit
					&& header.gas_limit <= config.max_gas_limit,
				<Error<T>>::InvalidGasLimit
			);
			ensure!(
				header.gas_used <= header.gas_limit,
				<Error<T>>::TooMuchGasUsed
			);

			// Ensure that the block doesn't contain any uncles which are meaningless in PoA
			ensure!(
				header.uncle_hash == KECCAK_EMPTY_LIST_RLP,
				<Error<T>>::InvalidUncleHash
			);

			// Ensure difficulty is valid
			ensure!(
				header.difficulty == DIFF_INTURN || header.difficulty == DIFF_NOTURN,
				<Error<T>>::InvalidDifficulty
			);
			// Ensure that the block's difficulty is meaningful (may not be correct at this point)
			ensure!(!header.difficulty.is_zero(), <Error<T>>::InvalidDifficulty);

			// Ensure that the mix digest is zero as we don't have fork protection currently
			ensure!(header.mix_digest.is_zero(), <Error<T>>::InvalidMixDigest);

			// Don't waste time checking blocks from the future
			ensure!(
				!Self::is_timestamp_ahead(header.timestamp),
				<Error<T>>::HeaderTimestampIsAhead
			);

			// Check that the extra-data contains the vanity, validators and signature.
			ensure!(
				header.extra_data.len() > VANITY_LENGTH,
				<Error<T>>::MissingVanity
			);

			let validator_bytes_len = header
				.extra_data
				.len()
				.checked_sub(VANITY_LENGTH + SIGNATURE_LENGTH)
				.ok_or(<Error<T>>::MissingSignature)?;
			// Ensure that the extra-data contains a validator list on checkpoint, but none otherwise
			let is_checkpoint = header.number % config.epoch_length == 0;

			if is_checkpoint {
				// Checkpoint blocks must at least contain one validator
				ensure!(
					validator_bytes_len != 0,
					<Error<T>>::InvalidCheckpointValidators
				);
				// Ensure that the validator bytes length is valid
				ensure!(
					validator_bytes_len % ADDRESS_LENGTH == 0,
					<Error<T>>::InvalidCheckpointValidators
				);
			} else {
				ensure!(validator_bytes_len == 0, <Error<T>>::ExtraValidators);
			}

			Ok(())
		}

		/// Perform checks that require access to parent header.
		pub fn contextual_checks(
			config: &BSCConfiguration,
			header: &BSCHeader,
			parent: &BSCHeader,
		) -> DispatchResult {
			// parent sanity check
			if parent.compute_hash() != header.parent_hash || parent.number + 1 != header.number {
				Err(<Error<T>>::UnknownAncestor)?;
			}

			// Ensure that the block's timestamp isn't too close to it's parent
			// And header.timestamp is greater than parents'
			if header.timestamp < parent.timestamp.saturating_add(config.period) {
				Err(<Error<T>>::HeaderTimestampTooClose)?;
			}

			Ok(())
		}

		/// Recover block creator from signature
		pub fn recover_creator(
			chain_id: u64,
			header: &BSCHeader,
		) -> Result<Address, DispatchError> {
			let data = &header.extra_data;

			ensure!(data.len() > VANITY_LENGTH, <Error<T>>::MissingVanity);
			ensure!(
				data.len() >= VANITY_LENGTH + SIGNATURE_LENGTH,
				<Error<T>>::MissingSignature
			);

			// Split `signed_extra data` and `signature`
			let (signed_data_slice, signature_slice) = data.split_at(data.len() - SIGNATURE_LENGTH);
			// convert `&[u8]` to `[u8; 65]`
			let signature = {
				let mut s = [0; SIGNATURE_LENGTH];
				s.copy_from_slice(signature_slice);

				s
			};
			// modify header and hash it
			let mut unsigned_header = header.to_owned();

			unsigned_header.extra_data = signed_data_slice.to_vec();

			let msg = unsigned_header.compute_hash_with_chain_id(chain_id);
			let pubkey = crypto::secp256k1_ecdsa_recover(&signature, msg.as_fixed_bytes())
				.map_err(|_| <Error<T>>::RecoverPubkeyFail)?;
			let creator = bsc_primitives::public_to_address(&pubkey);

			Ok(creator)
		}

		/// Extract authority set from extra_data.
		///
		/// Layout of extra_data:
		/// ----
		/// VANITY: 32 bytes
		/// Signers: N * 32 bytes as hex encoded (20 characters)
		/// Signature: 65 bytes
		/// --
		pub fn extract_authorities(header: &BSCHeader) -> Result<Vec<Address>, DispatchError> {
			let data = &header.extra_data;

			ensure!(
				data.len() > VANITY_LENGTH + SIGNATURE_LENGTH,
				<Error<T>>::CheckpointNoSigner
			);

			// extract only the portion of extra_data which includes the signer list
			let signers_raw = &data[VANITY_LENGTH..data.len() - SIGNATURE_LENGTH];

			ensure!(
				signers_raw.len() % ADDRESS_LENGTH == 0,
				<Error<T>>::CheckpointInvalidSigners
			);

			let num_signers = signers_raw.len() / ADDRESS_LENGTH;
			let signers: Vec<Address> = (0..num_signers)
				.map(|i| {
					let start = i * ADDRESS_LENGTH;
					let end = start + ADDRESS_LENGTH;

					Address::from_slice(&signers_raw[start..end])
				})
				.collect();

			Ok(signers)
		}

		/// Verify single header
		/// The header number should in the range `[genesis_header.number, finalized_checkpoint.number + N]`
		/// Before the first call of verify_and_update_authority_set extrinsic, genesis_header == finalized_checkpoint
		pub fn verify_header(header: &BSCHeader) -> DispatchResult {
			let cfg = T::BSCConfiguration::get();
			// ensure the number is in the range
			let round = header.number / cfg.epoch_length;

			ensure!(
				<AuthoritiesOfRound<T>>::contains_key(round),
				// it could be the signer which signed your header has not been finalized yet
				// or your header.number is less than the genesis header number
				<Error::<T>>::RidiculousNumber
			);

			Self::contextless_checks(&cfg, header)?;

			// get index vec
			let authorities_of_round = <AuthoritiesOfRound<T>>::get(round);
			// get all authorities
			let authorities = <Authorities<T>>::get();
			// filter authorities of this round out
			let signers = authorities_of_round
				.into_iter()
				.map(|i| authorities[i as usize])
				.collect::<Vec<_>>();
			// check signer
			let signer = Self::recover_creator(cfg.chain_id, header)?;

			ensure!(contains(&signers, signer), <Error::<T>>::InvalidSigner);

			Ok(())
		}

		/// Verify unsigned relayed headers and finalize authority set
		pub fn verify_and_update_authority_set(
			headers: &[BSCHeader],
		) -> Result<Vec<Address>, DispatchError> {
			// get finalized authority set from storage
			let last_authority_set = <FinalizedAuthority<T>>::get();

			// ensure valid length
			// we should submit at least `N / 2 + 1` headers
			ensure!(
				last_authority_set.len() / 2 < headers.len(),
				<Error::<T>>::InvalidHeadersSize
			);

			let last_checkpoint = <FinalizedCheckpoint<T>>::get();
			let checkpoint = &headers[0];
			let cfg = T::BSCConfiguration::get();

			// ensure valid header number
			// the first group headers that relayer submitted should exactly follow the initial checkpoint
			// eg. the initial header number is x, the first call of this extrinsic should submit
			// headers with numbers [x + epoch_length, x + epoch_length + 1, ...]
			ensure!(
				last_checkpoint.number + cfg.epoch_length == checkpoint.number,
				<Error::<T>>::RidiculousNumber
			);
			// ensure first element is checkpoint block header
			ensure!(
				checkpoint.number % cfg.epoch_length == 0,
				<Error::<T>>::NotCheckpoint
			);

			// verify checkpoint
			// basic checks
			Self::contextless_checks(&cfg, checkpoint)?;

			// check signer
			let signer = Self::recover_creator(cfg.chain_id, checkpoint)?;

			ensure!(
				contains(&last_authority_set, signer),
				<Error::<T>>::InvalidSigner
			);

			// extract new authority set from submitted checkpoint header
			let new_authority_set = Self::extract_authorities(checkpoint)?;
			// log already signed signer
			let mut recently = <BTreeSet<Address>>::new();

			for i in 1..headers.len() {
				Self::contextless_checks(&cfg, &headers[i])?;
				// check parent
				Self::contextual_checks(&cfg, &headers[i], &headers[i - 1])?;

				// who signed this header
				let signer = Self::recover_creator(cfg.chain_id, &headers[i])?;

				// signed must in last authority set
				ensure!(
					contains(&last_authority_set, signer),
					<Error::<T>>::InvalidSigner
				);
				// headers submitted must signed by different authority
				ensure!(!recently.contains(&signer), <Error::<T>>::SignedRecently);

				recently.insert(signer);

				// enough proof to finalize new authority set
				if recently.len() == last_authority_set.len() / 2 {
					// already have `N / 2` valid headers signed by different authority separately
					// do finalize new authority set
					<FinalizedAuthority<T>>::put(&new_authority_set);
					<FinalizedCheckpoint<T>>::put(checkpoint);

					let mut authorities = <Authorities<T>>::get();
					// track authorities
					let mut indexes = vec![];
					for authority in &new_authority_set {
						if !contains(&authorities, *authority) {
							authorities.push(*authority);
						}
						if let Some(i) = authorities
							.iter()
							.position(|authority_| authority_ == authority)
						{
							indexes.push(i as u32);
						}
					}

					<Authorities<T>>::put(authorities);
					// insert this epoch's authority indexes
					<AuthoritiesOfRound<T>>::insert(checkpoint.number / cfg.epoch_length, indexes);

					// skip the rest submitted headers
					return Ok(new_authority_set);
				}
			}

			Err(<Error<T>>::HeadersNotEnough)?
		}
	}

	/// Callbacks for header submission rewards/penalties.
	pub trait OnHeadersSubmitted<AccountId> {
		/// Called when valid headers have been submitted.
		///
		/// The submitter **must not** be rewarded for submitting valid headers, because greedy authority
		/// could produce and submit multiple valid headers (without relaying them to other peers) and
		/// get rewarded. Instead, the provider could track submitters and stop rewarding if too many
		/// headers have been submitted without finalization.
		fn on_valid_headers_submitted(submitter: AccountId, headers: &[BSCHeader]);
		/// Called when invalid headers have been submitted.
		fn on_invalid_headers_submitted(submitter: AccountId);
		/// Called when earlier submitted headers have been finalized.
		///
		/// finalized is the finalized authority set
		fn on_valid_authority_finalized(submitter: AccountId, finalized: &[Address]);
	}
	impl<AccountId> OnHeadersSubmitted<AccountId> for () {
		fn on_valid_headers_submitted(_: AccountId, _: &[BSCHeader]) {}
		fn on_invalid_headers_submitted(_: AccountId) {}
		fn on_valid_authority_finalized(_: AccountId, _: &[Address]) {}
	}

	/// BSC pallet configuration parameters.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
	pub struct BSCConfiguration {
		/// Chain ID
		pub chain_id: u64,
		/// Minimum gas limit.
		pub min_gas_limit: U256,
		/// Maximum gas limit.
		pub max_gas_limit: U256,
		/// epoch length
		pub epoch_length: u64,
		/// block period
		pub period: u64,
	}

	/// check if the signer address in a set of qualified signers
	fn contains(signers: &[Address], signer: Address) -> bool {
		signers.iter().any(|i| *i == signer)
	}
}
pub use pallet::*;
