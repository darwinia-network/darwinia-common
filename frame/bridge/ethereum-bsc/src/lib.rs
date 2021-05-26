// Copyright 2019-2021 Parity Technologies (UK) Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]
// Runtime-generated enums
#![allow(clippy::large_enum_variant)]

use bp_bsc::{Address, BSCHeader};
use codec::{Decode, Encode};
use frame_support::{decl_error, decl_module, decl_storage, ensure, traits::Get};
use sp_core::U256;
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod error;
mod utils;
mod verification;

/// BSC  pallet configuration parameters.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct BSCConfiguration {
	/// Minimum gas limit.
	pub min_gas_limit: U256,
	/// Maximum gas limit.
	pub max_gas_limit: U256,
	/// epoch length
	pub epoch_length: u64,
	/// block period
	pub period: u64,
}

/// ChainTime represents the runtime on-chain time
pub trait ChainTime: Default {
	/// Is a header timestamp ahead of the current on-chain time.
	///
	/// Check whether `timestamp` is ahead (i.e greater than) the current on-chain
	/// time. If so, return `true`, `false` otherwise.
	fn is_timestamp_ahead(&self, timestamp: u64) -> bool;
}

/// ChainTime implementation for the empty type.
///
/// This implementation will allow a runtime without the timestamp pallet to use
/// the empty type as its ChainTime associated type.
impl ChainTime for () {
	/// Is a header timestamp ahead of the current on-chain time.
	///
	/// Check whether `timestamp` is ahead (i.e greater than) the current on-chain
	/// time. If so, return `true`, `false` otherwise.
	fn is_timestamp_ahead(&self, timestamp: u64) -> bool {
		// This should succeed under the contraints that the system clock works
		let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
		Duration::from_secs(timestamp) > now
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
	fn on_valid_headers_submitted(_submitter: AccountId, _headers: &[BSCHeader]) {}
	fn on_invalid_headers_submitted(_submitter: AccountId) {}
	fn on_valid_authority_finalized(_submitter: AccountId, _finalized: &[Address]) {}
}

/// The module configuration trait.
pub trait Config: frame_system::Config {
	/// BSC configuration.
	type BSCConfiguration: Get<BSCConfiguration>;
	/// Header timestamp verification against current on-chain time.
	type ChainTime: ChainTime;
	/// Handler for headers submission result.
	type OnHeadersSubmitted: OnHeadersSubmitted<Self::AccountId>;
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Block number isn't sensible.
		RidiculousNumber,
		/// The size of submitted headers is not N/2+1
		InvalidHeadersSize,
		/// This header is not checkpoint,
		NotCheckpoint,
		/// Invalid signer
		InvalidSigner,
		/// Submitted headers not enough
		HeadersNotEnough,
		/// Signed recently
		SignedRecently,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		/// Verify unsigned relayed headers and finalize authority set
		#[weight = 0]
		pub fn verify_and_update_authority_set_unsigned(origin, headers: Vec<BSCHeader>) {
			// ensure not signed
			frame_system::ensure_none(origin)?;

			// get finalized authority set from storage
			let last_authority_set = &FinalizedAuthority::get();

			// ensure valid length
			ensure!(last_authority_set.len() / 2 < headers.len(), <Error::<T>>::InvalidHeadersSize);

			let last_checkpoint = FinalizedCheckpoint::get();
			let checkpoint = &headers[0];

			// get configuration
			let cfg: BSCConfiguration = T::BSCConfiguration::get();

			// ensure valid header number
			// the first group headers that relayer submitted should exactly follow the initial checkpoint
			// eg. the initial header number is x, the first call of this extrinsic shoule submit
			// headers with nunbers [x + epoch_length, x + epoch_length + 1, ...]
			ensure!(last_checkpoint.number + cfg.epoch_length == checkpoint.number, <Error::<T>>::RidiculousNumber);

			// ensure first element is checkpoint block header
			ensure!(checkpoint.number % cfg.epoch_length == 0, <Error::<T>>::NotCheckpoint);

			// verify checkpoint
			// basic checks
			verification::contextless_checks(&cfg, checkpoint, &T::ChainTime::default()).map_err(|e|e.msg())?;
			// check signer
			let signer = utils::recover_creator(checkpoint).map_err(|e| e.msg())?;
			ensure!(Self::contains(last_authority_set, signer), <Error::<T>>::InvalidSigner);


			// extract new authority set from submitted checkpoint header
			let new_authority_set = &utils::extract_authorities(checkpoint).map_err(|e| e.msg())?;

			// log already signed signer
			let mut recently = BTreeSet::<Address>::new();

			for i in 1..headers.len() {
				verification::contextless_checks(&cfg, &headers[i], &T::ChainTime::default()).map_err(|e|e.msg())?;
				// check parent
				verification::contextual_checks(&cfg, &headers[i], &headers[i-1]).map_err(|e|e.msg())?;
				// who signed this header
				let signer = utils::recover_creator(&headers[i]).map_err(|e| e.msg())?;
				// signed must in last authority set
				ensure!(Self::contains(last_authority_set, signer), <Error::<T>>::InvalidSigner);
				// headers submitted must signed by different authority
				ensure!(!recently.contains(&signer), <Error::<T>>::SignedRecently);
				recently.insert(signer);

				// enough proof to finalize new authority set
				if recently.len() >= last_authority_set.len()/2 {
					// already have N/2 valid headers signed by different authority separately
					// finalize new authroity set
					FinalizedAuthority::put(new_authority_set);
					FinalizedCheckpoint::put(checkpoint);
					// skip the rest submitted headers
					return Ok(());
				}
			}

			// <Error::<T>>::HeadersNotEnough
		}

		/// Verify signed relayed headers and finalize authority set
		#[weight = 0]
		pub fn verify_and_update_authority_set_signed(origin, headers: Vec<BSCHeader>) {
			let submitter = frame_system::ensure_signed(origin)?;

			// get finalized authority set from storage
			let last_authority_set = &FinalizedAuthority::get();

			// ensure valid length
			ensure!(last_authority_set.len() / 2 < headers.len(), <Error::<T>>::InvalidHeadersSize);

			let last_checkpoint = FinalizedCheckpoint::get();
			let checkpoint = &headers[0];

			// get configuration
			let cfg: BSCConfiguration = T::BSCConfiguration::get();

			// ensure valid header number
			ensure!(last_checkpoint.number + cfg.epoch_length == checkpoint.number, <Error::<T>>::RidiculousNumber);

			// ensure first element is checkpoint block header
			ensure!(checkpoint.number % cfg.epoch_length == 0, <Error::<T>>::NotCheckpoint);

			// verify checkpoint
			// basic checks
			verification::contextless_checks(&cfg, checkpoint, &T::ChainTime::default()).map_err(|e|e.msg())?;
			// check signer
			let signer = utils::recover_creator(checkpoint).map_err(|e| e.msg())?;
			ensure!(Self::contains(last_authority_set, signer), <Error::<T>>::InvalidSigner);


			// extract new authority set from submitted checkpoint header
			let new_authority_set = &utils::extract_authorities(checkpoint).map_err(|e| e.msg())?;

			// log already signed signer
			let mut recently = BTreeSet::<Address>::new();

			for i in 1..headers.len() {
				verification::contextless_checks(&cfg, &headers[i], &T::ChainTime::default()).map_err(|e|e.msg())?;
				// check parent
				verification::contextual_checks(&cfg, &headers[i], &headers[i-1]).map_err(|e|e.msg())?;
				// who signed this header
				let signer = utils::recover_creator(&headers[i]).map_err(|e| e.msg())?;
				// signed must in last authority set
				ensure!(Self::contains(last_authority_set, signer), <Error::<T>>::InvalidSigner);
				// headers submitted must signed by different authority
				ensure!(!recently.contains(&signer), <Error::<T>>::SignedRecently);
				recently.insert(signer);

				// enough proof to finalize new authority set
				if recently.len() == last_authority_set.len()/2 {
					// already have N/2 valid headers signed by different authority separately
					// finalize new authroity set
					FinalizedAuthority::put(new_authority_set);
					FinalizedCheckpoint::put(checkpoint);
					T::OnHeadersSubmitted::on_valid_authority_finalized(submitter, new_authority_set);
					// skip the rest submitted headers
					return Ok(());
				}
			}
			T::OnHeadersSubmitted::on_invalid_headers_submitted(submitter);
			// <Error::<T>>::HeadersNotEnough
		}
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Bridge {
		/// Finalized authority set.
		FinalizedAuthority get(fn finalized_authority) config(): Vec<Address>;
		FinalizedCheckpoint get(fn finalized_checkpoint) config(): BSCHeader;
	}
	add_extra_genesis {
		config(initial_header): BSCHeader;
		build(|config| {
			initialize_storage::<T>(
				&config.initial_header,
			);
		})
	}
}

/// Initialize storage.
#[cfg(any(feature = "std"))]
pub(crate) fn initialize_storage<T: Config>(header: &BSCHeader) {
	// extract initial authority set checkpoint header
	let initial_authority_set = &utils::extract_authorities(header);
	assert!(initial_authority_set.is_ok());
	FinalizedAuthority::put(initial_authority_set.as_ref().unwrap());
	FinalizedCheckpoint::put(header)
}

impl<T: Config> Module<T> {
	pub fn contains(signers: &[Address], signer: Address) -> bool {
		signers.iter().any(|i| *i == signer)
	}
}
