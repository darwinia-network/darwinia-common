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

use codec::Encode;
use frame_support::traits::{Currency as CurrencyT, ExistenceRequirement, Get};
use num_traits::Zero;
use sp_runtime::traits::Saturating;
use sp_std::fmt::Debug;
pub struct FeeMarketPayment<T, Currency, GetConfirmationFee, RootAccount> {
	_phantom: sp_std::marker::PhantomData<(T, Currency, GetConfirmationFee, RootAccount)>,
}

impl<T, Currency, GetConfirmationFee, RootAccount>
	MessageDeliveryAndDispatchPayment<T::AccountId, Currency::Balance>
	for FeeMarketPayment<T, Currency, GetConfirmationFee, RootAccount>
where
	T: frame_system::Config,
	Currency: CurrencyT<T::AccountId>,
	Currency::Balance: From<MessageNonce>,
	GetConfirmationFee: Get<Currency::Balance>,
	RootAccount: Get<Option<T::AccountId>>,
{
	type Error = &'static str;

	fn pay_delivery_and_dispatch_fee(
		submitter: &Sender<T::AccountId>,
		fee: &Currency::Balance,
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

		Currency::transfer(
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
		relayers_rewards: RelayersRewards<T::AccountId, Currency::Balance>,
		relayer_fund_account: &T::AccountId,
	) {
		pay_relayers_rewards::<Currency, _>(
			confirmation_relayer,
			relayers_rewards,
			relayer_fund_account,
			GetConfirmationFee::get(),
		);
	}
}

/// Pay rewards to given relayers, optionally rewarding confirmation relayer.
fn pay_relayers_rewards<Currency, AccountId>(
	confirmation_relayer: &AccountId,
	relayers_rewards: RelayersRewards<AccountId, Currency::Balance>,
	relayer_fund_account: &AccountId,
	confirmation_fee: Currency::Balance,
) where
	AccountId: Debug + Default + Encode + PartialEq,
	Currency: CurrencyT<AccountId>,
	Currency::Balance: From<u64>,
{
	// reward every relayer except `confirmation_relayer`
	let mut confirmation_relayer_reward = Currency::Balance::zero();
	for (relayer, reward) in relayers_rewards {
		let mut relayer_reward = reward.reward;

		if relayer != *confirmation_relayer {
			// If delivery confirmation is submitted by other relayer, let's deduct confirmation fee
			// from relayer reward.
			//
			// If confirmation fee has been increased (or if it was the only component of message fee),
			// then messages relayer may receive zero reward.
			let mut confirmation_reward = confirmation_fee.saturating_mul(reward.messages.into());
			if confirmation_reward > relayer_reward {
				confirmation_reward = relayer_reward;
			}
			relayer_reward = relayer_reward.saturating_sub(confirmation_reward);
			confirmation_relayer_reward =
				confirmation_relayer_reward.saturating_add(confirmation_reward);
		} else {
			// If delivery confirmation is submitted by this relayer, let's add confirmation fee
			// from other relayers to this relayer reward.
			confirmation_relayer_reward = confirmation_relayer_reward.saturating_add(reward.reward);
			continue;
		}

		pay_relayer_reward::<Currency, _>(relayer_fund_account, &relayer, relayer_reward);
	}

	// finally - pay reward to confirmation relayer
	pay_relayer_reward::<Currency, _>(
		relayer_fund_account,
		confirmation_relayer,
		confirmation_relayer_reward,
	);
}

/// Transfer funds from relayers fund account to given relayer.
fn pay_relayer_reward<Currency, AccountId>(
	relayer_fund_account: &AccountId,
	relayer_account: &AccountId,
	reward: Currency::Balance,
) where
	AccountId: Debug,
	Currency: CurrencyT<AccountId>,
{
	if reward.is_zero() {
		return;
	}

	let pay_result = Currency::transfer(
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
