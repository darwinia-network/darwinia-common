//! # Darwinia-ethereum-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::H128;
// --- substrate ---
use crate::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::traits::Hash;
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system::{self as system, ensure_root};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::{convert::From, prelude::*};
// --- darwinia ---
use darwinia_header_mmr_rpc_runtime_api::Proof as MMRProof;
use darwinia_support::relay::*;
use ethereum_primitives::{
	header::EthHeader, merkle::DoubleNodeWithMerkleProof, pow::EthashPartial, EthBlockNumber, H256,
};

type EthereumMMR = H256;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
	{
		PhantomEvent(AccountId),
		/// The specific confirmed block is removed
		RemoveConfirmedBlock(EthBlockNumber),

		/// The range of confirmed blocks are removed
		RemoveConfirmedBlockRang(EthBlockNumber, EthBlockNumber),

		/// The block confimed block parameters are changed
		UpdateConfrimedBlockCleanCycle(EthBlockNumber, EthBlockNumber),

		/// This Error event is caused by unreasonable Confirm block delete parameter set
		///
		/// ConfirmBlockKeepInMonth should be greator then 1 to avoid the relayer game cross the
		/// month
		ConfirmBlockManagementError(EthBlockNumber),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		TargetHeaderAE,
		HeaderInvalid,
		NotComplyWithConfirmebBlocks,
		ChainInvalid,
		MMRInvalid,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthereumRelay {
		/// Ethereum last confrimed header info including ethereum block number, hash, and mmr
		pub LastConfirmedHeaderInfo get(fn last_confirm_header_info): Option<(EthBlockNumber, H256, EthereumMMR)>;

		/// The Ethereum headers confrimed by relayer game
		/// The actural storage needs to be defined
		pub ConfirmedHeadersDoubleMap get(fn confirmed_header): double_map hasher(identity) EthBlockNumber, hasher(identity) EthBlockNumber => EthHeader;

		/// Dags merkle roots of ethereum epoch (each epoch is 30000)
		pub DagsMerkleRoots get(fn dag_merkle_root): map hasher(identity) u64 => H128;

		/// The current confirm block cycle nubmer (default is one month one cycle)
		LastConfirmedBlockCycle: EthBlockNumber;

		/// The number of ehtereum blocks in a month
		pub ConfirmBlocksInCycle get(fn confirm_block_cycle): EthBlockNumber = 185142;
		/// The confirm blocks keep in month
		pub ConfirmBlockKeepInMonth get(fn confirm_block_keep_in_mounth): EthBlockNumber = 3;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		fn deposit_event() = default;
		/// Remove the specific malicous block
		#[weight = 100_000_000]
		pub fn remove_confirmed_block(origin, number: EthBlockNumber) {
			ensure_root(origin)?;
			ConfirmedHeadersDoubleMap::take(number/ConfirmBlocksInCycle::get(), number);
			Self::deposit_event(RawEvent::RemoveConfirmedBlock(number));
		}

		/// Remove the blocks in particular month (month is calculated as cycle)
		#[weight = 100_000_000]
		pub fn remove_confirmed_blocks_in_month(origin, cycle: EthBlockNumber) {
			ensure_root(origin)?;
			let c = ConfirmBlocksInCycle::get();
			ConfirmedHeadersDoubleMap::remove_prefix(cycle);
			Self::deposit_event(RawEvent::RemoveConfirmedBlockRang(cycle * c, cycle.saturating_add(1) * c));
		}

		/// Setup the parameters to delete the confirmed blocks after month * blocks_in_month
		#[weight = 100_000_000]
		pub fn set_confirmed_blocks_clean_parameters(origin, month: EthBlockNumber, blocks_in_month: EthBlockNumber) {
			ensure_root(origin)?;
			if month < 2 {
				// read the doc string of of event
				Self::deposit_event(RawEvent::ConfirmBlockManagementError(month));
			} else {
				ConfirmBlocksInCycle::set(blocks_in_month);
				ConfirmBlockKeepInMonth::set(month);
				Self::deposit_event(RawEvent::UpdateConfrimedBlockCleanCycle(month, blocks_in_month));
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// validate block with the hash, difficulty of confirmed headers
	fn verify_block_with_confrim_blocks(header: &EthHeader) -> bool {
		let eth_partial = EthashPartial::production();
		let confirm_blocks_in_cycle = ConfirmBlocksInCycle::get();
		if ConfirmedHeadersDoubleMap::contains_key(
			(header.number.saturating_sub(1)) / confirm_blocks_in_cycle,
			header.number.saturating_sub(1),
		) {
			let previous_header = ConfirmedHeadersDoubleMap::get(
				(header.number.saturating_sub(1)) / confirm_blocks_in_cycle,
				header.number.saturating_sub(1),
			);
			if header.parent_hash != previous_header.hash.unwrap_or_default()
				|| *header.difficulty()
					!= eth_partial.calculate_difficulty(header, &previous_header)
			{
				return false;
			}
		}

		if ConfirmedHeadersDoubleMap::contains_key(
			(header.number.saturating_add(1)) / confirm_blocks_in_cycle,
			header.number.saturating_add(1),
		) {
			let subsequent_header = ConfirmedHeadersDoubleMap::get(
				(header.number.saturating_add(1)) / confirm_blocks_in_cycle,
				header.number.saturating_add(1),
			);
			if header.hash.unwrap_or_default() != subsequent_header.parent_hash
				|| *subsequent_header.difficulty()
					!= eth_partial.calculate_difficulty(&subsequent_header, header)
			{
				return false;
			}
		}

		true
	}

	fn verify_block_seal(header: &EthHeader, ethash_proof: &[DoubleNodeWithMerkleProof]) -> bool {
		if header.hash() != header.re_compute_hash() {
			return false;
		}

		let eth_partial = EthashPartial::production();

		if eth_partial.verify_block_basic(header).is_err() {
			return false;
		}

		if eth_partial.verify_block_basic(header).is_err() {
			return false;
		}

		// NOTE:
		// The `ethereum-linear-relay` is ready to drop, the merkle root data will migrate to
		// `ethereum-relay`
		let merkle_root = <Module<T>>::dag_merkle_root((header.number as usize / 30000) as u64);

		if eth_partial
			.verify_seal_with_proof(&header, &ethash_proof, &merkle_root)
			.is_err()
		{
			return false;
		};

		return true;
	}

	fn verify_mmr(hash: H256, mmr: EthereumMMR, mmr_proof: MMRProof<EthereumMMR>) -> bool {
		let mut cal_mmr: Vec<u8> = hash.as_ref().iter().cloned().collect();
		for h in mmr_proof.0 {
			let encodable = (mmr, h);
			cal_mmr = <T as frame_system::Trait>::Hashing::hash_of(&encodable)
				.as_ref()
				.iter()
				.cloned()
				.collect();
		}
		cal_mmr
			.iter()
			.zip(mmr.as_ref().iter())
			.all(|(a, b)| *a == *b)
	}
}

impl<T: Trait> Relayable for Module<T> {
	type TcBlockNumber = EthBlockNumber;
	type TcHeaderHash = H256;
	type TcHeaderMMR = EthereumMMR;

	fn best_block_number() -> Self::TcBlockNumber {
		return if let Some(i) = LastConfirmedHeaderInfo::get() {
			i.0
		} else {
			0u64.into()
		};
	}

	fn header_existed(block_number: Self::TcBlockNumber) -> bool {
		ConfirmedHeadersDoubleMap::contains_key(
			block_number / ConfirmBlocksInCycle::get(),
			block_number,
		)
	}

	fn verify_raw_header_thing(
		raw_header_thing: RawHeaderThing,
	) -> Result<
		TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
		DispatchError,
	> {
		let EthHeaderThing {
			header,
			ethash_proof,
			mmr,
			mmr_proof: _,
		} = raw_header_thing.into();

		if ConfirmedHeadersDoubleMap::contains_key(
			header.number / ConfirmBlocksInCycle::get(),
			header.number,
		) {
			return Err(<Error<T>>::TargetHeaderAE)?;
		}
		if !Self::verify_block_seal(&header, &ethash_proof) {
			return Err(<Error<T>>::HeaderInvalid)?;
		};

		Ok(TcHeaderBrief {
			block_number: header.number,
			hash: header.hash.unwrap_or_default(),
			parent_hash: header.parent_hash,
			mmr,
			others: header.encode(),
		})
	}

	fn verify_raw_header_thing_chain(
		raw_header_thing_chain: Vec<RawHeaderThing>,
	) -> Result<
		Vec<TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>>,
		DispatchError,
	> {
		let output = vec![];
		let mut previous_mmr = None;

		for (idx, raw_header_thing) in raw_header_thing_chain.into_iter().enumerate() {
			let EthHeaderThing {
				header,
				ethash_proof,
				mmr,
				mmr_proof,
			} = raw_header_thing.into();

			if !Self::verify_block_seal(&header, &ethash_proof) {
				return Err(<Error<T>>::HeaderInvalid)?;
			};

			if !Self::verify_block_with_confrim_blocks(&header) {
				return Err(<Error<T>>::NotComplyWithConfirmebBlocks)?;
			}

			if idx == 0 {
				if let Some(i) = LastConfirmedHeaderInfo::get() {
					if !Self::verify_mmr(i.2, mmr, mmr_proof) {
						return Err(<Error<T>>::MMRInvalid)?;
					}
				}
			} else {
				if !Self::verify_mmr(
					header.hash.unwrap_or_default(),
					previous_mmr.unwrap_or_default(),
					mmr_proof,
				) {
					return Err(<Error<T>>::MMRInvalid)?;
				}
			}
			previous_mmr = Some(mmr);
		}
		Ok(output)
	}

	fn on_chain_arbitrate(
		header_briefs_chain: Vec<
			darwinia_support::relay::TcHeaderBrief<
				Self::TcBlockNumber,
				Self::TcHeaderHash,
				Self::TcHeaderMMR,
			>,
		>,
	) -> DispatchResult {
		// Currently Ethereum samples function is continuesly sampling

		let eth_partial = EthashPartial::production();

		for i in 1..header_briefs_chain.len() - 1 {
			if header_briefs_chain[i].parent_hash != header_briefs_chain[i + 1].hash {
				return Err(<Error<T>>::ChainInvalid)?;
			}
			let header =
				EthHeader::decode(&mut &*header_briefs_chain[i].others).unwrap_or_default();
			let previous_header =
				EthHeader::decode(&mut &*header_briefs_chain[i + 1].others).unwrap_or_default();

			if *(header.difficulty()) != eth_partial.calculate_difficulty(&header, &previous_header)
			{
				return Err(<Error<T>>::ChainInvalid)?;
			}
		}
		Ok(())
	}

	fn store_header(raw_header_thing: RawHeaderThing) -> DispatchResult {
		let last_comfirmed_block_number = if let Some(i) = LastConfirmedHeaderInfo::get() {
			i.0
		} else {
			0
		};
		let EthHeaderThing {
			header,
			ethash_proof: _,
			mmr,
			mmr_proof: _,
		} = raw_header_thing.into();

		if header.number > last_comfirmed_block_number {
			LastConfirmedHeaderInfo::set(Some((
				header.number,
				header.hash.unwrap_or_default(),
				mmr,
			)))
		};

		let confirm_cycle = header.number / ConfirmBlocksInCycle::get();
		let last_confirmed_block_cycle = LastConfirmedBlockCycle::get();

		ConfirmedHeadersDoubleMap::insert(confirm_cycle, header.number, header);

		if confirm_cycle > last_confirmed_block_cycle {
			ConfirmedHeadersDoubleMap::remove_prefix(
				confirm_cycle.saturating_sub(ConfirmBlockKeepInMonth::get()),
			);
			LastConfirmedBlockCycle::set(confirm_cycle);
		}
		Ok(())
	}
}

#[derive(Encode, Decode, Default)]
pub struct EthHeaderThing {
	header: EthHeader,
	ethash_proof: Vec<DoubleNodeWithMerkleProof>,
	mmr: EthereumMMR,
	mmr_proof: MMRProof<EthereumMMR>,
}

impl From<RawHeaderThing> for EthHeaderThing {
	fn from(raw_header_thing: RawHeaderThing) -> Self {
		EthHeaderThing::decode(&mut &*raw_header_thing).unwrap_or_default()
	}
}
