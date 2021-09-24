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

use bp_messages::{
	source_chain::{MessageDeliveryAndDispatchPayment, RelayersRewards, Sender},
	MessageNonce,
};

use crate::*;
use crate::{Config, ConfirmedMessagesThisBlock, Orders};
use codec::Encode;
use frame_support::traits::{Currency as CurrencyT, ExistenceRequirement, Get};
use num_traits::Zero;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::Saturating;
use sp_runtime::Permill;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::fmt::Debug;
use sp_std::ops::Range;

pub struct FeeMarketPayment<T, GetConfirmationFee, RootAccount> {
	_phantom: sp_std::marker::PhantomData<(T, GetConfirmationFee, RootAccount)>,
}

impl<T, GetConfirmationFee, RootAccount>
	MessageDeliveryAndDispatchPayment<T::AccountId, RingBalance<T>>
	for FeeMarketPayment<T, GetConfirmationFee, RootAccount>
where
	T: Config,
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
		confirmation_relayer: &T::AccountId,
		relayers_rewards: RelayersRewards<T::AccountId, RingBalance<T>>,
		relayer_fund_account: &T::AccountId,
	) {
		let mut confirm_total_reward = RingBalance::<T>::zero();
		let mut assigned_total_reward = BTreeMap::<T::AccountId, RingBalance<T>>::new();
		let mut treasury_total_reward = RingBalance::<T>::zero();
		for order_hash in ConfirmedMessagesThisBlock::<T>::get() {
			// Get order info
			let order = <Orders<T>>::get(&order_hash);
			let order_confirm_time = order
				.confirm_time
				.expect("The message confirm_time already set in OnDeliveryConfirmed");
			let (p1, p2, p3) = order.order_relayers.clone().unwrap();

			// Calculate reward
			let mut message_reward = RingBalance::<T>::zero();
			let mut confirm_reward = RingBalance::<T>::zero();
			if p1.valid_range.contains(&order_confirm_time)
				|| p2.valid_range.contains(&order_confirm_time)
				|| p3.valid_range.contains(&order_confirm_time)
			{
				let total_reward = p3.fee;
				let treasury_reward = total_reward.saturating_sub(p1.fee);
				let assign_reward = T::ForAssignedRelayer::get() * p1.fee;
				let bridger_relayer_reward = p1.fee.saturating_sub(assign_reward);

				message_reward = T::ForMessageRelayer::get() * bridger_relayer_reward;
				confirm_reward = T::ForConfirmRelayer::get() * bridger_relayer_reward;

				treasury_total_reward = treasury_total_reward.saturating_add(treasury_reward);

				if p1.valid_range.contains(&order_confirm_time) {
					assigned_total_reward
						.entry(p1.id)
						.or_insert(RingBalance::<T>::zero())
						.saturating_add(assign_reward);
				} else if p2.valid_range.contains(&order_confirm_time) {
					assigned_total_reward
						.entry(p2.id)
						.or_insert(RingBalance::<T>::zero())
						.saturating_add(assign_reward);
				} else if p3.valid_range.contains(&order_confirm_time) {
					assigned_total_reward
						.entry(p3.id)
						.or_insert(RingBalance::<T>::zero())
						.saturating_add(assign_reward);
				}
			} else {
				let slash_reward = slash_assign_relayers::<T>(
					p3.valid_range.end,
					order_confirm_time,
					order.order_relayers.unwrap(),
					relayer_fund_account,
				);
				message_reward = T::ForMessageRelayer::get() * slash_reward;
				confirm_reward = T::ForConfirmRelayer::get() * slash_reward;
			}

			confirm_total_reward = confirm_total_reward.saturating_add(confirm_reward);

			// TODO:
			// Pay message relayer reward
			// pay_reward::<T>(relayer_fund_account, &relayer, message_reward);
		}
		// Pay confirmation relayer reward
		pay_reward::<T>(
			relayer_fund_account,
			confirmation_relayer,
			confirm_total_reward,
		);
		// Pay treasury reward
		pay_reward::<T>(
			relayer_fund_account,
			&T::TreasuryPalletId::get().into_account(),
			treasury_total_reward,
		);
		// Pay assign relayer reward
		for (relayer, reward) in assigned_total_reward {
			pay_reward::<T>(relayer_fund_account, &relayer, reward);
		}
	}
}

/// Transfer funds from relayers fund account to given relayer.
fn pay_reward<T: Config>(
	relayer_fund_account: &T::AccountId,
	relayer_account: &T::AccountId,
	reward: RingBalance<T>,
) {
	if reward.is_zero() {
		return;
	}

	let pay_result = <T as Config>::RingCurrency::transfer(
		relayer_fund_account,
		relayer_account,
		reward,
		// the relayer fund account must stay above ED (needs to be pre-funded)
		ExistenceRequirement::KeepAlive,
	);

	match pay_result {
		Ok(_) => log::trace!(
			target: "runtime::bridge-messages",
			"Rewarded relayer {:?} with {:?}",
			relayer_account,
			reward,
		),
		Err(error) => log::trace!(
			target: "runtime::bridge-messages",
			"Failed to pay relayer {:?} reward {:?}: {:?}",
			relayer_account,
			reward,
			error,
		),
	}
}

pub fn slash_assign_relayers<T: Config>(
	_p3_end_time: T::BlockNumber,
	_confirm_time: T::BlockNumber,
	assign_relayers: OrderRelayers<T::AccountId, T::BlockNumber, RingBalance<T>>,
	relayer_fund_account: &T::AccountId,
) -> RingBalance<T> {
	let (p1, p2, p3) = assign_relayers;
	let total_slash = p3.fee.saturating_add(T::SlashAssignRelayer::get());

	// Slash assign relayers and transfer the value to refund_fund_account
	// TODO:  Slash relayers from deposit balance or tranferable value
	let _ = <T as Config>::RingCurrency::transfer(
		&p1.id,
		relayer_fund_account,
		total_slash,
		ExistenceRequirement::KeepAlive,
	);
	let _ = <T as Config>::RingCurrency::transfer(
		&p2.id,
		relayer_fund_account,
		total_slash,
		ExistenceRequirement::KeepAlive,
	);
	let _ = <T as Config>::RingCurrency::transfer(
		&p3.id,
		relayer_fund_account,
		total_slash,
		ExistenceRequirement::KeepAlive,
	);

	total_slash
		.saturating_add(total_slash)
		.saturating_sub(total_slash)
}
