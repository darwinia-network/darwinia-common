//! # Treasury Module
//!
//! The Treasury module provides a "pot" of funds that can be managed by stakeholders in the
//! system and a structure for making spending proposals from this pot.
//!
//! - [`treasury::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! The Treasury Module itself provides the pot to store funds, and a means for stakeholders to
//! propose, approve, and deny expenditures. The chain will need to provide a method (e.g.
//! inflation, fees) for collecting funds.
//!
//! By way of example, the Council could vote to fund the Treasury with a portion of the block
//! reward and use the funds to pay developers.
//!
//! ### Tipping
//!
//! A separate subsystem exists to allow for an agile "tipping" process, whereby a reward may be
//! given without first having a pre-determined stakeholder group come to consensus on how much
//! should be paid.
//!
//! A group of `Tippers` is determined through the config `Trait`. After half of these have declared
//! some amount that they believe a particular reported reason deserves, then a countdown period is
//! entered where any remaining members can declare their tip amounts also. After the close of the
//! countdown period, the median of all declared tips is paid to the reported beneficiary, along
//! with any finders fee, in case of a public (and bonded) original report.
//!
//! ### Terminology
//!
//! - **Proposal:** A suggestion to allocate funds from the pot to a beneficiary.
//! - **Beneficiary:** An account who will receive the funds from a proposal iff
//! the proposal is approved.
//! - **Deposit:** Funds that a proposer must lock when making a proposal. The
//! deposit will be returned or slashed if the proposal is approved or rejected
//! respectively.
//! - **Pot:** Unspent funds accumulated by the treasury module.
//!
//! Tipping protocol:
//! - **Tipping:** The process of gathering declarations of amounts to tip and taking the median
//!   amount to be transferred from the treasury to a beneficiary account.
//! - **Tip Reason:** The reason for a tip; generally a URL which embodies or explains why a
//!   particular individual (identified by an account ID) is worthy of a recognition by the
//!   treasury.
//! - **Finder:** The original public reporter of some reason for tipping.
//! - **Finders Fee:** Some proportion of the tip amount that is paid to the reporter of the tip,
//!   rather than the main beneficiary.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! General spending/proposal protocol:
//! - `propose_spend` - Make a spending proposal and stake the required deposit.
//! - `set_pot` - Set the spendable balance of funds.
//! - `configure` - Configure the module's proposal requirements.
//! - `reject_proposal` - Reject a proposal, slashing the deposit.
//! - `approve_proposal` - Accept the proposal, returning the deposit.
//!
//! Tipping protocol:
//! - `report_awesome` - Report something worthy of a tip and register for a finders fee.
//! - `retract_tip` - Retract a previous (finders fee registered) report.
//! - `tip_new` - Report an item worthy of a tip and declare a specific amount to tip.
//! - `tip` - Declare or redeclare an amount to tip for a particular reason.
//! - `close_tip` - Close and pay out a tip.
//!
//! ## GenesisConfig
//!
//! The Treasury module depends on the [`GenesisConfig`](./struct.GenesisConfig.html).

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	/// An index of a proposal. Just a `u32`.
	pub type ProposalIndex = u32;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type RingPositiveImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type RingNegativeImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;
	pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type KtonPositiveImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type KtonNegativeImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

	type AccountId<T> = <T as system::Trait>::AccountId;
	type RingCurrency<T> = <T as Trait>::RingCurrency;
	type KtonCurrency<T> = <T as Trait>::KtonCurrency;
}

// --- crates ---
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, print,
	traits::{
		Contains, ContainsLengthBound, Currency, EnsureOrigin, ExistenceRequirement::KeepAlive,
		Get, Imbalance, OnUnbalanced, ReservableCurrency, WithdrawReason,
	},
	weights::{DispatchClass, Weight},
	Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, BadOrigin, Hash, Saturating, StaticLookup, Zero,
	},
	ModuleId, Percent, Permill, RuntimeDebug,
};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::balance::{lock::LockableCurrency, OnUnbalancedKton};
use types::*;

pub trait Trait: frame_system::Trait {
	/// The treasury's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	/// The staking *RING*.
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
		+ ReservableCurrency<Self::AccountId>;

	/// The staking *KTON*.
	type KtonCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
		+ ReservableCurrency<Self::AccountId>;

	/// Origin from which approvals must come.
	type ApproveOrigin: EnsureOrigin<Self::Origin>;

	/// Origin from which rejections must come.
	type RejectOrigin: EnsureOrigin<Self::Origin>;

	/// Origin from which tippers must come.
	///
	/// `ContainsLengthBound::max_len` must be cost free (i.e. no storage read or heavy operation).
	type Tippers: Contains<Self::AccountId> + ContainsLengthBound;

	/// The period for which a tip remains open after is has achieved threshold tippers.
	type TipCountdown: Get<Self::BlockNumber>;

	/// The percent of the final tip which goes to the original reporter of the tip.
	type TipFindersFee: Get<Percent>;

	/// The amount held on deposit for placing a tip report.
	type TipReportDepositBase: Get<RingBalance<Self>>;

	/// The amount held on deposit per byte within the tip report reason.
	type TipReportDepositPerByte: Get<RingBalance<Self>>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// Handler for the unbalanced decrease when slashing for a rejected proposal.
	type RingProposalRejection: OnUnbalanced<RingNegativeImbalance<Self>>;
	/// Handler for the unbalanced decrease when slashing for a rejected proposal.
	type KtonProposalRejection: OnUnbalancedKton<KtonNegativeImbalance<Self>>;

	/// Fraction of a proposal's value that should be bonded in order to place the proposal.
	/// An accepted proposal gets these back. A rejected proposal does not.
	type ProposalBond: Get<Permill>;

	/// Minimum amount of *RING* that should be placed in a deposit for making a proposal.
	type RingProposalBondMinimum: Get<RingBalance<Self>>;
	/// Minimum amount of *KTON* that should be placed in a deposit for making a proposal.
	type KtonProposalBondMinimum: Get<KtonBalance<Self>>;

	/// Period between successive spends.
	type SpendPeriod: Get<Self::BlockNumber>;

	/// Percentage of spare funds (if any) that are burnt per spend period.
	type Burn: Get<Permill>;
}

/// A spending proposal.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Proposal<AccountId, RingBalance, KtonBalance> {
	/// The account proposing it.
	proposer: AccountId,
	/// The account to whom the payment should be made if the proposal is accepted.
	beneficiary: AccountId,
	/// The (total) *RING* that should be paid if the proposal is accepted.
	ring_value: RingBalance,
	/// The (total) *KTON* that should be paid if the proposal is accepted.
	kton_value: KtonBalance,
	/// The *RING* held on deposit (reserved) for making this proposal.
	ring_bond: RingBalance,
	/// The *KTON* held on deposit (reserved) for making this proposal.
	kton_bond: KtonBalance,
}

/// An open tipping "motion". Retains all details of a tip including information on the finder
/// and the members who have voted.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct OpenTip<
	AccountId: Parameter,
	RingBalance: Parameter,
	BlockNumber: Parameter,
	Hash: Parameter,
> {
	/// The hash of the reason for the tip. The reason should be a human-readable UTF-8 encoded string. A URL would be
	/// sensible.
	reason: Hash,
	/// The account to be tipped.
	who: AccountId,
	/// The account who began this tip.
	finder: AccountId,
	/// The amount held on deposit for this tip.
	deposit: RingBalance,
	/// The block number at which this tip will close if `Some`. If `None`, then no closing is
	/// scheduled.
	closes: Option<BlockNumber>,
	/// The members who have voted for this tip. Sorted by AccountId.
	tips: Vec<(AccountId, RingBalance)>,
	/// Whether this tip should result in the finder taking a fee.
	finders_fee: bool,
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaTreasury {
		/// Number of proposals that have been made.
		ProposalCount get(fn proposal_count): ProposalIndex;

		/// Proposals that have been made.
		Proposals get(fn proposals): map hasher(twox_64_concat) ProposalIndex => Option<Proposal<T::AccountId, RingBalance<T>, KtonBalance<T>>>;

		/// Proposal indices that have been approved but not yet awarded.
		Approvals get(fn approvals): Vec<ProposalIndex>;

		/// Tips that are not yet completed. Keyed by the hash of `(reason, who)` from the value.
		/// This has the insecure enumerable hash function since the key itself is already
		/// guaranteed to be a secure hash.
		pub Tips get(fn tips): map hasher(twox_64_concat) T::Hash => Option<OpenTip<T::AccountId, RingBalance<T>, T::BlockNumber, T::Hash>>;

		/// Simple preimage lookup from the reason's hash to the original data. Again, has an
		/// insecure enumerable hash since the key is guaranteed to be the result of a secure hash.
		pub Reasons get(fn reasons): map hasher(identity) T::Hash => Option<Vec<u8>>;
	}
	add_extra_genesis {
		build(|_config| {
			// Create Treasury account
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance(),
			);

			let _ = T::KtonCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::KtonCurrency::minimum_balance(),
			);
		});
	}
}

decl_event!(
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
		<T as frame_system::Trait>::Hash,
		RingBalance = RingBalance<T>,
		KtonBalance = KtonBalance<T>,
	{
		/// New proposal.
		Proposed(ProposalIndex),
		/// We have ended a spend period and will now allocate funds.
		Spending(RingBalance, KtonBalance),
		/// Some funds have been allocated.
		Awarded(ProposalIndex, RingBalance, KtonBalance, AccountId),
		/// A proposal was rejected; funds were slashed.
		Rejected(ProposalIndex, RingBalance, KtonBalance),
		/// Some of our funds have been burnt.
		Burnt(RingBalance, KtonBalance),
		/// Spending has finished; this is the amount that rolls over until next spend.
		Rollover(RingBalance, KtonBalance),
		/// Some *RING* have been deposited.
		DepositRing(RingBalance),
		/// Some *KTON* have been deposited.
		DepositKton(KtonBalance),
		/// A new tip suggestion has been opened.
		NewTip(Hash),
		/// A tip suggestion has reached threshold and is closing.
		TipClosing(Hash),
		/// A tip suggestion has been closed.
		TipClosed(Hash, AccountId, RingBalance),
		/// A tip suggestion has been retracted.
		TipRetracted(Hash),
	}
);

decl_error! {
	/// Error for the treasury module.
	pub enum Error for Module<T: Trait> {
		/// Proposer's balance is too low.
		InsufficientProposersBalance,
		/// No proposal at that index.
		InvalidProposalIndex,
		/// The reason given is just too big.
		ReasonTooBig,
		/// The tip was already found/started.
		AlreadyKnown,
		/// The tip hash is unknown.
		UnknownTip,
		/// The account attempting to retract the tip is not the finder of the tip.
		NotFinder,
		/// The tip cannot be claimed/closed because there are not enough tippers yet.
		StillOpen,
		/// The tip cannot be claimed/closed because it's still in the countdown period.
		Premature,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		const ProposalBond: Permill = T::ProposalBond::get();

		/// Minimum amount of *RING* that should be placed in a deposit for making a proposal.
		const RingProposalBondMinimum: RingBalance<T> = T::RingProposalBondMinimum::get();
		/// Minimum amount of *KTON* that should be placed in a deposit for making a proposal.
		const KtonProposalBondMinimum: KtonBalance<T> = T::KtonProposalBondMinimum::get();

		/// Period between successive spends.
		const SpendPeriod: T::BlockNumber = T::SpendPeriod::get();

		/// Percentage of spare funds (if any) that are burnt per spend period.
		const Burn: Permill = T::Burn::get();

		/// The period for which a tip remains open after is has achieved threshold tippers.
		const TipCountdown: T::BlockNumber = T::TipCountdown::get();

		/// The amount of the final tip which goes to the original reporter of the tip.
		const TipFindersFee: Percent = T::TipFindersFee::get();

		/// The amount held on deposit for placing a tip report.
		const TipReportDepositBase: RingBalance<T> = T::TipReportDepositBase::get();

		/// The amount held on deposit per byte within the tip report reason.
		const TipReportDepositPerByte: RingBalance<T> = T::TipReportDepositPerByte::get();

		/// The treasury's module id, used for deriving its sovereign account ID.
		const ModuleId: ModuleId = T::ModuleId::get();

		fn deposit_event() = default;

		/// Put forward a suggestion for spending. A deposit proportional to the value
		/// is reserved and slashed if the proposal is rejected. It is returned once the
		/// proposal is awarded.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - DbReads: `ProposalCount`, `origin account`
		/// - DbWrites: `ProposalCount`, `Proposals`, `origin account`
		/// # </weight>
		#[weight = 120_000_000 + T::DbWeight::get().reads_writes(1, 2)]
		fn propose_spend(
			origin,
			#[compact] ring_value: RingBalance<T>,
			#[compact] kton_value: KtonBalance<T>,
			beneficiary: <T::Lookup as StaticLookup>::Source
		) {
			let proposer = ensure_signed(origin)?;
			let beneficiary = T::Lookup::lookup(beneficiary)?;

			let ring_bond = Self::calculate_bond::<_, T::RingProposalBondMinimum>(ring_value);
			let kton_bond = Self::calculate_bond::<_, T::KtonProposalBondMinimum>(kton_value);

			T::RingCurrency::reserve(&proposer, ring_bond)
				.map_err(|_| <Error<T>>::InsufficientProposersBalance)?;
			T::KtonCurrency::reserve(&proposer, kton_bond)
				.map_err(|_| <Error<T>>::InsufficientProposersBalance)?;

			let c = Self::proposal_count();
			ProposalCount::put(c + 1);
			<Proposals<T>>::insert(c, Proposal {
				proposer,
				beneficiary,
				ring_value,
				ring_bond,
				kton_value,
				kton_bond,
			});

			Self::deposit_event(RawEvent::Proposed(c));
		}

		/// Reject a proposed spend. The original deposit will be slashed.
		///
		/// May only be called from `T::RejectOrigin`.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - DbReads: `Proposals`, `rejected proposer account`
		/// - DbWrites: `Proposals`, `rejected proposer account`
		/// # </weight>
		#[weight = (130_000_000 + T::DbWeight::get().reads_writes(2, 2), DispatchClass::Operational)]
		fn reject_proposal(origin, #[compact] proposal_id: ProposalIndex) {
			T::RejectOrigin::ensure_origin(origin)?;

			let proposal = <Proposals<T>>::take(&proposal_id).ok_or(<Error<T>>::InvalidProposalIndex)?;

			let ring_bond = proposal.ring_bond;
			let imbalance_ring = T::RingCurrency::slash_reserved(&proposal.proposer, ring_bond).0;
			T::RingProposalRejection::on_unbalanced(imbalance_ring);

			let kton_bond = proposal.kton_bond;
			let imbalance_kton = T::KtonCurrency::slash_reserved(&proposal.proposer, kton_bond).0;
			T::KtonProposalRejection::on_unbalanced(imbalance_kton);

			Self::deposit_event(<Event<T>>::Rejected(proposal_id, ring_bond, kton_bond));
		}

		/// Approve a proposal. At a later time, the proposal will be allocated to the beneficiary
		/// and the original deposit will be returned.
		///
		/// May only be called from `T::RejectOrigin`.
		///
		/// # <weight>
		/// - Complexity: O(1).
		/// - DbReads: `Proposals`, `Approvals`
		/// - DbWrite: `Approvals`
		/// # </weight>
		#[weight = (34_000_000 + T::DbWeight::get().reads_writes(2, 1), DispatchClass::Operational)]
		fn approve_proposal(origin, #[compact] proposal_id: ProposalIndex) {
			T::ApproveOrigin::ensure_origin(origin)?;

			ensure!(<Proposals<T>>::contains_key(proposal_id), <Error<T>>::InvalidProposalIndex);
			Approvals::mutate(|v| v.push(proposal_id));
		}

		/// Report something `reason` that deserves a tip and claim any eventual the finder's fee.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// Payment: `TipReportDepositBase` will be reserved from the origin account, as well as
		/// `TipReportDepositPerByte` for each byte in `reason`.
		///
		/// - `reason`: The reason for, or the thing that deserves, the tip; generally this will be
		///   a UTF-8-encoded URL.
		/// - `who`: The account which should be credited for the tip.
		///
		/// Emits `NewTip` if successful.
		///
		/// # <weight>
		/// - Complexity: `O(R)` where `R` length of `reason`.
		///   - encoding and hashing of 'reason'
		/// - DbReads: `Reasons`, `Tips`, `who account data`
		/// - DbWrites: `Tips`, `who account data`
		/// # </weight>
		#[weight = 140_000_000 + 4_000 * reason.len() as Weight + T::DbWeight::get().reads_writes(3, 2)]
		fn report_awesome(origin, reason: Vec<u8>, who: T::AccountId) {
			let finder = ensure_signed(origin)?;

			const MAX_SENSIBLE_REASON_LENGTH: usize = 16384;
			ensure!(reason.len() <= MAX_SENSIBLE_REASON_LENGTH, <Error<T>>::ReasonTooBig);

			let reason_hash = T::Hashing::hash(&reason[..]);
			ensure!(!<Reasons<T>>::contains_key(&reason_hash), <Error<T>>::AlreadyKnown);
			let hash = T::Hashing::hash_of(&(&reason_hash, &who));
			ensure!(!<Tips<T>>::contains_key(&hash), <Error<T>>::AlreadyKnown);

			let deposit = T::TipReportDepositBase::get()
				+ T::TipReportDepositPerByte::get() * (reason.len() as u32).into();
			T::RingCurrency::reserve(&finder, deposit)?;

			<Reasons<T>>::insert(&reason_hash, &reason);
			let tip = OpenTip {
				reason: reason_hash,
				who,
				finder,
				deposit,
				closes: None,
				tips: vec![],
				finders_fee: true
			};
			<Tips<T>>::insert(&hash, tip);
			Self::deposit_event(RawEvent::NewTip(hash));
		}

		/// Retract a prior tip-report from `report_awesome`, and cancel the process of tipping.
		///
		/// If successful, the original deposit will be unreserved.
		///
		/// The dispatch origin for this call must be _Signed_ and the tip identified by `hash`
		/// must have been reported by the signing account through `report_awesome` (and not
		/// through `tip_new`).
		///
		/// - `hash`: The identity of the open tip for which a tip value is declared. This is formed
		///   as the hash of the tuple of the original tip `reason` and the beneficiary account ID.
		///
		/// Emits `TipRetracted` if successful.
		///
		/// # <weight>
		/// - Complexity: `O(1)`
		///   - Depends on the length of `T::Hash` which is fixed.
		/// - DbReads: `Tips`, `origin account`
		/// - DbWrites: `Reasons`, `Tips`, `origin account`
		/// # </weight>
		#[weight = 120_000_000 + T::DbWeight::get().reads_writes(1, 2)]
		fn retract_tip(origin, hash: T::Hash) {
			let who = ensure_signed(origin)?;
			let tip = <Tips<T>>::get(&hash).ok_or(<Error<T>>::UnknownTip)?;
			ensure!(tip.finder == who, <Error<T>>::NotFinder);

			<Reasons<T>>::remove(&tip.reason);
			<Tips<T>>::remove(&hash);
			if !tip.deposit.is_zero() {
				let _ = T::RingCurrency::unreserve(&who, tip.deposit);
			}
			Self::deposit_event(RawEvent::TipRetracted(hash));
		}

		/// Give a tip for something new; no finder's fee will be taken.
		///
		/// The dispatch origin for this call must be _Signed_ and the signing account must be a
		/// member of the `Tippers` set.
		///
		/// - `reason`: The reason for, or the thing that deserves, the tip; generally this will be
		///   a UTF-8-encoded URL.
		/// - `who`: The account which should be credited for the tip.
		/// - `tip_value`: The amount of tip that the sender would like to give. The median tip
		///   value of active tippers will be given to the `who`.
		///
		/// Emits `NewTip` if successful.
		///
		/// # <weight>
		/// - Complexity: `O(R + T)` where `R` length of `reason`, `T` is the number of tippers.
		///   - `O(T)`: decoding `Tipper` vec of length `T`
		///     `T` is charged as upper bound given by `ContainsLengthBound`.
		///     The actual cost depends on the implementation of `T::Tippers`.
		///   - `O(R)`: hashing and encoding of reason of length `R`
		/// - DbReads: `Tippers`, `Reasons`
		/// - DbWrites: `Reasons`, `Tips`
		/// # </weight>
		#[weight = 110_000_000
			+ 4_000 * reason.len() as Weight
			+ 480_000 * T::Tippers::max_len() as Weight
			+ T::DbWeight::get().reads_writes(2, 2)]
		fn tip_new(origin, reason: Vec<u8>, who: T::AccountId, tip_value: RingBalance<T>) {
			let tipper = ensure_signed(origin)?;
			ensure!(T::Tippers::contains(&tipper), BadOrigin);
			let reason_hash = T::Hashing::hash(&reason[..]);
			ensure!(!<Reasons<T>>::contains_key(&reason_hash), <Error<T>>::AlreadyKnown);
			let hash = T::Hashing::hash_of(&(&reason_hash, &who));

			<Reasons<T>>::insert(&reason_hash, &reason);
			Self::deposit_event(RawEvent::NewTip(hash.clone()));
			let tips = vec![(tipper.clone(), tip_value)];
			let tip = OpenTip {
				reason: reason_hash,
				who,
				finder: tipper,
				deposit: Zero::zero(),
				closes: None,
				tips,
				finders_fee: false,
			};
			<Tips<T>>::insert(&hash, tip);
		}

		/// Declare a tip value for an already-open tip.
		///
		/// The dispatch origin for this call must be _Signed_ and the signing account must be a
		/// member of the `Tippers` set.
		///
		/// - `hash`: The identity of the open tip for which a tip value is declared. This is formed
		///   as the hash of the tuple of the hash of the original tip `reason` and the beneficiary
		///   account ID.
		/// - `tip_value`: The amount of tip that the sender would like to give. The median tip
		///   value of active tippers will be given to the `who`.
		///
		/// Emits `TipClosing` if the threshold of tippers has been reached and the countdown period
		/// has started.
		///
		/// # <weight>
		/// - Complexity: `O(T)` where `T` is the number of tippers.
		///   decoding `Tipper` vec of length `T`, insert tip and check closing,
		///   `T` is charged as upper bound given by `ContainsLengthBound`.
		///   The actual cost depends on the implementation of `T::Tippers`.
		///
		///   Actually weight could be lower as it depends on how many tips are in `OpenTip` but it
		///   is weighted as if almost full i.e of length `T-1`.
		/// - DbReads: `Tippers`, `Tips`
		/// - DbWrites: `Tips`
		/// # </weight>
		#[weight = 68_000_000 + 2_000_000 * T::Tippers::max_len() as Weight
			+ T::DbWeight::get().reads_writes(2, 1)]
		fn tip(origin, hash: T::Hash, tip_value: RingBalance<T>) {
			let tipper = ensure_signed(origin)?;
			ensure!(T::Tippers::contains(&tipper), BadOrigin);

			let mut tip = <Tips<T>>::get(hash).ok_or(<Error<T>>::UnknownTip)?;
			if Self::insert_tip_and_check_closing(&mut tip, tipper, tip_value) {
				Self::deposit_event(RawEvent::TipClosing(hash.clone()));
			}
			<Tips<T>>::insert(&hash, tip);
		}

		/// Close and payout a tip.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// The tip identified by `hash` must have finished its countdown period.
		///
		/// - `hash`: The identity of the open tip for which a tip value is declared. This is formed
		///   as the hash of the tuple of the original tip `reason` and the beneficiary account ID.
		///
		/// # <weight>
		/// - Complexity: `O(T)` where `T` is the number of tippers.
		///   decoding `Tipper` vec of length `T`.
		///   `T` is charged as upper bound given by `ContainsLengthBound`.
		///   The actual cost depends on the implementation of `T::Tippers`.
		/// - DbReads: `Tips`, `Tippers`, `tip finder`
		/// - DbWrites: `Reasons`, `Tips`, `Tippers`, `tip finder`
		/// # </weight>
		#[weight = 220_000_000 + 1_100_000 * T::Tippers::max_len() as Weight
			+ T::DbWeight::get().reads_writes(3, 3)]
		fn close_tip(origin, hash: T::Hash) {
			ensure_signed(origin)?;

			let tip = <Tips<T>>::get(hash).ok_or(<Error<T>>::UnknownTip)?;
			let n = tip.closes.as_ref().ok_or(<Error<T>>::StillOpen)?;
			ensure!(<frame_system::Module<T>>::block_number() >= *n, <Error<T>>::Premature);
			// closed.
			<Reasons<T>>::remove(&tip.reason);
			<Tips<T>>::remove(hash);
			Self::payout_tip(hash, tip);
		}

		/// # <weight>
		/// - Complexity: `O(A)` where `A` is the number of approvals
		/// - Db reads and writes: `Approvals`, `pot account data`
		/// - Db reads and writes per approval:
		///   `Proposals`, `proposer account data`, `beneficiary account data`
		/// - The weight is overestimated if some approvals got missed.
		/// # </weight>
		fn on_initialize(n: T::BlockNumber) -> Weight {			// Check to see if we should spend some funds!
			if (n % T::SpendPeriod::get()).is_zero() {
				let approvals_len = Self::spend_funds();

				270_000_000 * approvals_len
					+ T::DbWeight::get().reads_writes(2 + approvals_len * 3, 2 + approvals_len * 3)
			} else {
				0
			}
		}
	}
}

impl<T: Trait> Module<T> {
	// Add public immutables and private mutables.

	/// The account ID of the treasury pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Return the amount of money in the pot.
	// The existential deposit is not part of the pot so treasury account never gets deleted.
	fn pot<C: LockableCurrency<T::AccountId>>() -> C::Balance {
		C::usable_balance(&Self::account_id())
			// Must never be less than 0 but better be safe.
			.saturating_sub(C::minimum_balance())
	}

	/// The needed bond for a proposal whose spend is `value`.
	fn calculate_bond<Balance, ProposalBondMinimum>(value: Balance) -> Balance
	where
		Balance: Clone + AtLeast32BitUnsigned,
		ProposalBondMinimum: Get<Balance>,
	{
		ProposalBondMinimum::get().max(T::ProposalBond::get() * value)
	}

	/// Given a mutable reference to an `OpenTip`, insert the tip into it and check whether it
	/// closes, if so, then deposit the relevant event and set closing accordingly.
	///
	/// `O(T)` and one storage access.
	fn insert_tip_and_check_closing(
		tip: &mut OpenTip<T::AccountId, RingBalance<T>, T::BlockNumber, T::Hash>,
		tipper: T::AccountId,
		tip_value: RingBalance<T>,
	) -> bool {
		match tip.tips.binary_search_by_key(&&tipper, |x| &x.0) {
			Ok(pos) => tip.tips[pos] = (tipper, tip_value),
			Err(pos) => tip.tips.insert(pos, (tipper, tip_value)),
		}
		Self::retain_active_tips(&mut tip.tips);
		let threshold = (T::Tippers::count() + 1) / 2;
		if tip.tips.len() >= threshold && tip.closes.is_none() {
			tip.closes = Some(<frame_system::Module<T>>::block_number() + T::TipCountdown::get());
			true
		} else {
			false
		}
	}

	/// Remove any non-members of `Tippers` from a `tips` vector. `O(T)`.
	fn retain_active_tips(tips: &mut Vec<(T::AccountId, RingBalance<T>)>) {
		let members = T::Tippers::sorted_members();
		let mut members_iter = members.iter();
		let mut member = members_iter.next();
		tips.retain(|(ref a, _)| loop {
			match member {
				None => break false,
				Some(m) if m > a => break false,
				Some(m) => {
					member = members_iter.next();
					if m < a {
						continue;
					} else {
						break true;
					}
				}
			}
		});
	}

	/// Execute the payout of a tip.
	///
	/// Up to three balance operations.
	/// Plus `O(T)` (`T` is Tippers length).
	fn payout_tip(
		hash: T::Hash,
		tip: OpenTip<T::AccountId, RingBalance<T>, T::BlockNumber, T::Hash>,
	) {
		let mut tips = tip.tips;
		Self::retain_active_tips(&mut tips);
		tips.sort_by_key(|i| i.1);
		let treasury = Self::account_id();
		let max_payout = Self::pot::<T::RingCurrency>();
		let mut payout = tips[tips.len() / 2].1.min(max_payout);
		if !tip.deposit.is_zero() {
			let _ = T::RingCurrency::unreserve(&tip.finder, tip.deposit);
		}
		if tip.finders_fee {
			if tip.finder != tip.who {
				// pay out the finder's fee.
				let finders_fee = T::TipFindersFee::get() * payout;
				payout -= finders_fee;
				// this should go through given we checked it's at most the free balance, but still
				// we only make a best-effort.
				let _ = T::RingCurrency::transfer(&treasury, &tip.finder, finders_fee, KeepAlive);
			}
		}
		// same as above: best-effort only.
		let _ = T::RingCurrency::transfer(&treasury, &tip.who, payout, KeepAlive);
		Self::deposit_event(RawEvent::TipClosed(hash, tip.who, payout));
	}

	/// Spend some money! returns number of approvals before spend.
	fn spend_funds() -> u64 {
		let mut budget_remaining_ring = Self::pot::<T::RingCurrency>();
		let mut budget_remaining_kton = Self::pot::<T::KtonCurrency>();

		Self::deposit_event(RawEvent::Spending(
			budget_remaining_ring,
			budget_remaining_kton,
		));

		let mut miss_any_ring = false;
		let mut imbalance_ring = <RingPositiveImbalance<T>>::zero();

		let mut miss_any_kton = false;
		let mut imbalance_kton = <KtonPositiveImbalance<T>>::zero();

		let prior_approvals_len = Approvals::mutate(|v| {
			let prior_approvals_len = v.len() as u64;
			v.retain(|&index| {
				// Should always be true, but shouldn't panic if false or we're screwed.
				if let Some(p) = Self::proposals(index) {
					if p.ring_value > budget_remaining_ring || p.kton_value > budget_remaining_kton
					{
						if p.ring_value > budget_remaining_ring {
							miss_any_ring = true;
						}

						if p.kton_value > budget_remaining_kton {
							miss_any_kton = true;
						}

						return true;
					}

					if p.ring_value <= budget_remaining_ring {
						budget_remaining_ring -= p.ring_value;

						// return their deposit.
						let _ = T::RingCurrency::unreserve(&p.proposer, p.ring_bond);

						// provide the allocation.
						imbalance_ring.subsume(T::RingCurrency::deposit_creating(
							&p.beneficiary,
							p.ring_value,
						));
					}
					if p.kton_value <= budget_remaining_kton {
						budget_remaining_kton -= p.kton_value;

						// return their deposit.
						let _ = T::KtonCurrency::unreserve(&p.proposer, p.kton_bond);

						// provide the allocation.
						imbalance_kton.subsume(T::KtonCurrency::deposit_creating(
							&p.beneficiary,
							p.kton_value,
						));
					}

					<Proposals<T>>::remove(index);
					Self::deposit_event(RawEvent::Awarded(
						index,
						p.ring_value,
						p.kton_value,
						p.beneficiary,
					));
					false
				} else {
					false
				}
			});

			prior_approvals_len
		});

		{
			let burn_ring = if !miss_any_ring {
				// burn some proportion of the remaining budget if we run a surplus.
				let burn = (T::Burn::get() * budget_remaining_ring).min(budget_remaining_ring);
				budget_remaining_ring -= burn;
				imbalance_ring.subsume(T::RingCurrency::burn(burn));

				burn
			} else {
				Zero::zero()
			};
			let burn_kton = if !miss_any_kton {
				let burn = (T::Burn::get() * budget_remaining_kton).min(budget_remaining_kton);
				budget_remaining_kton -= burn;
				imbalance_kton.subsume(T::KtonCurrency::burn(burn));

				burn
			} else {
				Zero::zero()
			};

			Self::deposit_event(RawEvent::Burnt(burn_ring, burn_kton));
		}

		// Must never be an error, but better to be safe.
		// proof: budget_remaining is account free balance minus ED;
		// Thus we can't spend more than account free balance minus ED;
		// Thus account is kept alive; qed;
		if let Err(problem) = T::RingCurrency::settle(
			&Self::account_id(),
			imbalance_ring,
			WithdrawReason::Transfer.into(),
			KeepAlive,
		) {
			print("Inconsistent state - couldn't settle imbalance for funds spent by treasury");
			// Nothing else to do here.
			drop(problem);
		}

		if let Err(problem) = T::KtonCurrency::settle(
			&Self::account_id(),
			imbalance_kton,
			WithdrawReason::Transfer.into(),
			KeepAlive,
		) {
			print("Inconsistent state - couldn't settle imbalance for funds spent by treasury");
			// Nothing else to do here.
			drop(problem);
		}

		Self::deposit_event(RawEvent::Rollover(
			budget_remaining_ring,
			budget_remaining_kton,
		));

		prior_approvals_len
	}

	pub fn migrate_retract_tip_for_tip_new() {
		/// An open tipping "motion". Retains all details of a tip including information on the finder
		/// and the members who have voted.
		#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
		pub struct OldOpenTip<
			AccountId: Parameter,
			RingBalance: Parameter,
			BlockNumber: Parameter,
			Hash: Parameter,
		> {
			/// The hash of the reason for the tip. The reason should be a human-readable UTF-8 encoded string. A URL would be
			/// sensible.
			reason: Hash,
			/// The account to be tipped.
			who: AccountId,
			/// The account who began this tip and the amount held on deposit.
			finder: Option<(AccountId, RingBalance)>,
			/// The block number at which this tip will close if `Some`. If `None`, then no closing is
			/// scheduled.
			closes: Option<BlockNumber>,
			/// The members who have voted for this tip. Sorted by AccountId.
			tips: Vec<(AccountId, RingBalance)>,
		}

		// --- substrate ---
		use frame_support::{migration::StorageKeyIterator, Twox64Concat};

		for (hash, old_tip) in StorageKeyIterator::<
			T::Hash,
			OldOpenTip<T::AccountId, RingBalance<T>, T::BlockNumber, T::Hash>,
			Twox64Concat,
		>::new(b"DarwiniaTreasury", b"Tips")
		.drain()
		{
			let (finder, deposit, finders_fee) = match old_tip.finder {
				Some((finder, deposit)) => (finder, deposit, true),
				None => (T::AccountId::default(), Zero::zero(), false),
			};
			let new_tip = OpenTip {
				reason: old_tip.reason,
				who: old_tip.who,
				finder,
				deposit,
				closes: old_tip.closes,
				tips: old_tip.tips,
				finders_fee,
			};
			<Tips<T>>::insert(hash, new_tip)
		}
	}
}

impl<T: Trait> OnUnbalanced<RingNegativeImbalance<T>> for Module<T> {
	fn on_nonzero_unbalanced(amount: RingNegativeImbalance<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::RingCurrency::resolve_creating(&Self::account_id(), amount);

		Self::deposit_event(RawEvent::DepositRing(numeric_amount));
	}
}

// FIXME: Ugly hack due to https://github.com/rust-lang/rust/issues/31844#issuecomment-557918823
impl<T: Trait> OnUnbalancedKton<KtonNegativeImbalance<T>> for Module<T> {
	fn on_nonzero_unbalanced(amount: KtonNegativeImbalance<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::KtonCurrency::resolve_creating(&Self::account_id(), amount);

		Self::deposit_event(RawEvent::DepositKton(numeric_amount));
	}
}
