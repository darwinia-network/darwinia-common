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
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::Get,
	traits::{Currency, EnsureOrigin, ExistenceRequirement::KeepAlive, ReservableCurrency},
	unsigned::{TransactionValidity, TransactionValidityError},
	IsSubType,
};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{AccountIdConversion, DispatchInfoOf, Dispatchable, SignedExtension},
	transaction_validity::ValidTransaction,
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
		RelayHeaderId = EthereumBlockNumber,
		RelayHeaderParcel = EthereumRelayHeaderParcel,
		RelayProofs = EthereumRelayProofs,
	>;

	type ApproveOrigin: EnsureOrigin<Self::Origin>;

	type RejectOrigin: EnsureOrigin<Self::Origin>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
	{
		/// The specific confirmed parcel removed. [block id]
		RemoveConfirmedParcel(EthereumBlockNumber),
		/// EthereumReceipt verification. [account, ethereum receipt, ethereum header]
		VerifyReceipt(AccountId, EthereumReceipt, EthereumHeader),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Header - INVALID
		HeaderInv,
		/// Confirmed Blocks - CONFLICT
		ConfirmedBlocksC,
		/// Continuous - INVALID
		ContinuousInv,
		// /// Proposal - INVALID
		// ProposalInv,
		/// Header Hash - INVALID
		HeaderHashInv,
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
		/// Confirmed ethereum parcel
		pub ConfirmedParcels
			get(fn confirmed_parcel_of)
			: map hasher(identity) EthereumBlockNumber => Option<EthereumRelayHeaderParcel>;

		/// Confirmed Ethereum block numbers
		///
		/// The order are from small to large
		pub ConfirmedBlockNumbers
			get(fn confirmed_block_numbers)
			: Vec<EthereumBlockNumber>;

		/// The highest ethereum block number that record in darwinia
		pub BestConfirmedBlockNumber
			get(fn best_confirmed_block_number)
			: EthereumBlockNumber;

		pub ConfirmedDepth get(fn confirmed_depth) config(): u32 = 10;

		/// Dags merkle roots of ethereum epoch (each epoch is 30000)
		pub DagsMerkleRoots
			get(fn dag_merkle_root)
			: map hasher(identity) u64
			=> H128;

		pub ReceiptVerifyFee
			get(fn receipt_verify_fee)
			config()
			: RingBalance<T>;
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

			BestConfirmedBlockNumber::put(genesis_header.number);
			ConfirmedBlockNumbers::mutate(|numbers| {
				numbers.push(genesis_header.number);

				ConfirmedParcels::insert(
					genesis_header.number,
					EthereumRelayHeaderParcel {
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

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			// --- substrate ---
			use frame_support::migration::*;

			let module = b"DarwiniaEthereumRelay";
			let items: [&[u8]; 3] = [
				b"ConfirmedHeaders",
				b"ConfirmedBlockNumbers",
				b"ConfirmedDepth",
			];

			for item in &items {
				remove_storage_prefix(module, item, &[]);
			}

			// Caution: Please set the genesis header in custom runtime upgrade

			0
		}

		#[weight = 0]
		pub fn affirm(
			origin,
			ethereum_relay_header_parcel: EthereumRelayHeaderParcel,
			optional_ethereum_relay_proofs: Option<EthereumRelayProofs>
		) {
			let relayer = ensure_signed(origin)?;

			T::RelayerGame::affirm(
				relayer,
				ethereum_relay_header_parcel,
				optional_ethereum_relay_proofs
			)?;
		}

		#[weight = 0]
		pub fn dispute_and_affirm(
			origin,
			ethereum_relay_header_parcel: EthereumRelayHeaderParcel,
			optional_ethereum_relay_proofs: Option<EthereumRelayProofs>
		) {
			let relayer = ensure_signed(origin)?;

			T::RelayerGame::dispute_and_affirm(
				relayer,
				ethereum_relay_header_parcel,
				optional_ethereum_relay_proofs
			)?;
		}

		#[weight = 0]
		pub fn complete_relay_proofs(
			origin,
			affirmation_id: RelayAffirmationId<EthereumBlockNumber>,
			ethereum_relay_proofs: Vec<EthereumRelayProofs>
		) {
			ensure_signed(origin)?;

			T::RelayerGame::complete_relay_proofs(affirmation_id, ethereum_relay_proofs)?;
		}

		#[weight = 0]
		fn extend_affirmation(
			origin,
			game_sample_points: Vec<EthereumRelayHeaderParcel>,
			extended_ethereum_relay_affirmation_id: RelayAffirmationId<EthereumBlockNumber>,
			optional_ethereum_relay_proofs: Option<Vec<EthereumRelayProofs>>,
		) {
			let relayer = ensure_signed(origin)?;

			T::RelayerGame::extend_affirmation(
				relayer,
				game_sample_points,
				extended_ethereum_relay_affirmation_id,
				optional_ethereum_relay_proofs
			)?;
		}

		#[weight = 100_000_000]
		pub fn approve_pending_relay_header_parcel(origin, pending_relay_block_id: EthereumBlockNumber) {
			T::ApproveOrigin::ensure_origin(origin)?;
			T::RelayerGame::approve_pending_relay_header_parcel(pending_relay_block_id)?;
		}

		#[weight = 100_000_000]
		pub fn reject_pending_relay_header_parcel(origin, pending_relay_block_id: EthereumBlockNumber) {
			T::RejectOrigin::ensure_origin(origin)?;
			T::RelayerGame::reject_pending_relay_header_parcel(pending_relay_block_id)?;
		}

		/// Check and verify the receipt
		///
		/// `check_receipt` will verify the validation of the ethereum receipt proof from ethereum.
		/// Ethereum receipt proof are constructed with 3 parts.
		///
		/// The first part `ethereum_proof_record` is the Ethereum receipt and its merkle member proof regarding
		/// to the receipt root in related Ethereum block header.
		///
		/// The second part `ethereum_header` is the Ethereum block header which included/generated this
		/// receipt, we need to provide this as part of proof, because in Darwinia Relay, we only have
		/// last confirmed block's MMR root, don't have previous blocks, so we need to include this to
		/// provide the `receipt_root` inside it, we will need to verify validation by checking header hash.
		///
		/// The third part `mmr_proof` is the mmr proof generate according to
		/// `(member_index=[ethereum_header.number], last_index=last_confirmed_block_header.number)`
		/// it can prove that the `ethereum_header` is the chain which is committed by last confirmed block's `mmr_root`
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
		pub fn check_receipt(
			origin,
			ethereum_proof_record: EthereumReceiptProof,
			ethereum_header: EthereumHeader,
			mmr_proof: MMRProof
		) {
			let worker = ensure_signed(origin)?;
			let verified_receipt = Self::verify_receipt(&(ethereum_header.clone(), ethereum_proof_record, mmr_proof)).map_err(|_| <Error<T>>::ReceiptProofInv)?;
			let fee = Self::receipt_verify_fee();
			let module_account = Self::account_id();

			T::Currency::transfer(&worker, &module_account, fee, KeepAlive)?;

			Self::deposit_event(RawEvent::VerifyReceipt(worker, verified_receipt, ethereum_header));
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

		/// Remove the specific malicous confirmed parcel
		#[weight = 100_000_000]
		pub fn remove_confirmed_parcel_of(origin, confirmed_block_number: EthereumBlockNumber) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedBlockNumbers::mutate(|confirmed_block_numbers| {
				if let Some(i) = confirmed_block_numbers
					.iter()
					.position(|confirmed_block_number_|
						*confirmed_block_number_ == confirmed_block_number)
				{
					confirmed_block_numbers.remove(i);
				}

				ConfirmedParcels::remove(confirmed_block_number);
				BestConfirmedBlockNumber::put(confirmed_block_numbers
					.iter()
					.max()
					.map(ToOwned::to_owned)
					.unwrap_or(0));
			});

			Self::deposit_event(RawEvent::RemoveConfirmedParcel(confirmed_block_number));
		}

		// --- root call ---

		/// Caution: the genesis parcel will be removed too
		#[weight = 10_000_000]
		pub fn clean_confirmed_parcels(origin) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedParcels::remove_all();
			ConfirmedBlockNumbers::kill();
			BestConfirmedBlockNumber::kill();
		}

		#[weight = 10_000_000]
		pub fn set_confirmed_parcel(origin, ethereum_relay_header_parcel: EthereumRelayHeaderParcel) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ConfirmedBlockNumbers::mutate(|confirmed_block_numbers| {
				confirmed_block_numbers.push(ethereum_relay_header_parcel.header.number);

				BestConfirmedBlockNumber::put(confirmed_block_numbers
					.iter()
					.max()
					.map(ToOwned::to_owned)
					.unwrap_or(0));
			});
			ConfirmedParcels::insert(ethereum_relay_header_parcel.header.number, ethereum_relay_header_parcel);
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

	// TODO: more clearly error info, not just false
	pub fn verify_header(header: &EthereumHeader, ethash_proof: &[EthashProof]) -> bool {
		if header.hash() != header.re_compute_hash() {
			return false;
		}

		let ethereum_partial = Self::ethash_params();

		if ethereum_partial.verify_block_basic(header).is_err() {
			return false;
		}

		let merkle_root = Self::dag_merkle_root((header.number as usize / 30000) as u64);

		if ethereum_partial
			.verify_seal_with_proof(&header, &ethash_proof, &merkle_root)
			.is_err()
		{
			return false;
		};

		true
	}

	// TODO: more clearly error info, not just false
	/// Verify the MMR root
	///
	/// Leaves are (block_number, H256)
	/// Block number will transform to position in this function
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
	type RelayHeaderId = EthereumBlockNumber;
	type RelayHeaderParcel = EthereumRelayHeaderParcel;
	type RelayProofs = EthereumRelayProofs;

	fn best_confirmed_relay_header_id() -> Self::RelayHeaderId {
		Self::best_confirmed_block_number()
	}

	fn verify_relay_proofs(
		relay_header_id: &Self::RelayHeaderId,
		relay_header_parcel: &Self::RelayHeaderParcel,
		relay_proofs: &Self::RelayProofs,
		optional_best_confirmed_relay_header_id: Option<&Self::RelayHeaderId>,
	) -> DispatchResult {
		let Self::RelayHeaderParcel { header, mmr_root } = relay_header_parcel;
		let Self::RelayProofs {
			ethash_proof,
			mmr_proof,
		} = relay_proofs;

		ensure!(
			Self::verify_header(header, ethash_proof),
			<Error<T>>::HeaderInv
		);

		let last_leaf = *relay_header_id - 1;
		let mmr_root = array_unchecked!(mmr_root, 0, 32).into();

		if let Some(best_confirmed_block_number) = optional_best_confirmed_relay_header_id {
			let maybe_best_confirmed_block_header_hash =
				Self::confirmed_parcel_of(best_confirmed_block_number)
					.ok_or(<Error<T>>::ConfirmedHeaderNE)?
					.header
					.hash;
			let best_confirmed_block_header_hash =
				maybe_best_confirmed_block_header_hash.ok_or(<Error<T>>::HeaderHashInv)?;

			// The mmr_root of first submit should includ the hash last confirm block
			//      mmr_root of 1st
			//     / \
			//    -   -
			//   /     \
			//  c  ...  1st
			//  c: last comfirmed block 1st: 1st submit block
			ensure!(
				Self::verify_mmr(
					last_leaf,
					mmr_root,
					mmr_proof
						.iter()
						.map(|h| array_unchecked!(h, 0, 32).into())
						.collect(),
					vec![(
						*best_confirmed_block_number,
						best_confirmed_block_header_hash
					)],
				),
				<Error<T>>::MMRInv
			);
		} else {
			// last confirm no exsit the mmr verification will be passed
			//
			//      mmr_root of 1st
			//     / \
			//    - ..-
			//   /   | \
			//  -  ..c  1st
			// c: current submit  1st: 1st submit block
			ensure!(
				Self::verify_mmr(
					last_leaf,
					mmr_root,
					mmr_proof
						.iter()
						.map(|h| array_unchecked!(h, 0, 32).into())
						.collect(),
					vec![(
						header.number,
						array_unchecked!(header.hash.ok_or(<Error<T>>::HeaderInv)?, 0, 32).into(),
					)],
				),
				<Error<T>>::MMRInv
			);
		}

		Ok(())
	}

	fn verify_relay_chain(mut relay_chain: Vec<&Self::RelayHeaderParcel>) -> DispatchResult {
		let eth_partial = Self::ethash_params();
		let verify_continuous = |previous_relay_header_parcel: &EthereumRelayHeaderParcel,
		                         next_relay_header_parcel: &EthereumRelayHeaderParcel|
		 -> DispatchResult {
			ensure!(
				previous_relay_header_parcel
					.header
					.hash
					.ok_or(<Error<T>>::HeaderHashInv)?
					== next_relay_header_parcel.header.parent_hash,
				<Error<T>>::ContinuousInv
			);
			ensure!(
				next_relay_header_parcel.header.difficulty().to_owned()
					== eth_partial.calculate_difficulty(
						&next_relay_header_parcel.header,
						&previous_relay_header_parcel.header
					),
				<Error<T>>::ContinuousInv
			);

			Ok(())
		};

		relay_chain.sort_by_key(|relay_header_parcel| relay_header_parcel.header.number);

		for window in relay_chain.windows(2) {
			let previous_relay_header_parcel = window[0];
			let next_relay_header_parcel = window[1];

			verify_continuous(previous_relay_header_parcel, next_relay_header_parcel)?;
		}

		verify_continuous(
			&Self::confirmed_parcel_of(T::RelayerGame::best_confirmed_header_id_of(&0))
				.ok_or(<Error<T>>::ConfirmedHeaderNE)?,
			*relay_chain.get(0).ok_or(<Error<T>>::ContinuousInv)?,
		)?;

		Ok(())
	}

	fn distance_between(
		relay_header_id: &Self::RelayHeaderId,
		best_confirmed_relay_header_id: Self::RelayHeaderId,
	) -> u32 {
		relay_header_id
			.checked_sub(best_confirmed_relay_header_id)
			.map(|distance| distance as u32)
			.unwrap_or(0)
	}

	fn store_relay_header_parcel(relay_header_parcel: Self::RelayHeaderParcel) -> DispatchResult {
		let best_confirmed_block_number = Self::best_confirmed_block_number();
		let relay_block_number = relay_header_parcel.header.number;

		// Not allow to relay genesis header
		ensure!(
			relay_block_number > best_confirmed_block_number,
			<Error<T>>::HeaderInv
		);

		ConfirmedBlockNumbers::mutate(|confirmed_block_numbers| {
			// TODO: remove old numbers according to `ConfirmedDepth`

			confirmed_block_numbers.push(relay_block_number);

			BestConfirmedBlockNumber::put(relay_block_number);
		});
		ConfirmedParcels::insert(relay_block_number, relay_header_parcel);

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
		ethereum_receipt_proof_thing: &Self::EthereumReceiptProofThing,
	) -> Result<EthereumReceipt, DispatchError> {
		// Verify header hash
		let (ethereum_header, ethereum_proof_record, mmr_proof) = ethereum_receipt_proof_thing;
		let header_hash = ethereum_header.hash();

		ensure!(
			header_hash == ethereum_header.re_compute_hash(),
			<Error<T>>::HeaderHashMis,
		);
		ensure!(
			ethereum_header.number == mmr_proof.member_leaf_index,
			<Error<T>>::MMRInv,
		);

		// Verify header member to last confirmed block using mmr proof
		let mmr_root = Self::confirmed_parcel_of(mmr_proof.last_leaf_index + 1)
			.ok_or(<Error<T>>::ConfirmedHeaderNE)?
			.mmr_root;

		ensure!(
			Self::verify_mmr(
				mmr_proof.last_leaf_index,
				mmr_root,
				mmr_proof.proof.to_vec(),
				vec![(
					ethereum_header.number,
					array_unchecked!(
						ethereum_header.hash.ok_or(<Error<T>>::HeaderHashInv)?,
						0,
						32
					)
					.into(),
				)]
			),
			<Error<T>>::MMRInv
		);

		// Verify receipt proof
		let receipt = EthereumReceipt::verify_proof_and_generate(
			ethereum_header.receipts_root(),
			&ethereum_proof_record,
		)
		.map_err(|_| <Error<T>>::ReceiptProofInv)?;

		Ok(receipt)
	}

	fn gen_receipt_index(proof: &Self::EthereumReceiptProofThing) -> EthereumTransactionIndex {
		let (_, ethereum_receipt_proof, _) = proof;

		(
			ethereum_receipt_proof.header_hash,
			ethereum_receipt_proof.index,
		)
	}
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct EthereumRelayHeaderParcel {
	header: EthereumHeader,
	mmr_root: H256,
}
impl RelayHeaderParcelInfo for EthereumRelayHeaderParcel {
	type HeaderId = EthereumBlockNumber;

	fn header_id(&self) -> Self::HeaderId {
		self.header.number
	}
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct EthereumRelayProofs {
	ethash_proof: Vec<EthashProof>,
	mmr_proof: Vec<H256>,
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
		_call: &Self::Call,
		_: &DispatchInfoOf<Self::Call>,
		_: usize,
	) -> TransactionValidity {
		// TODO: pre-verify
		// if let Some(Call::submit_proposal(ref proposal)) = call.is_sub_type() {
		// 	if let Some(proposed_header_thing) = proposal.get(0) {
		// 		for existed_proposal in
		// 			T::RelayerGame::proposals_of_game(proposed_header_thing.header.number)
		// 		{
		// 			if existed_proposal
		// 				.bonded_proposal
		// 				.iter()
		// 				.zip(proposal.iter())
		// 				.all(
		// 					|(
		// 						(
		// 							_,
		// 							EthereumHeaderThing {
		// 								header: header_a,
		// 								mmr_root: mmr_root_a,
		// 							},
		// 						),
		// 						EthereumHeaderThingWithProof {
		// 							header: header_b,
		// 							mmr_root: mmr_root_b,
		// 							..
		// 						},
		// 					)| header_a == header_b && mmr_root_a == mmr_root_b,
		// 				) {
		// 				return InvalidTransaction::Custom(<Error<T>>::ProposalInv.as_u8()).into();
		// 			}
		// 		}
		// 	}
		// }

		Ok(ValidTransaction::default())
	}
}
