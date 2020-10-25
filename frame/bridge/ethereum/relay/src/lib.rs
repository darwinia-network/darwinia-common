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
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;

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
	traits::{
		ChangeMembers, Contains, Currency, EnsureOrigin, ExistenceRequirement::KeepAlive, Get,
		ReservableCurrency,
	},
	unsigned::{TransactionValidity, TransactionValidityError},
	weights::Weight,
	IsSubType,
};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{AccountIdConversion, DispatchInfoOf, Dispatchable, SignedExtension, Zero},
	transaction_validity::ValidTransaction,
	DispatchError, DispatchResult, ModuleId, Perbill, RuntimeDebug,
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

	type Currency: LockableCurrency<AccountId<Self>, Moment = Self::BlockNumber>
		+ ReservableCurrency<AccountId<Self>>;

	type RelayerGame: RelayerGameProtocol<
		Relayer = AccountId<Self>,
		RelayHeaderId = EthereumBlockNumber,
		RelayHeaderParcel = EthereumRelayHeaderParcel,
		RelayProofs = EthereumRelayProofs,
	>;

	type ApproveOrigin: EnsureOrigin<Self::Origin>;

	type RejectOrigin: EnsureOrigin<Self::Origin>;

	/// The comfirm period for guard
	///
	/// Tech.Comm. can vote for the pending header within this period
	/// If not enough Tech.Comm. votes for the pending header it will be confirmed
	/// automatically after this period
	type ConfirmPeriod: Get<Self::BlockNumber>;

	type TechnicalMembership: Contains<AccountId<Self>>;

	type ApproveThreshold: Get<Perbill>;

	type RejectThreshold: Get<Perbill>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
		RelayAffirmationId = RelayAffirmationId<EthereumBlockNumber>,
	{
		/// A new relay header parcel affirmed. [relayer, relay affirmation id]
		Affirmed(AccountId, RelayAffirmationId),
		/// A different affirmation submitted, dispute found. [relayer, relay affirmation id]
		DisputedAndAffirmed(AccountId, RelayAffirmationId),
		/// An extended affirmation submitted, dispute go on. [relayer, relay affirmation id]
		Extended(AccountId, RelayAffirmationId),
		/// A new round started. [game id, game sample points]
		NewRound(EthereumBlockNumber, Vec<EthereumBlockNumber>),
		/// A game has been settled. [game id]
		GameOver(EthereumBlockNumber),
		/// The specific confirmed parcel removed. [block number]
		RemoveConfirmedParcel(EthereumBlockNumber),
		/// EthereumReceipt verification. [account, ethereum receipt, ethereum header]
		VerifyReceipt(AccountId, EthereumReceipt, EthereumHeader),
		/// A relay header parcel got pended. [block number]
		Pended(EthereumBlockNumber),
		/// Pending relay header parcel approved. [block number, reason]
		PendingRelayHeaderParcelApproved(EthereumBlockNumber, Vec<u8>),
		/// Pending relay header parcel rejected. [block number]
		PendingRelayHeaderParcelRejected(EthereumBlockNumber),
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
		// /// Affirmation - INVALID
		// AffirmationInv,
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
		/// Pending Relay Parcel - NOT EXISTED
		PendingRelayParcelNE,
		/// Already Vote for Aye - DUPLICATED
		AlreadyVoteForAyeDup,
		/// Already Vote for Nay - DUPLICATED
		AlreadyVoteForNayDup,
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
		/// Confirmed ethereum header parcel
		pub ConfirmedHeaderParcels
			get(fn confirmed_header_parcel_of)
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

		pub PendingRelayHeaderParcels
			get(fn pending_relay_header_parcels)
			: Vec<(BlockNumber<T>, EthereumRelayHeaderParcel, RelayVotingState<AccountId<T>>)>;
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

				ConfirmedHeaderParcels::insert(
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

		fn on_initialize(now: BlockNumber<T>) -> Weight {
			// TODO: handle error
			// TODO: weight
			Self::system_approve_pending_relay_header_parcels(now).unwrap_or(0)
		}

		#[weight = 0]
		pub fn affirm(
			origin,
			ethereum_relay_header_parcel: EthereumRelayHeaderParcel,
			optional_ethereum_relay_proofs: Option<EthereumRelayProofs>
		) {
			let relayer = ensure_signed(origin)?;
			let game_id = T::RelayerGame::affirm(
				&relayer,
				ethereum_relay_header_parcel,
				optional_ethereum_relay_proofs
			)?;

			Self::deposit_event(RawEvent::Affirmed(
				relayer,
				RelayAffirmationId { game_id, round: 0, index: 0 }
			));
		}

		#[weight = 0]
		pub fn dispute_and_affirm(
			origin,
			ethereum_relay_header_parcel: EthereumRelayHeaderParcel,
			optional_ethereum_relay_proofs: Option<EthereumRelayProofs>
		) {
			let relayer = ensure_signed(origin)?;
			let (game_id, index) = T::RelayerGame::dispute_and_affirm(
				&relayer,
				ethereum_relay_header_parcel,
				optional_ethereum_relay_proofs
			)?;

			Self::deposit_event(RawEvent::DisputedAndAffirmed(
				relayer,
				RelayAffirmationId { game_id, round: 0, index }
			));
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
			extended_ethereum_relay_affirmation_id: RelayAffirmationId<EthereumBlockNumber>,
			game_sample_points: Vec<EthereumRelayHeaderParcel>,
			optional_ethereum_relay_proofs: Option<Vec<EthereumRelayProofs>>,
		) {
			let relayer = ensure_signed(origin)?;
			let (game_id, round, index) = T::RelayerGame::extend_affirmation(
				&relayer,
				extended_ethereum_relay_affirmation_id,
				game_sample_points,
				optional_ethereum_relay_proofs
			)?;

			Self::deposit_event(RawEvent::Extended(
				relayer,
				RelayAffirmationId { game_id, round, index }
			));
		}

		#[weight = 100_000_000]
		pub fn vote_pending_relay_header_parcel(
			origin,
			block_number: EthereumBlockNumber,
			aye: bool
		) {
			let technical_member = {
				let account_id = ensure_signed(origin)?;

				if T::TechnicalMembership::contains(&account_id) {
					account_id
				} else {
					Err(DispatchError::BadOrigin)?
				}
			};
			let res: Result<(), DispatchError> = <PendingRelayHeaderParcels<T>>::try_mutate(|pending_relay_header_parcels| {
				if let Some(i) =
					pending_relay_header_parcels
						.iter()
						.position(|(_, relay_header_parcel, _)| {
							relay_header_parcel.header.number == block_number
						}) {
					let (
						_,
						_,
						RelayVotingState { ayes, nays }
					) =	&mut pending_relay_header_parcels[i];

					if aye {
						if ayes.contains(&technical_member) {
							Err(<Error<T>>::AlreadyVoteForAyeDup)?;
						} else {
							if let Some(i) = nays.iter().position(|nay| nay == &technical_member) {
								nays.remove(i);
							}

							ayes.push(technical_member);
						}
					} else {
						if nays.contains(&technical_member) {
							Err(<Error<T>>::AlreadyVoteForNayDup)?;
						} else {
							if let Some(i) = ayes.iter().position(|aye| aye == &technical_member) {
								ayes.remove(i);
							}

							nays.push(technical_member);
						}
					}

					let approve = ayes.len() as u32;
					let reject = nays.len() as u32;
					let total = T::TechnicalMembership::count() as u32;
					let approve_threashold =
						Perbill::from_rational_approximation(approve, total);
					let reject_threashold =
						Perbill::from_rational_approximation(reject, total);

					if approve_threashold >= T::ApproveThreshold::get() {
						Self::store_relay_header_parcel(
							pending_relay_header_parcels.remove(i).1
						)?;

						Self::deposit_event(RawEvent::PendingRelayHeaderParcelApproved(
							block_number,
							b"Approved By Tech.Comm".to_vec(),
						));
					} else if reject_threashold >= T::RejectThreshold::get() {
						pending_relay_header_parcels.remove(i);

						Self::deposit_event(RawEvent::PendingRelayHeaderParcelRejected(
							block_number,
						));
					}

					Ok(())
				} else {
					Err(<Error<T>>::PendingRelayParcelNE)?
				}
			});
			res?;
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

				ConfirmedHeaderParcels::remove(confirmed_block_number);
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

			ConfirmedHeaderParcels::remove_all();
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
			ConfirmedHeaderParcels::insert(ethereum_relay_header_parcel.header.number, ethereum_relay_header_parcel);
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

	pub fn system_approve_pending_relay_header_parcels(
		now: BlockNumber<T>,
	) -> Result<Weight, DispatchError> {
		<PendingRelayHeaderParcels<T>>::mutate(|pending_relay_header_parcels| {
			pending_relay_header_parcels.retain(|(confirm_at, pending_relay_header_parcel, _)| {
				if *confirm_at == now {
					// TODO: handle error
					let _ = Self::store_relay_header_parcel(pending_relay_header_parcel.to_owned());

					Self::deposit_event(RawEvent::PendingRelayHeaderParcelApproved(
						pending_relay_header_parcel.header.number,
						b"Not Enough Technical Member Online, Approved By System".to_vec(),
					));

					false
				} else {
					true
				}
			})
		});

		// TODO: weight
		Ok(0)
	}

	pub fn migrate_genesis(use_ethereum_mainnet: bool) {
		// --- substrate ---
		use frame_support::migration::*;

		let module = b"DarwiniaEthereumRelay";

		{
			let items: [&[u8]; 2] = [b"ConfirmedHeaders", b"ConfirmedBlockNumbers"];

			for item in &items {
				remove_storage_prefix(module, item, &[]);
			}
		}

		let genesis_header = if use_ethereum_mainnet {
			let raw_genesis_header = vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131,
				69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181,
				227, 99, 180, 33, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182,
				204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147,
				71, 128, 17, 187, 232, 219, 78, 52, 123, 78, 140, 147, 124, 28, 131, 112, 228, 181,
				237, 51, 173, 179, 219, 105, 203, 219, 122, 56, 225, 229, 11, 27, 130, 250, 215,
				248, 151, 79, 181, 172, 120, 217, 172, 9, 155, 154, 213, 1, 139, 237, 194, 206, 10,
				114, 218, 209, 130, 122, 23, 9, 218, 48, 88, 15, 5, 68, 86, 232, 31, 23, 27, 204,
				85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173,
				192, 1, 98, 47, 181, 227, 99, 180, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136, 19, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 8, 132, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 136, 0, 0, 0, 0, 0, 0, 0, 66, 1, 212, 229,
				103, 64, 248, 118, 174, 248, 192, 16, 184, 106, 64, 213, 245, 103, 69, 161, 24,
				208, 144, 106, 52, 230, 154, 236, 140, 13, 177, 203, 143, 163,
			];

			EthereumHeader::decode(&mut &*raw_genesis_header.to_vec()).unwrap()
		} else {
			let raw_genesis_header = vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131,
				69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181,
				227, 99, 180, 33, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182,
				204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147,
				71, 128, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53,
				53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 53, 33, 123, 11, 188, 251, 114,
				226, 213, 126, 40, 243, 60, 179, 97, 185, 152, 53, 19, 23, 119, 85, 220, 63, 51,
				206, 62, 112, 34, 237, 98, 183, 123, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131,
				69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181,
				227, 99, 180, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 132, 160,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 36, 136, 0, 0, 0, 0, 0, 0, 0, 66, 1, 65, 148, 16, 35, 104, 9, 35, 224,
				254, 77, 116, 163, 75, 218, 200, 20, 31, 37, 64, 227, 174, 144, 98, 55, 24, 228,
				125, 102, 209, 202, 74, 45,
			];

			EthereumHeader::decode(&mut &*raw_genesis_header.to_vec()).unwrap()
		};
		let genesis_header_mmr_root: H256 = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".into();

		put_storage_value(
			module,
			b"BestConfirmedBlockNumber",
			&[],
			genesis_header.number,
		);
		put_storage_value(
			module,
			b"ConfirmedBlockNumbers",
			&[],
			vec![genesis_header.number],
		);
		put_storage_value(
			module,
			b"ConfirmedHeaderParcels",
			&genesis_header.number.to_ne_bytes(),
			EthereumRelayHeaderParcel {
				header: genesis_header,
				mmr_root: genesis_header_mmr_root,
			},
		);
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
				Self::confirmed_header_parcel_of(best_confirmed_block_number)
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
			&Self::confirmed_header_parcel_of(T::RelayerGame::best_confirmed_header_id_of(&0))
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
		let confirm_period = T::ConfirmPeriod::get();
		let best_confirmed_block_number = Self::best_confirmed_block_number();
		let relay_block_number = relay_header_parcel.header.number;

		// Not allow to relay genesis header
		ensure!(
			relay_block_number > best_confirmed_block_number,
			<Error<T>>::HeaderInv
		);

		if confirm_period.is_zero() {
			ConfirmedBlockNumbers::mutate(|confirmed_block_numbers| {
				// TODO: remove old numbers according to `ConfirmedDepth`

				confirmed_block_numbers.push(relay_block_number);

				BestConfirmedBlockNumber::put(relay_block_number);
			});
			ConfirmedHeaderParcels::insert(relay_block_number, relay_header_parcel);
		} else {
			<PendingRelayHeaderParcels<T>>::append((
				<frame_system::Module<T>>::block_number() + confirm_period,
				relay_header_parcel,
				RelayVotingState::default(),
			));

			Self::deposit_event(RawEvent::Pended(relay_block_number));
		}

		Ok(())
	}

	fn new_round(game_id: &Self::RelayHeaderId, game_sample_points: Vec<Self::RelayHeaderId>) {
		Self::deposit_event(RawEvent::NewRound(*game_id, game_sample_points));
	}

	fn game_over(game_id: &Self::RelayHeaderId) {
		Self::deposit_event(RawEvent::GameOver(*game_id));
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
		let mmr_root = Self::confirmed_header_parcel_of(mmr_proof.last_leaf_index + 1)
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
	pub header: EthereumHeader,
	pub mmr_root: H256,
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
	pub ethash_proof: Vec<EthashProof>,
	pub mmr_proof: Vec<H256>,
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct MMRProof {
	pub member_leaf_index: u64,
	pub last_leaf_index: u64,
	pub proof: Vec<H256>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct CheckEthereumRelayHeaderParcel<T: Trait>(PhantomData<T>);
impl<T: Trait> CheckEthereumRelayHeaderParcel<T> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}
impl<T: Trait> Debug for CheckEthereumRelayHeaderParcel<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "CheckEthereumRelayHeaderParcel")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut Formatter) -> FmtResult {
		Ok(())
	}
}
impl<T: Send + Sync + Trait> SignedExtension for CheckEthereumRelayHeaderParcel<T> {
	const IDENTIFIER: &'static str = "CheckEthereumRelayHeaderParcel";
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
		// 				return InvalidTransaction::Custom(<Error<T>>::AffirmationInv.as_u8()).into();
		// 			}
		// 		}
		// 	}
		// }

		Ok(ValidTransaction::default())
	}
}

impl<T: Trait> ChangeMembers<AccountId<T>> for Module<T> {
	fn change_members_sorted(_: &[T::AccountId], outgoing: &[T::AccountId], _: &[T::AccountId]) {
		let _ = <PendingRelayHeaderParcels<T>>::try_mutate(|pending_relay_header_parcels| {
			let mut changed = false;

			for (_, _, RelayVotingState { ayes, nays }) in pending_relay_header_parcels {
				for removed_member in outgoing {
					if let Some(i) = ayes.iter().position(|aye| aye == removed_member) {
						changed = true;

						ayes.remove(i);
					} else if let Some(i) = nays.iter().position(|nay| nay == removed_member) {
						changed = true;

						nays.remove(i);
					}
				}
			}

			if changed {
				Ok(())
			} else {
				Err(())
			}
		});
	}

	// TODO: if someone give up
	fn set_prime(_: Option<T::AccountId>) {}
}
