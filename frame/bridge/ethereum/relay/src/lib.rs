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

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type RingBalance<T> = <CurrencyT<T> as Currency<AccountId<T>>>::Balance;

	pub type MMRProof = Vec<H256>;

	type CurrencyT<T> = <T as Trait>::Currency;
}

// --- core ---
use core::fmt::{Debug, Formatter, Result as FmtResult};
// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::H128;
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::Get,
	traits::{Currency, EnsureOrigin, ExistenceRequirement::KeepAlive, ReservableCurrency},
	unsigned::{TransactionValidity, TransactionValidityError},
	IsSubType,
};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{AccountIdConversion, DispatchInfoOf, Dispatchable, SignedExtension},
	transaction_validity::{InvalidTransaction, ValidTransaction},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{convert::From, marker::PhantomData, prelude::*};
// --- darwinia ---
use crate::mmr::{leaf_index_to_mmr_size, leaf_index_to_pos, MMRMerge, MerkleProof};
use array_bytes::array_unchecked;
use darwinia_relay_primitives::*;
use darwinia_support::{
	balance::lock::LockableCurrency, traits::EthereumReceipt as EthereumReceiptT,
};
use ethereum_primitives::{
	error::EthereumError,
	ethashproof::EthashProof,
	header::EthereumHeader,
	pow::EthashPartial,
	receipt::{EthereumReceipt, EthereumReceiptProof, EthereumTransactionIndex},
	EthereumBlockNumber, H256,
};
use types::*;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/dags_merkle_roots.rs"));

pub trait Trait: frame_system::Trait {
	/// The ethereum-relay's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type Call: Dispatchable + From<Call<Self>> + IsSubType<Call<Self>> + Clone;

	type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
		+ ReservableCurrency<Self::AccountId>;

	type RelayerGame: RelayerGameProtocol<
		Relayer = AccountId<Self>,
		Balance = RingBalance<Self>,
		HeaderThingWithProof = EthereumHeaderThingWithProof,
		HeaderThing = EthereumHeaderThing,
	>;

	type ApproveOrigin: EnsureOrigin<Self::Origin>;

	type RejectOrigin: EnsureOrigin<Self::Origin>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
	{
		/// The specific confirmed block is removed. [block height]
		RemoveConfirmedBlock(EthereumBlockNumber),

		/// The range of confirmed blocks are removed. [block height, block height]
		RemoveConfirmedBlockRang(EthereumBlockNumber, EthereumBlockNumber),

		/// The block confimed block parameters are changed. [block height, block height]
		UpdateConfrimedBlockCleanCycle(EthereumBlockNumber, EthereumBlockNumber),

		/// This Error event is caused by unreasonable Confirm block delete parameter set.
		///
		/// ConfirmBlockKeepInMonth should be greator then 1 to avoid the relayer game cross the
		/// month.
		/// [block height]
		ConfirmBlockManagementError(EthereumBlockNumber),

		/// EthereumReceipt Verification. [account, receipt, header]
		VerifyReceipt(AccountId, EthereumReceipt, EthereumHeader),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Target Header - ALREADY EXISTED
		TargetHeaderAE,
		/// Header - INVALID
		HeaderI,
		/// Confirmed Blocks - CONFLICT
		ConfirmebBlocksC,
		/// Proposal - INVALID
		ProposalI,
		/// MMR - INVALID
		MMRI,
		/// Header Hash - MISMATCHED
		HeaderHashMis,
		/// Last Header - NOT EXISTED
		LastHeaderNE,
		/// EthereumReceipt Proof - INVALID
		ReceiptProofI,
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
		pub
			LastConfirmedHeaderInfo
			get(fn last_confirm_header_info)
			: Option<(EthereumBlockNumber, H256, H256)>;

		/// The Ethereum headers confrimed by relayer game
		/// The actural storage needs to be defined
		pub
			ConfirmedHeadersDoubleMap
			get(fn confirmed_header)
			: double_map hasher(identity) EthereumBlockNumber, hasher(identity) EthereumBlockNumber
			=> EthereumHeader;

		/// Dags merkle roots of ethereum epoch (each epoch is 30000)
		pub DagsMerkleRoots get(fn dag_merkle_root): map hasher(identity) u64 => H128;

		/// The current confirm block cycle nubmer (default is one month one cycle)
		LastConfirmedBlockCycle: EthereumBlockNumber;

		/// The number of ehtereum blocks in a month
		pub ConfirmBlocksInCycle get(fn confirm_block_cycle): EthereumBlockNumber = 185142;
		/// The confirm blocks keep in month
		pub ConfirmBlockKeepInMonth get(fn confirm_block_keep_in_mounth): EthereumBlockNumber = 3;

		pub ReceiptVerifyFee get(fn receipt_verify_fee) config(): RingBalance<T>;
	}
	add_extra_genesis {
		config(dags_merkle_roots_loader): DagsMerkleRootsLoader;
		build(|config| {
			let GenesisConfig {
				dags_merkle_roots_loader,
				..
			} = config;

			let dags_merkle_roots = if dags_merkle_roots_loader.dags_merkle_roots.is_empty() {
				DagsMerkleRootsLoader::from_str(DAGS_MERKLE_ROOTS_STR).dags_merkle_roots.clone()
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

		#[weight = 100_000_000]
		pub fn submit_proposal(origin, proposal: Vec<EthereumHeaderThingWithProof>) {
			let relayer = ensure_signed(origin)?;

			T::RelayerGame::submit_proposal(relayer, proposal)?;
		}

		#[weight = 100_000_000]
		pub fn approve_pending_header(origin, pending: EthereumBlockNumber) {
			T::ApproveOrigin::ensure_origin(origin)?;
			T::RelayerGame::approve_pending_header(pending)?;
		}

		#[weight = 100_000_000]
		pub fn reject_pending_header(origin, pending: EthereumBlockNumber) {
			T::RejectOrigin::ensure_origin(origin)?;
			T::RelayerGame::reject_pending_header(pending)?;
		}

		/// Check and verify the receipt
		///
		/// `check_receipt` will verify the validation of the ethereum receipt proof from ethereum.
		/// Ethereum receipt proof are constructed with 3 parts.
		///
		/// The first part `proof_record` is the Ethereum receipt and its merkle member proof regarding
		/// to the receipt root in related Ethereum block header.
		///
		/// The second part `eth_header` is the Ethereum block header which included/generated this
		/// receipt, we need to provide this as part of proof, because in Darwinia Relay, we only have
		/// last confirmed block's MMR root, don't have previous blocks, so we need to include this to
		/// provide the `receipt_root` inside it, we will need to verify validation by checking header hash.
		///
		/// The third part `mmr_proof` is the mmr proof generate according to
		/// `(member_index=[eth_header.number], last_index=last_confirmed_block_header.number)`
		/// it can prove that the `eth_header` is the chain which is committed by last confirmed block's `mmr_root`
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		///
		/// # <weight>
		/// - `O(1)`.
		/// - Limited Storage reads
		/// - Up to one event
		///
		/// Related functions:
		///
		///   - `set_receipt_verify_fee` can be used to set the verify fee for each receipt check.
		/// # </weight>
		#[weight = 100_000_000]
		pub fn check_receipt(origin, proof_record: EthereumReceiptProof, eth_header: EthereumHeader, mmr_proof: MMRProof) {
			let worker = ensure_signed(origin)?;

			let verified_receipt = Self::verify_receipt(&(eth_header.clone(), proof_record, mmr_proof)).map_err(|_| <Error<T>>::ReceiptProofI)?;

			let fee = Self::receipt_verify_fee();

			let module_account = Self::account_id();

			T::Currency::transfer(&worker, &module_account, fee, KeepAlive)?;

			<Module<T>>::deposit_event(RawEvent::VerifyReceipt(worker, verified_receipt, eth_header));
		}

		/// Set verify receipt fee
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_receipt_verify_fee(origin, #[compact] new: RingBalance<T>) {
			T::RejectOrigin::ensure_origin(origin)?;

			<ReceiptVerifyFee<T>>::put(new);
		}

		/// Remove the specific malicous block
		#[weight = 100_000_000]
		pub fn remove_confirmed_block(origin, number: EthereumBlockNumber) {
			T::RejectOrigin::ensure_origin(origin)?;

			ConfirmedHeadersDoubleMap::take(number / ConfirmBlocksInCycle::get(), number);

			Self::deposit_event(RawEvent::RemoveConfirmedBlock(number));
		}

		/// Remove the blocks in particular month (month is calculated as cycle)
		#[weight = 100_000_000]
		pub fn remove_confirmed_blocks_in_month(origin, cycle: EthereumBlockNumber) {
			T::RejectOrigin::ensure_origin(origin)?;

			let c = ConfirmBlocksInCycle::get();

			ConfirmedHeadersDoubleMap::remove_prefix(cycle);

			Self::deposit_event(RawEvent::RemoveConfirmedBlockRang(cycle * c, cycle.saturating_add(1) * c));
		}

		/// Setup the parameters to delete the confirmed blocks after month * blocks_in_month
		#[weight = 100_000_000]
		pub fn set_confirmed_blocks_clean_parameters(origin, month: EthereumBlockNumber, blocks_in_month: EthereumBlockNumber) {
			T::RejectOrigin::ensure_origin(origin)?;

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
	fn account_id() -> AccountId<T> {
		T::ModuleId::get().into_account()
	}

	/// validate block with the hash, difficulty of confirmed headers
	fn verify_block_with_confrim_blocks(header: &EthereumHeader) -> bool {
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

	fn verify_block_seal(header: &EthereumHeader, ethash_proof: &[EthashProof]) -> bool {
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

	fn verify_basic(header: &EthereumHeader, ethash_proof: &[EthashProof]) -> DispatchResult {
		ensure!(
			Self::verify_block_seal(header, ethash_proof),
			<Error<T>>::HeaderI
		);
		ensure!(
			Self::verify_block_with_confrim_blocks(header),
			<Error<T>>::ConfirmebBlocksC
		);

		Ok(())
	}

	// Verify the MMR root
	// NOTE: leaves are (block_number, H256)
	// block_number will transform to position in this function
	fn verify_mmr(
		last_block_number: u64,
		mmr_root: H256,
		mmr_proof: MMRProof,
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
	type HeaderThingWithProof = EthereumHeaderThingWithProof;
	type HeaderThing = EthereumHeaderThing;

	fn basic_verify(
		proposal_with_proof: Vec<Self::HeaderThingWithProof>,
	) -> Result<Vec<Self::HeaderThing>, DispatchError> {
		let proposal_len = proposal_with_proof.len();

		ensure!(proposal_len != 0, <Error<T>>::ProposalI);
		// Not allow to relay genesis header
		ensure!(
			proposal_with_proof[0].header.number > 0,
			<Error<T>>::ProposalI
		);

		let mut proposal = vec![];
		let mut proposal_with_proof = proposal_with_proof.into_iter();
		let (proposed_header_mmr_root, last_leaf) = {
			let Self::HeaderThingWithProof {
				header,
				ethash_proof,
				mmr_root,
				mmr_proof,
			} = proposal_with_proof.next().unwrap();

			Self::verify_basic(&header, &ethash_proof)?;

			let parsed_mmr_root = array_unchecked!(mmr_root, 0, 32).into();
			let last_leaf = header.number - 1;

			if proposal_len == 1 {
				if let Some(l) = LastConfirmedHeaderInfo::get() {
					// The mmr_root of first submit should includ the hash last confirm block
					//      mmr_root of 1st
					//     / \
					//    -   -
					//   /     \
					//  C  ...  1st
					//  C: Last Comfirmed Block 1st: 1st submit block
					ensure!(
						Self::verify_mmr(
							last_leaf,
							parsed_mmr_root,
							mmr_proof
								.iter()
								.map(|h| array_unchecked!(h, 0, 32).into())
								.collect(),
							vec![(l.0, l.1)],
						),
						<Error<T>>::MMRI
					);
				}
			}

			proposal.push(Self::HeaderThing { header, mmr_root });

			(parsed_mmr_root, last_leaf)
		};

		for header_with_proof in proposal_with_proof.into_iter() {
			let Self::HeaderThingWithProof {
				header,
				ethash_proof,
				mmr_root,
				mmr_proof,
			} = header_with_proof;

			Self::verify_basic(&header, &ethash_proof)?;

			// last confirm no exsit the mmr verification will be passed
			//
			//      mmr_root of prevous submit
			//     / \
			//    - ..-
			//   /   | \
			//  -  ..c  1st
			// c: current submit  1st: 1st submit block
			ensure!(
				Self::verify_mmr(
					last_leaf,
					proposed_header_mmr_root,
					mmr_proof
						.iter()
						.map(|h| array_unchecked!(h, 0, 32).into())
						.collect(),
					vec![(
						header.number,
						array_unchecked!(header.hash.ok_or(<Error<T>>::HeaderI)?, 0, 32).into(),
					)],
				),
				<Error<T>>::MMRI
			);

			proposal.push(Self::HeaderThing { header, mmr_root });
		}

		Ok(proposal)
	}

	fn best_block_number() -> <Self::HeaderThing as HeaderThing>::Number {
		if let Some(i) = LastConfirmedHeaderInfo::get() {
			i.0
		} else {
			0
		}
	}

	fn on_chain_arbitrate(proposal: Vec<Self::HeaderThing>) -> DispatchResult {
		// Currently Ethereum samples function is continuously sampling

		let eth_partial = EthashPartial::production();

		for i in 1..proposal.len() - 1 {
			let header = &proposal[i].header;
			let prev_header = &proposal[i + 1].header;

			ensure!(
				header.parent_hash == header.hash.ok_or(<Error<T>>::ProposalI)?,
				<Error<T>>::ProposalI
			);
			ensure!(
				header.difficulty().to_owned()
					== eth_partial.calculate_difficulty(&header, &prev_header),
				<Error<T>>::ProposalI
			);
		}

		Ok(())
	}

	fn store_header(header_thing: Self::HeaderThing) -> DispatchResult {
		let last_comfirmed_block_number = if let Some(i) = LastConfirmedHeaderInfo::get() {
			i.0
		} else {
			0
		};
		let EthereumHeaderThing { header, mmr_root } = header_thing;

		if header.number > last_comfirmed_block_number {
			LastConfirmedHeaderInfo::set(Some((
				header.number,
				array_unchecked!(header.hash.unwrap_or_default(), 0, 32).into(),
				mmr_root,
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

impl<T: Trait> EthereumReceiptT<AccountId<T>, RingBalance<T>> for Module<T> {
	type EthereumReceiptProof = (EthereumHeader, EthereumReceiptProof, MMRProof);

	fn account_id() -> AccountId<T> {
		Self::account_id()
	}

	fn receipt_verify_fee() -> RingBalance<T> {
		Self::receipt_verify_fee()
	}

	fn verify_receipt(
		proof: &Self::EthereumReceiptProof,
	) -> Result<EthereumReceipt, EthereumError> {
		// Verify header hash
		let eth_header = &proof.0;
		let proof_record = &proof.1;
		let mmr_proof = &proof.2;
		let header_hash = eth_header.hash();

		ensure!(
			header_hash == eth_header.re_compute_hash(),
			EthereumError::InvalidReceiptProof
		);

		// Verify header member to last confirmed block using mmr proof
		let last_block_info =
			Self::last_confirm_header_info().ok_or(EthereumError::InvalidReceiptProof)?;

		ensure!(
			Self::verify_mmr(
				last_block_info.0,
				last_block_info.2,
				mmr_proof.to_vec(),
				vec![(
					eth_header.number,
					array_unchecked!(eth_header.hash.unwrap_or_default(), 0, 32).into(),
				)]
			),
			EthereumError::InvalidReceiptProof
		);

		// Verify receipt proof
		let receipt =
			EthereumReceipt::verify_proof_and_generate(eth_header.receipts_root(), &proof_record)
				.map_err(|_| EthereumError::InvalidReceiptProof)?;

		Ok(receipt)
	}

	fn gen_receipt_index(proof: &Self::EthereumReceiptProof) -> EthereumTransactionIndex {
		let proof_record = &proof.1;
		(proof_record.header_hash, proof.1.index)
	}
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct EthereumHeaderThingWithProof {
	header: EthereumHeader,
	ethash_proof: Vec<EthashProof>,
	mmr_root: H256,
	mmr_proof: Vec<H256>,
}

#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct EthereumHeaderThing {
	header: EthereumHeader,
	mmr_root: H256,
}
impl HeaderThing for EthereumHeaderThing {
	type Number = EthereumBlockNumber;
	type Hash = H256;

	fn number(&self) -> Self::Number {
		self.header.number()
	}

	fn hash(&self) -> Self::Hash {
		self.header.hash()
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct CheckEthereumRelayHeaderHash<T: Trait>(PhantomData<T>);
impl<T: Trait> Debug for CheckEthereumRelayHeaderHash<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "CheckEthereumRelayHeaderHash")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut Formatter) -> FmtResult {
		Ok(())
	}
}
impl<T: Send + Sync + Trait> SignedExtension for CheckEthereumRelayHeaderHash<T> {
	const IDENTIFIER: &'static str = "CheckEthereumRelayHeaderHash";
	type AccountId = T::AccountId;
	type Call = <T as Trait>::Call;
	type AdditionalSigned = ();
	type Pre = ();

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		_: &Self::AccountId,
		call: &Self::Call,
		_: &DispatchInfoOf<Self::Call>,
		_: usize,
	) -> TransactionValidity {
		if let Some(Call::submit_proposal(ref proposal)) = call.is_sub_type() {
			if let Some(proposed_header_thing) = proposal.get(0) {
				for existed_proposal in
					T::RelayerGame::proposals_of_game(proposed_header_thing.header.number)
				{
					if existed_proposal
						.bonded_proposal
						.iter()
						.zip(proposal.iter())
						.all(
							|(
								(
									_,
									EthereumHeaderThing {
										header: header_a,
										mmr_root: mmr_root_a,
									},
								),
								EthereumHeaderThingWithProof {
									header: header_b,
									mmr_root: mmr_root_b,
									..
								},
							)| header_a == header_b && mmr_root_a == mmr_root_b,
						) {
						return InvalidTransaction::Custom(<Error<T>>::ProposalI.as_u8()).into();
					}
				}
			}
		}

		Ok(ValidTransaction::default())
	}
}
