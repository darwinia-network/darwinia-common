// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet {
	// --- substrate ---
	use frame_support::{pallet_prelude::*, traits::UnixTime};
	use frame_system::pallet_prelude::*;
	use sp_core::U256;
	use sp_runtime::RuntimeDebug;
	use sp_std::collections::btree_set::BTreeSet;
	// --- darwinia ---
	use crate::*;
	use bp_bsc::{Address, BSCHeader};

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

	#[pallet::storage]
	#[pallet::getter(fn finalized_authority)]
	pub type FinalizedAuthority<T> = StorageValue<_, Vec<Address>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn finalized_checkpoint)]
	pub type FinalizedCheckpoint<T> = StorageValue<_, BSCHeader, ValueQuery>;

	#[cfg_attr(feature = "std", derive(Default))]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub finalized_authority: Vec<Address>,
		pub finalized_checkpoint: BSCHeader,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<FinalizedAuthority<T>>::put(&self.finalized_authority);
			<FinalizedCheckpoint<T>>::put(&self.finalized_checkpoint);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
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

			// get finalized authority set from storage
			let last_authority_set = <FinalizedAuthority<T>>::get();

			// ensure valid length
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
			verification::contextless_checks::<T>(&cfg, checkpoint).map_err(|e| e.msg())?;
			// check signer
			let signer = utils::recover_creator(checkpoint).map_err(|e| e.msg())?;
			ensure!(
				contains(&last_authority_set, signer),
				<Error::<T>>::InvalidSigner
			);

			// extract new authority set from submitted checkpoint header
			let new_authority_set = &utils::extract_authorities(checkpoint).map_err(|e| e.msg())?;

			// log already signed signer
			let mut recently = BTreeSet::<Address>::new();

			for i in 1..headers.len() {
				verification::contextless_checks::<T>(&cfg, &headers[i]).map_err(|e| e.msg())?;
				// check parent
				verification::contextual_checks(&cfg, &headers[i], &headers[i - 1])
					.map_err(|e| e.msg())?;
				// who signed this header
				let signer = utils::recover_creator(&headers[i]).map_err(|e| e.msg())?;
				// signed must in last authority set
				ensure!(
					contains(&last_authority_set, signer),
					<Error::<T>>::InvalidSigner
				);
				// headers submitted must signed by different authority
				ensure!(!recently.contains(&signer), <Error::<T>>::SignedRecently);
				recently.insert(signer);

				// enough proof to finalize new authority set
				if recently.len() >= last_authority_set.len() / 2 {
					// already have N/2 valid headers signed by different authority separately
					// finalize new authroity set
					<FinalizedAuthority<T>>::put(new_authority_set);
					<FinalizedCheckpoint<T>>::put(checkpoint);
					// skip the rest submitted headers
					return Ok(().into());
				}
			}

			Err(<Error<T>>::HeadersNotEnough)?
		}

		/// Verify signed relayed headers and finalize authority set
		#[pallet::weight(0)]
		pub fn verify_and_update_authority_set_signed(
			origin: OriginFor<T>,
			headers: Vec<BSCHeader>,
		) -> DispatchResultWithPostInfo {
			let submitter = frame_system::ensure_signed(origin)?;

			// get finalized authority set from storage
			let last_authority_set = &<FinalizedAuthority<T>>::get();

			// ensure valid length
			ensure!(
				last_authority_set.len() / 2 < headers.len(),
				<Error::<T>>::InvalidHeadersSize
			);

			let last_checkpoint = <FinalizedCheckpoint<T>>::get();
			let checkpoint = &headers[0];

			// get configuration
			let cfg: BSCConfiguration = T::BSCConfiguration::get();

			// ensure valid header number
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
			verification::contextless_checks::<T>(&cfg, checkpoint).map_err(|e| e.msg())?;
			// check signer
			let signer = utils::recover_creator(checkpoint).map_err(|e| e.msg())?;
			ensure!(
				contains(&last_authority_set, signer),
				<Error::<T>>::InvalidSigner
			);

			// extract new authority set from submitted checkpoint header
			let new_authority_set = &utils::extract_authorities(checkpoint).map_err(|e| e.msg())?;

			// log already signed signer
			let mut recently = BTreeSet::<Address>::new();

			for i in 1..headers.len() {
				verification::contextless_checks::<T>(&cfg, &headers[i]).map_err(|e| e.msg())?;
				// check parent
				verification::contextual_checks(&cfg, &headers[i], &headers[i - 1])
					.map_err(|e| e.msg())?;
				// who signed this header
				let signer = utils::recover_creator(&headers[i]).map_err(|e| e.msg())?;
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
					// already have N/2 valid headers signed by different authority separately
					// finalize new authroity set
					<FinalizedAuthority<T>>::put(new_authority_set);
					<FinalizedCheckpoint<T>>::put(checkpoint);
					T::OnHeadersSubmitted::on_valid_authority_finalized(
						submitter,
						new_authority_set,
					);
					// skip the rest submitted headers
					return Ok(().into());
				}
			}
			T::OnHeadersSubmitted::on_invalid_headers_submitted(submitter);

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
		/// Minimum gas limit.
		pub min_gas_limit: U256,
		/// Maximum gas limit.
		pub max_gas_limit: U256,
		/// epoch length
		pub epoch_length: u64,
		/// block period
		pub period: u64,
	}

	fn contains(signers: &[Address], signer: Address) -> bool {
		signers.iter().any(|i| *i == signer)
	}
}
pub use pallet::*;

mod error;
mod utils;
mod verification;
