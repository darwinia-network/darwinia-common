// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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

//! Auxillary struct/enums for Darwinia runtime.

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use bp_messages::{
	source_chain::{LaneMessageVerifier, Sender},
	LaneId, OutboundLaneData,
};
use bridge_runtime_common::messages::{source::*, *};
use frame_support::traits::{Currency, Get, Imbalance, OnUnbalanced};
use sp_io::offchain;
use sp_npos_elections::ExtendedBalance;
use sp_runtime::{traits::TrailingZeroInput, RuntimeDebug};
// --- darwinia-network ---
use crate::*;
use drml_primitives::*;

darwinia_support::impl_account_data! {
	struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = drml_primitives::Balance
	{
		// other data
	}
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(PhantomData<R>);
impl<R> OnUnbalanced<RingNegativeImbalance<R>> for ToAuthor<R>
where
	R: darwinia_balances::Config<RingInstance> + pallet_authorship::Config,
	<R as frame_system::Config>::AccountId: From<AccountId> + Into<AccountId>,
	<R as frame_system::Config>::Event: From<darwinia_balances::Event<R, RingInstance>>,
{
	fn on_nonzero_unbalanced(amount: RingNegativeImbalance<R>) {
		let numeric_amount = amount.peek();
		let author = <pallet_authorship::Pallet<R>>::author();

		<darwinia_balances::Pallet<R, RingInstance>>::resolve_creating(
			&<pallet_authorship::Pallet<R>>::author(),
			amount,
		);
		<frame_system::Pallet<R>>::deposit_event(darwinia_balances::Event::Deposit(
			author,
			numeric_amount,
		));
	}
}

pub struct DealWithFees<R>(PhantomData<R>);
impl<R> OnUnbalanced<RingNegativeImbalance<R>> for DealWithFees<R>
where
	R: darwinia_balances::Config<RingInstance>
		+ pallet_treasury::Config
		+ pallet_authorship::Config,
	pallet_treasury::Pallet<R>: OnUnbalanced<RingNegativeImbalance<R>>,
	<R as frame_system::Config>::AccountId: From<AccountId> + Into<AccountId>,
	<R as frame_system::Config>::Event: From<darwinia_balances::Event<R, RingInstance>>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = RingNegativeImbalance<R>>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 80% to treasury, 20% to author
			let mut split = fees.ration(80, 20);

			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				tips.merge_into(&mut split.1);
			}

			<pallet_treasury::Pallet<R> as OnUnbalanced<_>>::on_unbalanced(split.0);
			<ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
		}
	}
}

/// A source of random balance for the NPoS Solver, which is meant to be run by the OCW election
/// miner.
pub struct OffchainRandomBalancing;
impl Get<Option<(usize, ExtendedBalance)>> for OffchainRandomBalancing {
	fn get() -> Option<(usize, ExtendedBalance)> {
		let iters = match MINER_MAX_ITERATIONS {
			0 => 0,
			max @ _ => {
				let seed = offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);

				random as usize
			}
		};

		Some((iters, 0))
	}
}

/// Message verifier that is doing all basic checks.
///
/// This verifier assumes following:
///
/// - all message lanes are equivalent, so all checks are the same;
/// - messages are being dispatched using `pallet-bridge-dispatch` pallet on the target chain.
///
/// Following checks are made:
///
/// - message is rejected if its lane is currently blocked;
/// - message is rejected if there are too many pending (undelivered) messages at the outbound
///   lane;
/// - check that the sender has rights to dispatch the call on target chain using provided
///   dispatch origin;
/// - check that the sender has paid enough funds for both message delivery and dispatch.
#[derive(RuntimeDebug)]
pub struct FromThisChainMessageVerifier<B, R, I>(PhantomData<(B, R, I)>);
impl<B, R, I>
	LaneMessageVerifier<
		AccountIdOf<ThisChain<B>>,
		FromThisChainMessagePayload<B>,
		BalanceOf<ThisChain<B>>,
	> for FromThisChainMessageVerifier<B, R, I>
where
	B: MessageBridge,
	R: darwinia_fee_market::Config<I>,
	I: 'static,
	AccountIdOf<ThisChain<B>>: Clone + PartialEq,
	darwinia_fee_market::RingBalance<R, I>: From<BalanceOf<ThisChain<B>>>,
{
	type Error = &'static str;

	fn verify_message(
		submitter: &Sender<AccountIdOf<ThisChain<B>>>,
		delivery_and_dispatch_fee: &BalanceOf<ThisChain<B>>,
		lane: &LaneId,
		lane_outbound_data: &OutboundLaneData,
		payload: &FromThisChainMessagePayload<B>,
	) -> Result<(), Self::Error> {
		// reject message if lane is blocked
		if !ThisChain::<B>::is_outbound_lane_enabled(lane) {
			return Err(OUTBOUND_LANE_DISABLED);
		}

		// reject message if there are too many pending messages at this lane
		let max_pending_messages = ThisChain::<B>::maximal_pending_messages_at_outbound_lane();
		let pending_messages = lane_outbound_data
			.latest_generated_nonce
			.saturating_sub(lane_outbound_data.latest_received_nonce);
		if pending_messages > max_pending_messages {
			return Err(TOO_MANY_PENDING_MESSAGES);
		}

		// Do the dispatch-specific check. We assume that the target chain uses
		// `Dispatch`, so we verify the message accordingly.
		pallet_bridge_dispatch::verify_message_origin(submitter, payload)
			.map_err(|_| BAD_ORIGIN)?;

		// Do the delivery_and_dispatch_fee. We assume that the delivery and dispatch fee always
		// greater than the fee market provided fee.
		if let Some(market_fee) = darwinia_fee_market::Pallet::<R, I>::market_fee() {
			let message_fee: darwinia_fee_market::RingBalance<R, I> =
				(*delivery_and_dispatch_fee).into();

			// compare with actual fee paid
			if message_fee < market_fee {
				return Err(TOO_LOW_FEE);
			}
		} else {
			const NO_MARKET_FEE: &str = "The fee market are not ready for accepting messages.";

			return Err(NO_MARKET_FEE);
		}

		Ok(())
	}
}

#[macro_export]
macro_rules! impl_self_contained_call {
	() => {
		impl fp_self_contained::SelfContainedCall for Call {
			type SignedInfo = H160;

			fn is_self_contained(&self) -> bool {
				match self {
					Call::Ethereum(call) => call.is_self_contained(),
					_ => false,
				}
			}

			fn check_self_contained(
				&self,
			) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
				match self {
					Call::Ethereum(call) => call.check_self_contained(),
					_ => None,
				}
			}

			fn validate_self_contained(
				&self,
				info: &Self::SignedInfo,
			) -> Option<TransactionValidity> {
				match self {
					Call::Ethereum(call) => call.validate_self_contained(info),
					_ => None,
				}
			}

			fn pre_dispatch_self_contained(
				&self,
				info: &Self::SignedInfo,
			) -> Option<Result<(), TransactionValidityError>> {
				match self {
					Call::Ethereum(call) => call.pre_dispatch_self_contained(info),
					_ => None,
				}
			}

			fn apply_self_contained(
				self,
				info: Self::SignedInfo,
			) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
				match self {
					call @ Call::Ethereum(darwinia_ethereum::Call::transact { .. }) => {
						Some(call.dispatch(Origin::from(
							darwinia_ethereum::RawOrigin::EthereumTransaction(info),
						)))
					}
					_ => None,
				}
			}
		}

		fn validate_self_contained_inner(
			call: &Call,
			eth_call: &darwinia_ethereum::Call<Runtime>,
			signed_info: &<Call as fp_self_contained::SelfContainedCall>::SignedInfo,
		) -> TransactionValidity {
			if let darwinia_ethereum::Call::transact { ref transaction } = eth_call {
				// Previously, ethereum transactions were contained in an unsigned
				// extrinsic, we now use a new form of dedicated extrinsic defined by
				// frontier, but to keep the same behavior as before, we must perform
				// the controls that were performed on the unsigned extrinsic.
				use sp_runtime::traits::SignedExtension as _;
				let input_len = match transaction {
					darwinia_ethereum::Transaction::Legacy(t) => t.input.len(),
					darwinia_ethereum::Transaction::EIP2930(t) => t.input.len(),
					darwinia_ethereum::Transaction::EIP1559(t) => t.input.len(),
				};
				let extra_validation =
					SignedExtra::validate_unsigned(call, &call.get_dispatch_info(), input_len)?;
				// Then, do the controls defined by the ethereum pallet.
				use fp_self_contained::SelfContainedCall as _;
				let self_contained_validation =
					eth_call.validate_self_contained(signed_info).ok_or(
						TransactionValidityError::Invalid(InvalidTransaction::BadProof),
					)??;

				Ok(extra_validation.combine_with(self_contained_validation))
			} else {
				Err(TransactionValidityError::Unknown(
					sp_runtime::transaction_validity::UnknownTransaction::CannotLookup,
				))
			}
		}
	};
}
