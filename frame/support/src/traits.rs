// --- substrate ---
pub use frame_support::traits::{LockIdentifier, VestingSchedule, WithdrawReason, WithdrawReasons};

// --- core ---
use core::fmt::Debug;
// --- crates ---
use codec::FullCodec;
use impl_trait_for_tuples::impl_for_tuples;
// --- substrate ---
use frame_support::traits::{Currency, TryDrop};
use sp_runtime::{traits::Convert, DispatchError, DispatchResult};
use sp_std::prelude::*;
// --- darwinia ---
use crate::{
	balance::{
		lock::{LockFor, LockReasons},
		FrozenBalance,
	},
	relay::{RawHeaderThing, Round, TcHeaderId},
};

pub trait BalanceInfo<Balance, Module> {
	fn free(&self) -> Balance;
	fn set_free(&mut self, new_free: Balance);

	fn reserved(&self) -> Balance;
	fn set_reserved(&mut self, new_reserved: Balance);

	/// The total balance in this account including any that is reserved and ignoring any frozen.
	fn total(&self) -> Balance;

	/// How much this account's balance can be reduced for the given `reasons`.
	fn usable(&self, reasons: LockReasons, frozen_balance: FrozenBalance<Balance>) -> Balance;
}

/// A currency whose accounts can have liquidity restrictions.
pub trait LockableCurrency<AccountId>: Currency<AccountId> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type Moment;

	/// Create a new balance lock on account `who`.
	///
	/// If the new lock is valid (i.e. not already expired), it will push the struct to
	/// the `Locks` vec in storage. Note that you can lock more funds than a user has.
	///
	/// If the lock `id` already exists, this will update it.
	fn set_lock(
		id: LockIdentifier,
		who: &AccountId,
		lock_for: LockFor<Self::Balance, Self::Moment>,
		reasons: WithdrawReasons,
	);

	// TODO: for democracy
	// /// Changes a balance lock (selected by `id`) so that it becomes less liquid in all
	// /// parameters or creates a new one if it does not exist.
	// ///
	// /// Calling `extend_lock` on an existing lock `id` differs from `set_lock` in that it
	// /// applies the most severe constraints of the two, while `set_lock` replaces the lock
	// /// with the new parameters. As in, `extend_lock` will set:
	// /// - maximum `amount`
	// /// - bitwise mask of all `reasons`
	// fn extend_lock(
	// 	id: LockIdentifier,
	// 	who: &AccountId,
	// 	amount: Self::Balance,
	// 	reasons: WithdrawReasons,
	// );

	/// Remove an existing lock.
	fn remove_lock(id: LockIdentifier, who: &AccountId);

	/// Get the balance of an account that can be used for transfers, reservations, or any other
	/// non-locking, non-transaction-fee activity. Will be at most `free_balance`.
	fn usable_balance(who: &AccountId) -> Self::Balance;

	/// Get the balance of an account that can be used for paying transaction fees (not tipping,
	/// or any other kind of fees, though). Will be at most `free_balance`.
	fn usable_balance_for_fees(who: &AccountId) -> Self::Balance;
}

pub trait DustCollector<AccountId> {
	fn check(who: &AccountId) -> Result<(), ()>;

	fn collect(who: &AccountId);
}

#[impl_for_tuples(15)]
impl<AccountId> DustCollector<AccountId> for Currencies {
	fn check(who: &AccountId) -> Result<(), ()> {
		for_tuples!( #( Currencies::check(who)?; )* );
		Ok(())
	}

	fn collect(who: &AccountId) {
		for_tuples!( #( Currencies::collect(who); )* );
	}
}

/// Callback on eth-backing module
pub trait OnDepositRedeem<AccountId> {
	type Balance;

	fn on_deposit_redeem(
		backing: &AccountId,
		start_at: u64,
		months: u8,
		amount: Self::Balance,
		stash: &AccountId,
	) -> DispatchResult;
}

// FIXME: Ugly hack due to https://github.com/rust-lang/rust/issues/31844#issuecomment-557918823
/// Handler for when some currency "account" decreased in balance for
/// some reason.
///
/// The only reason at present for an increase would be for validator rewards, but
/// there may be other reasons in the future or for other chains.
///
/// Reasons for decreases include:
///
/// - Someone got slashed.
/// - Someone paid for a transaction to be included.
pub trait OnUnbalancedKton<Imbalance: TryDrop> {
	/// Handler for some imbalance. Infallible.
	fn on_unbalanced(amount: Imbalance) {
		amount
			.try_drop()
			.unwrap_or_else(Self::on_nonzero_unbalanced)
	}

	/// Actually handle a non-zero imbalance. You probably want to implement this rather than
	/// `on_unbalanced`.
	fn on_nonzero_unbalanced(amount: Imbalance);
}

impl<Imbalance: TryDrop> OnUnbalancedKton<Imbalance> for () {
	fn on_nonzero_unbalanced(amount: Imbalance) {
		drop(amount);
	}
}

// A regulator to adjust relay args for a specific chain
// Implement this in runtime's impls
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type TcBlockNumber;
	type Sampler: Convert<Round, Vec<Self::TcBlockNumber>> + Convert<u32, Round>;

	fn challenge_time(round: Round) -> Self::Moment;

	fn round_from_chain_len(chain_len: u32) -> Round;

	fn samples_from_round(round: Round) -> Vec<Self::TcBlockNumber>;

	fn estimate_bond(round: Round, proposals_count: u32) -> Self::Balance;
}

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait Relayable {
	type TcBlockNumber: Clone + Copy + Debug + Default + PartialEq + FullCodec;
	type TcHeaderHash: Clone + Debug + Default + PartialEq + FullCodec;

	/// The latest finalize block's header's record id in darwinia
	fn highest_confirmed_tc_header_id() -> TcHeaderId<Self::TcBlockNumber, Self::TcHeaderHash>;

	/// Verify the codec style header thing chain
	fn verify_header_chain(
		raw_header_thing_chain: &[RawHeaderThing],
	) -> Result<Vec<TcHeaderId<Self::TcBlockNumber, Self::TcHeaderHash>>, DispatchError>;

	/// Check the header if it's already existed
	fn header_existed(block_number: Self::TcBlockNumber) -> bool;
}
