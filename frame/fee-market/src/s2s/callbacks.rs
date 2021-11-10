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

use crate::*;
use bp_messages::{
	source_chain::{OnDeliveryConfirmed, OnMessageAccepted},
	DeliveredMessages, LaneId, MessageNonce,
};
use dp_fee::Order;

pub struct FeeMarketMessageAcceptedHandler<T>(PhantomData<T>);
impl<T: Config> OnMessageAccepted for FeeMarketMessageAcceptedHandler<T> {
	// Called when the message is accepted by message pallet
	fn on_messages_accepted(lane: &LaneId, message: &MessageNonce) -> Weight {
		// Create a new order based on the latest block, assign relayers which have priority to relaying
		let now = frame_system::Pallet::<T>::block_number();
		if let Some(assigned_relayers) = <Pallet<T>>::assigned_relayers() {
			let order = Order::new(
				*lane,
				*message,
				now,
				assigned_relayers.clone(),
				T::Slot::get(),
			);
			// Store the create order
			<Orders<T>>::insert((order.lane, order.message), order.clone());
			// Update relayer orders and order_capacity
			Pallet::<T>::relayer_accept_order(&order);
		}

		// TODO: The returned weight should be more accurately. See: https://github.com/darwinia-network/darwinia-common/issues/911
		<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}
}

pub struct FeeMarketMessageConfirmedHandler<T>(PhantomData<T>);

impl<T: Config> OnDeliveryConfirmed for FeeMarketMessageConfirmedHandler<T> {
	fn on_messages_delivered(lane: &LaneId, delivered_messages: &DeliveredMessages) -> Weight {
		let now = frame_system::Pallet::<T>::block_number();
		for message_nonce in delivered_messages.begin..=delivered_messages.end {
			if let Some(order) = <Orders<T>>::get((lane, message_nonce)) {
				if !order.is_confirmed() {
					<Orders<T>>::mutate((lane, message_nonce), |order| match order {
						Some(order) => order.set_confirm_time(Some(now)),
						None => {}
					});
					<ConfirmedMessagesThisBlock<T>>::append((lane, message_nonce));

					let assigned_relayers_ids: Vec<T::AccountId> = order
						.relayers_slice()
						.iter()
						.map(|r| r.id.clone())
						.collect();
					// todo: should we move this part after the reward?
					Pallet::<T>::relayer_finish_order(
						&order.lane,
						&order.message,
						&assigned_relayers_ids,
					);
				}
			}
		}

		// TODO: The returned weight should be more accurately. See: https://github.com/darwinia-network/darwinia-common/issues/911
		<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}
}
