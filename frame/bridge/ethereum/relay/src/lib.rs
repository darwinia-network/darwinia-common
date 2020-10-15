//! # Darwinia Ethereum Relay Module

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
	debug::trace,
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
	ethashproof::EthashProof,
	header::EthereumHeader,
	pow::EthashPartial,
	receipt::{EthereumReceipt, EthereumReceiptProof, EthereumTransactionIndex},
	EthereumBlockNumber, EthereumNetworkType, H256,
};
use types::*;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/dags_merkle_roots.rs"));

pub trait Trait: frame_system::Trait {
	/// The ethereum-relay's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type EthereumNetwork: Get<EthereumNetworkType>;

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

		/// EthereumReceipt Verification. [account, receipt, header]
		VerifyReceipt(AccountId, EthereumReceipt, EthereumHeader),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Header - INVALID
		HeaderInv,
		/// Confirmed Blocks - CONFLICT
		ConfirmebBlocksC,
		/// Proposal - INVALID
		ProposalInv,
		/// MMR - INVALID
		MMRInv,
		/// Header Hash - MISMATCHED
		HeaderHashMis,
		/// Confirmed Header - NOT EXISTED
		ConfirmedHeaderNE,
		/// EthereumReceipt Proof - INVALID
		ReceiptProofInv,
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
		/// Confirmed Ethereum Headers
		pub ConfirmedHeaders
			get(fn confirmed_header)
			: map hasher(identity) EthereumBlockNumber => Option<EthereumHeaderThing>;

		/// Confirmed Ethereum Block Numbers
		/// The orders are from small to large
		pub ConfirmedBlockNumbers get(fn confirmed_header_numbers): Vec<EthereumBlockNumber>;

		pub ConfirmedDepth get(fn confirmed_depth) config(): u32 = 10;

		/// Dags merkle roots of ethereum epoch (each epoch is 30000)
		pub DagsMerkleRoots get(fn dag_merkle_root): map hasher(identity) u64 => H128;

		pub ReceiptVerifyFee get(fn receipt_verify_fee) config(): RingBalance<T>;
	}
	add_extra_genesis {
		config(genesis_header_info): (Vec<u8>, H256);
		config(dags_merkle_roots_loader): DagsMerkleRootsLoader;
		build(|config| {
			let GenesisConfig {
				genesis_header_info: (genesis_header, genesis_header_mmr_root),
				dags_merkle_roots_loader,
				..
			} = config;
			let genesis_header = EthereumHeader::decode(&mut &*genesis_header.to_vec()).unwrap();

			ConfirmedBlockNumbers::mutate(|numbers| {
				numbers.push(genesis_header.number);

				ConfirmedHeaders::insert(
					genesis_header.number,
					EthereumHeaderThing {
						header: genesis_header,
						mmr_root: *genesis_header_mmr_root
					}
				);
			});


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

		#[weight = 0]
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

			let verified_receipt = Self::verify_receipt(&(eth_header.clone(), proof_record, mmr_proof)).map_err(|_| <Error<T>>::ReceiptProofInv)?;

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
			T::ApproveOrigin::ensure_origin(origin)?;

			<ReceiptVerifyFee<T>>::put(new);
		}

		/// Remove the specific malicous block
		#[weight = 100_000_000]
		pub fn remove_confirmed_block(origin, number: EthereumBlockNumber) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedBlockNumbers::mutate(|numbers| {
				if let Some(i) = numbers.iter().position(|number_| *number_ == number) {
					numbers.remove(i);
				}

				ConfirmedHeaders::remove(number);
			});

			Self::deposit_event(RawEvent::RemoveConfirmedBlock(number));
		}

		// --- root call ---

		#[weight = 10_000_000]
		pub fn clean_confirmeds(origin) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedHeaders::remove_all();
			ConfirmedBlockNumbers::put(<Vec<EthereumBlockNumber>>::new());
		}

		#[weight = 10_000_000]
		pub fn set_confirmed(origin, header_thing: EthereumHeaderThing) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedBlockNumbers::mutate(|numbers| {
				numbers.push(header_thing.header.number);

				ConfirmedHeaders::insert(header_thing.header.number, header_thing);
			});
		}
	}
}

impl<T: Trait> Module<T> {
	/// The account ID of the ethereum relay pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> AccountId<T> {
		T::ModuleId::get().into_account()
	}

	pub fn ethash_params() -> EthashPartial {
		match T::EthereumNetwork::get() {
			EthereumNetworkType::Mainnet => EthashPartial::production(),
			EthereumNetworkType::Ropsten => EthashPartial::ropsten_testnet(),
		}
	}

	pub fn verify_header(header: &EthereumHeader, ethash_proof: &[EthashProof]) -> bool {
		if header.hash() != header.re_compute_hash() {
			return false;
		}

		let eth_partial = Self::ethash_params();

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

	// TODO
	// pub fn verify_continuous() -> bool {}

	// Verify the MMR root
	// NOTE: leaves are (block_number, H256)
	// block_number will transform to position in this function
	pub fn verify_mmr(
		last_leaf: u64,
		mmr_root: H256,
		mmr_proof: Vec<H256>,
		leaves: Vec<(u64, H256)>,
	) -> bool {
		let p = MerkleProof::<[u8; 32], MMRMerge>::new(
			leaf_index_to_mmr_size(last_leaf),
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

	fn verify(
		proposal_with_proof: Vec<Self::HeaderThingWithProof>,
	) -> Result<Vec<Self::HeaderThing>, DispatchError> {
		let proposal_len = proposal_with_proof.len();

		ensure!(proposal_len != 0, <Error<T>>::ProposalInv);
		// Not allow to relay genesis header
		ensure!(
			proposal_with_proof[0].header.number > 0,
			<Error<T>>::ProposalInv
		);

		let mut proposed_mmr_root = Default::default();
		let mut last_leaf = Default::default();
		let mut proposal = vec![];

		for (i, header_with_proof) in proposal_with_proof.into_iter().enumerate() {
			let Self::HeaderThingWithProof {
				header,
				ethash_proof,
				mmr_root,
				mmr_proof,
			} = header_with_proof;

			if i == 0 {
				proposed_mmr_root = array_unchecked!(mmr_root, 0, 32).into();
				last_leaf = header.number - 1;

				if proposal_len == 1 {
					ensure!(
						Self::verify_header(&header, &ethash_proof),
						<Error<T>>::HeaderInv
					);

					let EthereumHeaderThing {
						header: last_confirmed_header,
						..
					} = Self::confirmed_header(Self::best_block_number())
						.ok_or(<Error<T>>::ConfirmedHeaderNE)?;

					// last_confirmed_header.hash should not be None.
					let (last_confirmed_block_number, last_confirmed_hash) = (
						last_confirmed_header.number,
						last_confirmed_header.hash.unwrap_or_default(),
					);

					trace!(
						target: "ethereum-relay",
						"last_leaf: {:?}\n\
						proposed_mmr_root: {:?}\n\
						mmr_proof: {:#?}\n\
						last_confirmed_block_number: {:?}\n\
						last_confirmed_hash: {:?}",
						last_leaf,
						proposed_mmr_root,
						mmr_proof,
						last_confirmed_block_number,
						last_confirmed_hash,
					);

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
							proposed_mmr_root,
							mmr_proof
								.iter()
								.map(|h| array_unchecked!(h, 0, 32).into())
								.collect(),
							vec![(last_confirmed_block_number, last_confirmed_hash)],
						),
						<Error<T>>::MMRInv
					);
				}
			} else if i == proposal_len - 1 {
				ensure!(
					Self::verify_header(&header, &ethash_proof),
					<Error<T>>::HeaderInv
				);

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
						proposed_mmr_root,
						mmr_proof
							.iter()
							.map(|h| array_unchecked!(h, 0, 32).into())
							.collect(),
						vec![(
							header.number,
							array_unchecked!(header.hash.ok_or(<Error<T>>::HeaderInv)?, 0, 32)
								.into(),
						)],
					),
					<Error<T>>::MMRInv
				);
			}

			proposal.push(Self::HeaderThing { header, mmr_root });
		}

		Ok(proposal)
	}

	fn best_block_number() -> <Self::HeaderThing as HeaderThing>::Number {
		Self::confirmed_header_numbers()
			.last()
			.copied()
			.unwrap_or(0)
	}

	fn on_chain_arbitrate(proposal: Vec<Self::HeaderThing>) -> DispatchResult {
		// Currently Ethereum samples function is continuously sampling

		let eth_partial = Self::ethash_params();

		for i in 1..proposal.len() - 1 {
			let header = &proposal[i].header;
			let prev_header = &proposal[i + 1].header;

			ensure!(
				header.parent_hash == header.hash.ok_or(<Error<T>>::ProposalInv)?,
				<Error<T>>::ProposalInv
			);
			ensure!(
				header.difficulty().to_owned()
					== eth_partial.calculate_difficulty(&header, &prev_header),
				<Error<T>>::ProposalInv
			);
		}

		Ok(())
	}

	fn store_header(header_thing: Self::HeaderThing) -> DispatchResult {
		let last_comfirmed_block_number = Self::best_block_number();

		// Not allow to relay genesis header
		ensure!(
			header_thing.header.number > last_comfirmed_block_number,
			<Error<T>>::HeaderInv
		);

		ConfirmedBlockNumbers::mutate(|numbers| {
			numbers.push(header_thing.header.number);

			// TODO: remove old numbers according to ConfirmedDepth

			ConfirmedHeaders::insert(header_thing.header.number, header_thing);
		});

		Ok(())
	}
}

impl<T: Trait> EthereumReceiptT<AccountId<T>, RingBalance<T>> for Module<T> {
	type EthereumReceiptProofThing = (EthereumHeader, EthereumReceiptProof, MMRProof);

	fn account_id() -> AccountId<T> {
		Self::account_id()
	}

	fn receipt_verify_fee() -> RingBalance<T> {
		Self::receipt_verify_fee()
	}

	fn verify_receipt(
		proof: &Self::EthereumReceiptProofThing,
	) -> Result<EthereumReceipt, DispatchError> {
		// Verify header hash
		let eth_header = &proof.0;
		let proof_record = &proof.1;
		let mmr_proof = &proof.2;
		let header_hash = eth_header.hash();

		ensure!(
			header_hash == eth_header.re_compute_hash(),
			<Error<T>>::HeaderHashMis,
		);

		ensure!(
			eth_header.number == mmr_proof.member_leaf_index,
			<Error<T>>::MMRInv,
		);

		// Verify header member to last confirmed block using mmr proof
		let EthereumHeaderThing { mmr_root, .. } =
			Self::confirmed_header(mmr_proof.last_leaf_index + 1)
				.ok_or(<Error<T>>::ConfirmedHeaderNE)?;

		ensure!(
			Self::verify_mmr(
				mmr_proof.last_leaf_index,
				mmr_root,
				mmr_proof.proof.to_vec(),
				vec![(
					eth_header.number,
					array_unchecked!(eth_header.hash.unwrap_or_default(), 0, 32).into(),
				)]
			),
			<Error<T>>::MMRInv
		);

		// Verify receipt proof
		let receipt =
			EthereumReceipt::verify_proof_and_generate(eth_header.receipts_root(), &proof_record)
				.map_err(|_| <Error<T>>::ReceiptProofInv)?;

		Ok(receipt)
	}

	fn gen_receipt_index(proof: &Self::EthereumReceiptProofThing) -> EthereumTransactionIndex {
		let proof_record = &proof.1;
		(proof_record.header_hash, proof.1.index)
	}
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct EthereumHeaderThingWithProof {
	header: EthereumHeader,
	ethash_proof: Vec<EthashProof>,
	mmr_root: H256,
	mmr_proof: Vec<H256>,
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug)]
pub struct EthereumHeaderThing {
	/// Ethereum Block Number
	pub header: EthereumHeader,
	/// MMR root of all previous headers.
	pub mmr_root: H256,
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

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct MMRProof {
	pub member_leaf_index: u64,
	pub last_leaf_index: u64,
	pub proof: Vec<H256>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct CheckEthereumRelayHeaderHash<T: Trait>(PhantomData<T>);
impl<T: Trait> CheckEthereumRelayHeaderHash<T> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}
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
						return InvalidTransaction::Custom(<Error<T>>::ProposalInv.as_u8()).into();
					}
				}
			}
		}

		Ok(ValidTransaction::default())
	}
}
