//! # Darwinia-ethereum-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

mod mmr;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type Balance<T> = <CurrencyT<T> as Currency<AccountId<T>>>::Balance;

	type AccountId<T> = <T as system::Trait>::AccountId;

	type CurrencyT<T> = <T as Trait>::Currency;
}

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::H128;
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::Get,
	traits::{Currency, ExistenceRequirement::KeepAlive, ReservableCurrency},
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
	traits::AccountIdConversion, DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{convert::From, prelude::*};
// --- darwinia ---
use crate::mmr::{leaf_index_to_mmr_size, leaf_index_to_pos, MMRMerge, MerkleProof};
use array_bytes::array_unchecked;
use darwinia_support::balance::lock::LockableCurrency;
use darwinia_support::relay::*;
use ethereum_primitives::{
	header::EthHeader, merkle::EthashProof, pow::EthashPartial, receipt::Receipt, EthBlockNumber,
	H256,
};
use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};
use types::*;

type EthereumMMRHash = H256;

pub trait Trait: frame_system::Trait {
	/// The ethereum-relay's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
		+ ReservableCurrency<Self::AccountId>;
}

#[derive(Encode, Decode, Default, RuntimeDebug)]
pub struct EthHeaderThing {
	eth_header: EthHeader,
	ethash_proof: Vec<EthashProof>,
	mmr_root: EthereumMMRHash,
	mmr_proof: Vec<EthereumMMRHash>,
}

#[derive(Encode, Decode, Default, RuntimeDebug)]
pub struct ProposalEthHeaderThing {
	eth_header: EthHeader,
	mmr_root: EthereumMMRHash,
}

impl From<RawHeaderThing> for EthHeaderThing {
	fn from(raw_header_thing: RawHeaderThing) -> Self {
		EthHeaderThing::decode(&mut &*raw_header_thing).unwrap_or_default()
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthReceiptProof {
	pub index: u64,
	pub proof: Vec<u8>,
	pub header_hash: H256,
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

		VerifyReceipt(AccountId, Receipt, EthHeader),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		TargetHeaderAE,
		HeaderInvalid,
		NotComplyWithConfirmebBlocks,
		ChainInvalid,
		MMRInvalid,
		HeaderHashMis,
		LastHeaderNE,
		RlpDcF,
		ProofVF,
		TrieKeyNE,
		ReceiptDsF,
	}
}

#[cfg(feature = "std")]
darwinia_support::impl_genesis! {
	struct DagsMerkleRootsLoader {
		dags_merkle_roots: Vec<H128>
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthereumRelay {
		/// Ethereum last confrimed header info including ethereum block number, hash, and mmr
		pub LastConfirmedHeaderInfo get(fn last_confirm_header_info): Option<(EthBlockNumber, H256, EthereumMMRHash)>;

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

		pub ReceiptVerifyFee get(fn receipt_verify_fee) config(): Balance<T>;
	}
	add_extra_genesis {
		config(dags_merkle_roots_loader): DagsMerkleRootsLoader;
		build(|config| {
			let GenesisConfig {
				dags_merkle_roots_loader,
				..
			} = config;

			let dags_merkle_roots = if dags_merkle_roots_loader.dags_merkle_roots.is_empty() {
				DagsMerkleRootsLoader::from_str(r#""\"{}\"""#).dags_merkle_roots
			} else {
				dags_merkle_roots_loader.dags_merkle_roots.clone()
			};
			for (i, dag_merkle_root) in dags_merkle_roots.into_iter().enumerate() {
				DagsMerkleRoots::insert(i as u64, dag_merkle_root);
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Check receipt
		///
		/// # <weight>
		/// - `O(1)`.
		/// - Limited Storage reads
		/// - Up to one event
		/// # </weight>
		#[weight = 100_000_000]
		pub fn check_receipt(origin, proof_record: EthReceiptProof, eth_header: EthHeader, mmr_proof: Vec<H256>) {
			let worker = ensure_signed(origin)?;

			let (verified_receipt, fee) = Self::verify_receipt(&proof_record, &eth_header, mmr_proof)?;

			let module_account = Self::account_id();

			T::Currency::transfer(&worker, &module_account, fee, KeepAlive)?;

			<Module<T>>::deposit_event(RawEvent::VerifyReceipt(worker, verified_receipt, eth_header));
		}

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
	/// The account ID of the ethereum relay pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

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

	fn verify_block_seal(header: &EthHeader, ethash_proof: &[EthashProof]) -> bool {
		if header.hash() != header.re_compute_hash() {
			return false;
		}

		let eth_partial = EthashPartial::production();

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

		true
	}

	// Verify the MMR root
	// NOTE: leaves are (block_number, H256)
	// block_number will transform to position in this function
	fn verify_mmr(
		last_block_number: u64,
		mmr_root: H256,
		mmr_proof: Vec<H256>,
		leaves: Vec<(u64, H256)>,
	) -> bool {
		let p = MerkleProof::<[u8; 32], MMRMerge>::new(
			leaf_index_to_mmr_size(last_block_number),
			mmr_proof.into_iter().map(|h| h.into()).collect(),
		);
		p.verify(
			mmr_root.into(),
			leaves
				.into_iter()
				.map(|(n, h)| (leaf_index_to_pos(n), h.into()))
				.collect(),
		)
		.unwrap_or(false)
	}
}

impl<T: Trait> Relayable for Module<T> {
	type TcBlockNumber = EthBlockNumber;
	type TcHeaderHash = ethereum_primitives::H256;
	type TcHeaderMMR = sp_core::H256;

	fn best_block_number() -> Self::TcBlockNumber {
		if let Some(i) = LastConfirmedHeaderInfo::get() {
			i.0
		} else {
			0u64
		}
	}

	fn verify_raw_header_thing(
		raw_header_thing: RawHeaderThing,
		with_proposed_raw_header: bool,
	) -> Result<
		(
			TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
			RawHeaderThing,
		),
		DispatchError,
	> {
		let EthHeaderThing {
			eth_header,
			ethash_proof,
			mmr_root,
			mmr_proof: _,
		} = raw_header_thing.into();

		if ConfirmedHeadersDoubleMap::contains_key(
			eth_header.number / ConfirmBlocksInCycle::get(),
			eth_header.number,
		) {
			return Err(<Error<T>>::TargetHeaderAE.into());
		}
		if !Self::verify_block_seal(&eth_header, &ethash_proof) {
			return Err(<Error<T>>::HeaderInvalid.into());
		};
		if with_proposed_raw_header {
			Ok((
				TcHeaderBrief {
					number: eth_header.number,
					hash: eth_header.hash.unwrap_or_default(),
					parent_hash: eth_header.parent_hash,
					mmr: array_unchecked!(mmr_root, 0, 32).into(),
					others: eth_header.encode(),
				},
				ProposalEthHeaderThing {
					eth_header,
					mmr_root,
				}
				.encode(),
			))
		} else {
			Ok((
				TcHeaderBrief {
					number: eth_header.number,
					hash: eth_header.hash.unwrap_or_default(),
					parent_hash: eth_header.parent_hash,
					mmr: array_unchecked!(mmr_root, 0, 32).into(),
					others: eth_header.encode(),
				},
				vec![],
			))
		}
	}

	fn verify_raw_header_thing_chain(
		raw_header_thing_chain: Vec<RawHeaderThing>,
	) -> Result<
		Vec<TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>>,
		DispatchError,
	> {
		let mut output = vec![];
		let mut first_header_mmr = None;
		let mut first_header_number = 0;
		let chain_lengeth = raw_header_thing_chain.len();

		for (idx, raw_header_thing) in raw_header_thing_chain.into_iter().enumerate() {
			let EthHeaderThing {
				eth_header,
				ethash_proof,
				mmr_root,
				mmr_proof,
			} = raw_header_thing.into();

			if !Self::verify_block_seal(&eth_header, &ethash_proof) {
				return Err(<Error<T>>::HeaderInvalid.into());
			};

			if !Self::verify_block_with_confrim_blocks(&eth_header) {
				return Err(<Error<T>>::NotComplyWithConfirmebBlocks.into());
			}
			if idx == 0 {
				// The mmr_root of first submit should includ the hash last confirm block
				//      mmr_root of 1st
				//     / \
				//    -   -
				//   /     \
				//  C  ...  1st
				//  C: Last Comfirmed Block  1st: 1st submit block
				if let Some(l) = LastConfirmedHeaderInfo::get() {
					if chain_lengeth == 1
						&& !Self::verify_mmr(
							eth_header.number,
							array_unchecked!(mmr_root, 0, 32).into(),
							mmr_proof
								.iter()
								.map(|h| array_unchecked!(h, 0, 32).into())
								.collect(),
							vec![(l.0, l.1)],
						) {
						return Err(<Error<T>>::MMRInvalid.into());
					}
				};

				first_header_mmr = Some(array_unchecked!(mmr_root, 0, 32).into());
				first_header_number = eth_header.number;
			// the hash of other submit should be included by previous mmr_root
			} else {
				// last confirm no exsit the mmr verification will be passed
				//
				//      mmr_root of prevous submit
				//     / \
				//    - ..-
				//   /   | \
				//  -  ..c  1st
				// c: current submit  1st: 1st submit block
				if idx == chain_lengeth - 1
					&& !Self::verify_mmr(
						first_header_number,
						first_header_mmr.unwrap_or_default(),
						mmr_proof
							.iter()
							.map(|h| array_unchecked!(h, 0, 32).into())
							.collect(),
						vec![(
							eth_header.number,
							array_unchecked!(eth_header.hash.unwrap_or_default(), 0, 32).into(),
						)],
					) {
					return Err(<Error<T>>::MMRInvalid.into());
				}
			}
			output.push(TcHeaderBrief {
				number: eth_header.number,
				hash: eth_header.hash.unwrap_or_default(),
				parent_hash: eth_header.parent_hash,
				mmr: array_unchecked!(mmr_root, 0, 32).into(),
				others: eth_header.encode(),
			});
		}
		Ok(output)
	}

	fn on_chain_arbitrate(
		header_brief_chain: Vec<
			darwinia_support::relay::TcHeaderBrief<
				Self::TcBlockNumber,
				Self::TcHeaderHash,
				Self::TcHeaderMMR,
			>,
		>,
	) -> DispatchResult {
		// Currently Ethereum samples function is continuesly sampling

		let eth_partial = EthashPartial::production();

		for i in 1..header_brief_chain.len() - 1 {
			if header_brief_chain[i].parent_hash != header_brief_chain[i + 1].hash {
				return Err(<Error<T>>::ChainInvalid.into());
			}
			let header = EthHeader::decode(&mut &*header_brief_chain[i].others).unwrap_or_default();
			let previous_header =
				EthHeader::decode(&mut &*header_brief_chain[i + 1].others).unwrap_or_default();

			if *(header.difficulty()) != eth_partial.calculate_difficulty(&header, &previous_header)
			{
				return Err(<Error<T>>::ChainInvalid.into());
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
			eth_header,
			ethash_proof: _,
			mmr_root,
			mmr_proof: _,
		} = raw_header_thing.into();

		if eth_header.number > last_comfirmed_block_number {
			LastConfirmedHeaderInfo::set(Some((
				eth_header.number,
				array_unchecked!(eth_header.hash.unwrap_or_default(), 0, 32).into(),
				mmr_root,
			)))
		};

		let confirm_cycle = eth_header.number / ConfirmBlocksInCycle::get();
		let last_confirmed_block_cycle = LastConfirmedBlockCycle::get();

		ConfirmedHeadersDoubleMap::insert(confirm_cycle, eth_header.number, eth_header);

		if confirm_cycle > last_confirmed_block_cycle {
			ConfirmedHeadersDoubleMap::remove_prefix(
				confirm_cycle.saturating_sub(ConfirmBlockKeepInMonth::get()),
			);
			LastConfirmedBlockCycle::set(confirm_cycle);
		}
		Ok(())
	}
}

/// Handler for selecting the genesis validator set.
pub trait VerifyEthReceipts<Balance, AccountId> {
	fn verify_receipt(
		proof_record: &EthReceiptProof,
		eth_header: &EthHeader,
		mmr_proof: Vec<H256>,
	) -> Result<(Receipt, Balance), DispatchError>;

	fn account_id() -> AccountId;
}

impl<T: Trait> VerifyEthReceipts<Balance<T>, T::AccountId> for Module<T> {
	/// confirm that the block hash is right
	/// get the receipt MPT trie root from the block header
	/// Using receipt MPT trie root to verify the proof and index etc.
	fn verify_receipt(
		proof_record: &EthReceiptProof,
		eth_header: &EthHeader,
		mmr_proof: Vec<H256>,
	) -> Result<(Receipt, Balance<T>), DispatchError> {
		// Verify header hash
		let header_hash = eth_header.hash();

		ensure!(
			header_hash == eth_header.re_compute_hash(),
			<Error<T>>::HeaderHashMis
		);

		// Verify header member to last confirmed block using mmr proof
		let last_block_info = Self::last_confirm_header_info().ok_or(<Error<T>>::LastHeaderNE)?;

		ensure!(
			Self::verify_mmr(
				last_block_info.0,
				last_block_info.2,
				mmr_proof,
				vec![(
					eth_header.number,
					array_unchecked!(eth_header.hash.unwrap_or_default(), 0, 32).into(),
				)]
			),
			<Error<T>>::MMRInvalid
		);

		// Verify receipt proof
		let proof: Proof = rlp::decode(&proof_record.proof).map_err(|_| <Error<T>>::RlpDcF)?;
		let key = rlp::encode(&proof_record.index);
		let value =
			MerklePatriciaTrie::verify_proof(eth_header.receipts_root().0.to_vec(), &key, proof)
				.map_err(|_| <Error<T>>::ProofVF)?
				.ok_or(<Error<T>>::TrieKeyNE)?;
		let receipt = rlp::decode(&value).map_err(|_| <Error<T>>::ReceiptDsF)?;

		Ok((receipt, Self::receipt_verify_fee()))
	}

	fn account_id() -> T::AccountId {
		<Module<T>>::account_id()
	}
}
