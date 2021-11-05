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
		} = slash_and_calculate_rewards::<T, I>(messages_relayers, relayer_fund_account);

		// Pay confirmation relayer rewards
		transfer_and_print_logs_on_error::<T>(
			relayer_fund_account,
			confirmation_relayer,
			confirmation_relayer_rewards,
		);
		// Pay messages relayers rewards
		for (relayer, reward) in messages_relayers_rewards {
			transfer_and_print_logs_on_error::<T>(relayer_fund_account, &relayer, reward);
		}
		// Pay assign relayer reward
		for (relayer, reward) in assigned_relayers_rewards {
			transfer_and_print_logs_on_error::<T>(relayer_fund_account, &relayer, reward);
		}
		// Pay treasury reward
		transfer_and_print_logs_on_error::<T>(
			relayer_fund_account,
			&T::TreasuryPalletId::get().into_account(),
			treasury_total_rewards,
		);
	}
}

/// Slash and calculate rewards for messages_relayers, confirmation relayers, treasury, assigned_relayers
pub fn slash_and_calculate_rewards<T, I>(
	messages_relayers: VecDeque<UnrewardedRelayer<T::AccountId>>,
	relayer_fund_account: &T::AccountId,
) -> RewardsBook<T::AccountId, RingBalance<T>>
where
	T: frame_system::Config + pallet_bridge_messages::Config<I> + Config,
	I: 'static,
{
	let mut confirmation_rewards = RingBalance::<T>::zero();
	let mut messages_rewards = BTreeMap::<T::AccountId, RingBalance<T>>::new();
	let mut assigned_relayers_rewards = BTreeMap::<T::AccountId, RingBalance<T>>::new();
	let mut treasury_total_rewards = RingBalance::<T>::zero();

	for (lane_id, message_nonce) in <ConfirmedMessagesThisBlock<T>>::get() {
		// The order created when message was accepted, so we can always get the order info below.
		if let Some(order) = <Orders<T>>::get(&(lane_id, message_nonce)) {
			// The confirm_time of the order is set in the `OnDeliveryConfirmed` callback. And the callback function
			// was called as source chain received message delivery proof, before the reward payment.
			let order_confirm_time = order
				.confirm_time
				.unwrap_or_else(|| frame_system::Pallet::<T>::block_number());
			// Iterate the unrewarded relayer list to get message relayer
			let mut message_relayer = T::AccountId::default();
			for unrewarded_relayer in messages_relayers.iter() {
				if unrewarded_relayer.messages.contains_message(message_nonce) {
					message_relayer = unrewarded_relayer.relayer.clone();
					break;
				}
			}
			let lowest_fee = order.lowest_and_highest_fee().0.unwrap_or_default();
			let message_fee = order.lowest_and_highest_fee().1.unwrap_or_default();

			let message_reward;
			let confirm_reward;
			if let Some(who) = order.required_delivery_relayer_for_time(order_confirm_time) {
				// message fee - lowest fee => treasury
				let treasury_reward = message_fee.saturating_sub(lowest_fee);
				treasury_total_rewards = treasury_total_rewards.saturating_add(treasury_reward);

				// 60% * lowest fee => assigned_relayers_rewards
				let assigned_relayers_reward = T::AssignedRelayersRewardRatio::get() * lowest_fee;
				assigned_relayers_rewards
					.entry(who)
					.and_modify(|r| *r = r.saturating_add(assigned_relayers_reward))
					.or_insert(assigned_relayers_reward);

				let bridger_relayers_reward = lowest_fee.saturating_sub(assigned_relayers_reward);
				// 80% * (1 - 60%) * lowest_fee => message relayer
				message_reward = T::MessageRelayersRewardRatio::get() * bridger_relayers_reward;
				// 20% * (1 - 60%) * lowest_fee => confirm relayer
				confirm_reward = T::ConfirmRelayersRewardRatio::get() * bridger_relayers_reward;
			} else {
				// The message is delivered by common relayer instead of order assigned relayers, all assigned relayers of this order should be punished.
				let slashed_reward = slash_assigned_relayers::<T>(order, relayer_fund_account);

				// 80% total slash => confirm relayer
				message_reward = T::MessageRelayersRewardRatio::get() * slashed_reward;
				// 20% total slash => confirm relayer
				confirm_reward = T::ConfirmRelayersRewardRatio::get() * slashed_reward;
			}

			// Update confirmation relayer total rewards
			confirmation_rewards = confirmation_rewards.saturating_add(confirm_reward);
			// Update message relayers total rewards
			messages_rewards
				.entry(message_relayer)
				.and_modify(|r| *r = r.saturating_add(message_reward))
				.or_insert(message_reward);
		}
	}
	RewardsBook {
		messages_relayers_rewards: messages_rewards,
		confirmation_relayer_rewards: confirmation_rewards,
		assigned_relayers_rewards,
		treasury_total_rewards,
	}
}

/// Slash order assigned relayers
pub fn slash_assigned_relayers<T: Config>(
	order: Order<T::AccountId, T::BlockNumber, RingBalance<T>>,
	relayer_fund_account: &T::AccountId,
) -> RingBalance<T> {
	let mut total_slash = RingBalance::<T>::zero();
	match (order.confirm_time, order.range_end()) {
		(Some(confirm_time), Some(end_time)) if confirm_time >= end_time => {
			let timeout = confirm_time - end_time;
			let message_fee = order.lowest_and_highest_fee().1.unwrap_or_default();
			let slash_max = T::Slasher::slash(message_fee, timeout);
			debug_assert!(
				slash_max <= T::MiniumLockCollateral::get(),
				"The maximum slash value returned from Slasher is MiniumLockCollateral"
			);

			for assigned_relayer in order.relayers_slice() {
				let slashed_asset =
					do_slash::<T>(&assigned_relayer.id, relayer_fund_account, slash_max);
				total_slash += slashed_asset;
			}
		}
		_ => {}
	}
	total_slash
}

/// Do slash for absent assigned relayers
pub fn do_slash<T: Config>(
	slash_account: &T::AccountId,
	fund_account: &T::AccountId,
	slash_max: RingBalance<T>,
) -> RingBalance<T> {
	let slashed;
	let locked_collateral = crate::Pallet::<T>::relayer_locked_collateral(&slash_account);
	T::RingCurrency::remove_lock(T::LockId::get(), &slash_account);
	if locked_collateral >= slash_max {
		slashed = slash_max;
		let locked_reserved = locked_collateral.saturating_sub(slashed);
		transfer_and_print_logs_on_error::<T>(slash_account, fund_account, slashed);
		crate::Pallet::<T>::update_collateral(&slash_account, locked_reserved);
	} else {
		slashed = locked_collateral;
		transfer_and_print_logs_on_error::<T>(slash_account, fund_account, slashed);
		crate::Pallet::<T>::update_collateral(&slash_account, RingBalance::<T>::zero());
	}
	slashed
}

/// Do transfer
fn transfer_and_print_logs_on_error<T: Config>(
	from: &T::AccountId,
	to: &T::AccountId,
	amount: RingBalance<T>,
) {
	if amount.is_zero() {
		return;
	}

	let pay_result = <T as Config>::RingCurrency::transfer(
		from,
		to,
		amount,
		// the relayer fund account must stay above ED (needs to be pre-funded)
		ExistenceRequirement::KeepAlive,
	);

	match pay_result {
		Ok(_) => log::trace!("Transfer, from {:?} to {:?} amount: {:?}", from, to, amount,),

		Err(error) => log::error!(
			"Transfer, from {:?} to {:?} amount {:?}: {:?}",
			from,
			to,
			amount,
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
