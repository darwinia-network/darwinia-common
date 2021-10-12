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

// --- substrate ---
use bp_messages::{
	source_chain::{MessageDeliveryAndDispatchPayment, Sender},
	MessageNonce, UnrewardedRelayer,
};
use frame_support::traits::{Currency as CurrencyT, ExistenceRequirement, Get};
use sp_runtime::traits::{AccountIdConversion, Saturating};
use sp_std::{
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	ops::RangeInclusive,
};
// --- darwinia-network ---
use crate::{Config, ConfirmedMessagesThisBlock, Orders, *};
// --- std ---
use num_traits::Zero;

pub struct FeeMarketPayment<T, I, Currency, GetConfirmationFee, RootAccount> {
	_phantom: sp_std::marker::PhantomData<(T, I, Currency, GetConfirmationFee, RootAccount)>,
}

impl<T, I, Currency, GetConfirmationFee, RootAccount>
	MessageDeliveryAndDispatchPayment<T::AccountId, RingBalance<T>>
	for FeeMarketPayment<T, I, Currency, GetConfirmationFee, RootAccount>
where
	T: frame_system::Config + pallet_bridge_messages::Config<I> + Config,
	I: 'static,
	Currency: CurrencyT<T::AccountId, Balance = T::OutboundMessageFee>,
	Currency::Balance: From<MessageNonce>,
	GetConfirmationFee: Get<RingBalance<T>>,
	RootAccount: Get<Option<T::AccountId>>,
{
	type Error = &'static str;

	fn pay_delivery_and_dispatch_fee(
		submitter: &Sender<T::AccountId>,
		fee: &RingBalance<T>, // P3
		relayer_fund_account: &T::AccountId,
	) -> Result<(), Self::Error> {
		if !frame_system::Pallet::<T>::account_exists(relayer_fund_account) {
			return Err("The relayer fund account must exist for the message lanes pallet to work correctly.");
		}

		let root_account = RootAccount::get();
		let account = match submitter {
			Sender::Signed(submitter) => submitter,
			Sender::Root | Sender::None => root_account
				.as_ref()
				.ok_or("Sending messages using Root or None origin is disallowed.")?,
		};

		<T as Config>::RingCurrency::transfer(
			account,
			relayer_fund_account,
			*fee,
			// it's fine for the submitter to go below Existential Deposit and die.
			ExistenceRequirement::AllowDeath,
		)
		.map_err(Into::into)
	}

	fn pay_relayers_rewards(
		_lane_id: LaneId,
		messages_relayers: VecDeque<UnrewardedRelayer<T::AccountId>>,
		confirmation_relayer: &T::AccountId,
		_received_range: &RangeInclusive<MessageNonce>,
		relayer_fund_account: &T::AccountId,
	) {
		let RewardsBook {
			messages_relayers_rewards,
			confirmation_relayer_rewards,
			assigned_relayers_rewards,
			treasury_total_rewards,
		} = cal_rewards::<T, I>(messages_relayers, relayer_fund_account);

		// Pay confirmation relayer rewards
		pay_reward::<T>(
			relayer_fund_account,
			confirmation_relayer,
			confirmation_relayer_rewards,
		);
		// Pay messages relayers rewards
		for (relayer, reward) in messages_relayers_rewards {
			pay_reward::<T>(relayer_fund_account, &relayer, reward);
		}
		// Pay assign relayer reward
		for (relayer, reward) in assigned_relayers_rewards {
			pay_reward::<T>(relayer_fund_account, &relayer, reward);
		}
		// Pay treasury reward
		pay_reward::<T>(
			relayer_fund_account,
			&T::TreasuryPalletId::get().into_account(),
			treasury_total_rewards,
		);
	}
}

/// Calculate rewards for messages_relayers, confirmation relayers, treasury, assigned_relayers
pub fn cal_rewards<T, I>(
	messages_relayers: VecDeque<UnrewardedRelayer<T::AccountId>>,
	relayer_fund_account: &T::AccountId,
) -> RewardsBook<T::AccountId, RingBalance<T>>
where
	T: frame_system::Config + pallet_bridge_messages::Config<I> + Config,
	I: 'static,
{
	let mut confirmation_relayer_rewards = RingBalance::<T>::zero();
	let mut assigned_relayers_rewards = BTreeMap::<T::AccountId, RingBalance<T>>::new();
	let mut messages_relayers_rewards = BTreeMap::<T::AccountId, RingBalance<T>>::new();
	let mut treasury_total_rewards = RingBalance::<T>::zero();

	for (lane_id, message_nonce) in <ConfirmedMessagesThisBlock<T>>::get() {
		// The order created when message was accepted, so we can always get the order info below.
		if let Some(order) = <Orders<T>>::get(&(lane_id, message_nonce)) {
			// The confirm_time of the order is set in the `OnDeliveryConfirmed` callback. And the callback function
			// was called as source chain received message delivery proof, before the reward payment.
			let order_confirm_time = order
				.confirm_time
				.expect("The message confirm_time already set in OnDeliveryConfirmed");
			let (p1, p2, p3) = order
				.assigned_relayers
				.clone()
				.expect("The order assigned_relayers already set in OnMessageAccepted");

			// Look up the unrewarded relayer list to get message relayer of this message
			let mut message_relayer = T::AccountId::default();
			for unrewarded_relayer in messages_relayers.iter() {
				if unrewarded_relayer.messages.contains_message(message_nonce) {
					message_relayer = unrewarded_relayer.relayer.clone();
					break;
				}
			}

			// Calculate message relayer's reward, confirmation_relayer's reward, treasury's reward, assigned_relayer's reward
			let message_reward;
			let confirm_reward;
			if p1.valid_range.contains(&order_confirm_time)
				|| p2.valid_range.contains(&order_confirm_time)
				|| p3.valid_range.contains(&order_confirm_time)
			{
				let message_fee = p3.fee;
				let treasury_reward = message_fee.saturating_sub(p1.fee);
				let assigned_relayers_reward = T::ForAssignedRelayers::get() * p1.fee;
				let bridger_relayers_reward = p1.fee.saturating_sub(assigned_relayers_reward);
				message_reward = T::ForMessageRelayer::get() * bridger_relayers_reward;
				confirm_reward = T::ForConfirmRelayer::get() * bridger_relayers_reward;

				// Update treasury total rewards
				treasury_total_rewards = treasury_total_rewards.saturating_add(treasury_reward);
				// Update assigned relayers total rewards
				if p1.valid_range.contains(&order_confirm_time) {
					assigned_relayers_rewards
						.entry(p1.id)
						.and_modify(|r| *r = r.saturating_add(assigned_relayers_reward))
						.or_insert(assigned_relayers_reward);
				} else if p2.valid_range.contains(&order_confirm_time) {
					assigned_relayers_rewards
						.entry(p2.id)
						.and_modify(|r| *r = r.saturating_add(assigned_relayers_reward))
						.or_insert(assigned_relayers_reward);
				} else if p3.valid_range.contains(&order_confirm_time) {
					assigned_relayers_rewards
						.entry(p3.id)
						.and_modify(|r| *r = r.saturating_add(assigned_relayers_reward))
						.or_insert(assigned_relayers_reward);
				}
			} else {
				// In the case of the message is delivered by common relayer instead of p1, p2, p3, we slash all
				// assigned relayers of this order.
				let timeout = order_confirm_time - p3.valid_range.end;
				let slashed_reward = slash_order_assigned_relayers::<T>(
					timeout,
					order.assigned_relayers,
					relayer_fund_account,
				);
				message_reward = T::ForMessageRelayer::get() * slashed_reward;
				confirm_reward = T::ForConfirmRelayer::get() * slashed_reward;
			}

			// Update confirmation relayer total rewards
			confirmation_relayer_rewards =
				confirmation_relayer_rewards.saturating_add(confirm_reward);
			// Update message relayers total rewards
			messages_relayers_rewards
				.entry(message_relayer)
				.and_modify(|r| *r = r.saturating_add(message_reward))
				.or_insert(message_reward);
		}
	}

	RewardsBook {
		messages_relayers_rewards,
		confirmation_relayer_rewards,
		assigned_relayers_rewards,
		treasury_total_rewards,
	}
}

/// Slash order assigned relayers
pub fn slash_order_assigned_relayers<T: Config>(
	timeout: T::BlockNumber,
	assign_relayers: Option<AssignedRelayers<T::AccountId, T::BlockNumber, RingBalance<T>>>,
	relayer_fund_account: &T::AccountId,
) -> RingBalance<T> {
	let mut total_slash = RingBalance::<T>::zero();
	let (p1, p2, p3) = assign_relayers.unwrap_or_default();
	let slash_result = T::Slasher::slash(p3.fee, timeout);

	// Slash assign relayers and transfer the value to refund_fund_account
	slash_and_update_market::<T>(&p1.id, relayer_fund_account, slash_result);
	slash_and_update_market::<T>(&p2.id, relayer_fund_account, slash_result);
	slash_and_update_market::<T>(&p3.id, relayer_fund_account, slash_result);

	total_slash = total_slash
		.saturating_add(slash_result)
		.saturating_add(slash_result)
		.saturating_add(slash_result);
	total_slash
}

/// Pay slash value for absent assigned relayers
pub fn slash_and_update_market<T: Config>(
	slash_account: &T::AccountId,
	fund_account: &T::AccountId,
	slash_value: RingBalance<T>,
) {
	debug_assert!(
		slash_value <= T::MiniumLockCollateral::get(),
		"The maximum slash value returned from Slasher is MiniumLockCollateral"
	);
	// If usable_balance is enough to pay slash, no need to update lock.
	if slash_value <= T::RingCurrency::usable_balance(&slash_account) {
		pay_reward::<T>(slash_account, fund_account, slash_value);
		return;
	}

	// Otherwise, unlock and pay for slash, then lock the remaining usable balance
	T::RingCurrency::remove_lock(T::LockId::get(), &slash_account);
	if T::RingCurrency::usable_balance(&slash_account) >= slash_value {
		pay_reward::<T>(slash_account, fund_account, slash_value);
		// Important: It's necessary to update fee market, since the slash account's lock balance changes
		crate::Pallet::<T>::update_collateral(
			&slash_account,
			T::RingCurrency::usable_balance(&slash_account),
		);
	} else {
		log::error!(
			"The usable balance is not enough to pay slash value, the usable balance {:?} slash value {:?}",
			T::RingCurrency::usable_balance(&slash_account),
			slash_value,
		)
	}
}

/// Pay reward to a specific account
fn pay_reward<T: Config>(from: &T::AccountId, to: &T::AccountId, reward: RingBalance<T>) {
	if reward.is_zero() {
		return;
	}

	let pay_result = <T as Config>::RingCurrency::transfer(
		from,
		to,
		reward,
		// the relayer fund account must stay above ED (needs to be pre-funded)
		ExistenceRequirement::KeepAlive,
	);

	match pay_result {
		Ok(_) => log::trace!(
			"Pay reward, from {:?} to {:?} reward: {:?}",
			from,
			to,
			reward,
		),

		Err(error) => log::error!(
			"Failed to pay reward, from {:?} to {:?} reward {:?}: {:?}",
			from,
			to,
			reward,
			error,
		),
	}
}

/// Record the calculation rewards result
pub struct RewardsBook<AccountId, Balance> {
	pub messages_relayers_rewards: BTreeMap<AccountId, Balance>,
	pub confirmation_relayer_rewards: Balance,
	pub assigned_relayers_rewards: BTreeMap<AccountId, Balance>,
	pub treasury_total_rewards: Balance,
}
