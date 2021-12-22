// This file is part of Frontier.

// Copyright (C) 2017-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg_attr(not(feature = "std"), no_std)]

// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use frame_support::Parameter;
use sp_std::{
	cmp::{Ord, Ordering, PartialEq},
	default::Default,
	ops::{Add, AddAssign, Range, Sub},
	vec::Vec,
};
/// Relayer who has enrolled the fee market
#[derive(Clone, Debug, Eq, Encode, Decode, TypeInfo)]
pub struct Relayer<AccountId, Balance> {
	pub id: AccountId,
	pub collateral: Balance,
	pub fee: Balance,
}

impl<AccountId, Balance> Relayer<AccountId, Balance> {
	pub fn new(id: AccountId, collateral: Balance, fee: Balance) -> Relayer<AccountId, Balance> {
		Relayer {
			id,
			collateral,
			fee,
		}
	}
}

impl<AccountId: Parameter, Balance: PartialOrd> PartialOrd for Relayer<AccountId, Balance> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.fee == other.fee {
			return other.collateral.partial_cmp(&self.collateral);
		}
		self.fee.partial_cmp(&other.fee)
	}
}

impl<AccountId: Parameter, Balance: Ord> Ord for Relayer<AccountId, Balance> {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.fee == other.fee {
			return self.collateral.cmp(&other.collateral);
		}
		self.fee.cmp(&other.fee)
	}
}

impl<AccountId: PartialEq, Balance: PartialEq> PartialEq for Relayer<AccountId, Balance> {
	fn eq(&self, other: &Self) -> bool {
		self.fee == other.fee && self.id == other.id && self.collateral == other.collateral
	}
}

impl<AccountId: Default, Balance: Default> Default for Relayer<AccountId, Balance> {
	fn default() -> Self {
		Relayer {
			id: AccountId::default(),
			collateral: Balance::default(),
			fee: Balance::default(),
		}
	}
}

/// Order represent cross-chain message relay task. Only support sub-sub message for now.
#[derive(Clone, Debug, Default, Encode, Decode, TypeInfo)]
pub struct Order<AccountId, BlockNumber, Balance> {
	pub lane: LaneId,
	pub message: MessageNonce,
	pub sent_time: BlockNumber,
	pub confirm_time: Option<BlockNumber>,
	pub locked_collateral: Balance,
	pub relayers: Vec<PriorRelayer<AccountId, BlockNumber, Balance>>,
}

impl<AccountId, BlockNumber, Balance> Order<AccountId, BlockNumber, Balance>
where
	BlockNumber:
		Add<Output = BlockNumber> + Copy + AddAssign + PartialOrd + Sub<Output = BlockNumber>,
	Balance: Copy + PartialOrd + Default,
	AccountId: Clone + PartialEq,
{
	pub fn new(
		lane: LaneId,
		message: MessageNonce,
		sent_time: BlockNumber,
		locked_collateral: Balance,
		assigned_relayers: Vec<Relayer<AccountId, Balance>>,
		slot: BlockNumber,
	) -> Self {
		let prior_relayers_len = assigned_relayers.len();
		let mut relayers = Vec::with_capacity(prior_relayers_len);
		let mut start_time = sent_time;

		// PriorRelayer has a duty time zone
		for i in 0..prior_relayers_len {
			if let Some(r) = assigned_relayers.get(i) {
				let p = PriorRelayer::new(r.id.clone(), r.fee, start_time, slot);

				start_time += slot;
				relayers.push(p);
			}
		}

		Self {
			lane,
			message,
			sent_time,
			confirm_time: None,
			locked_collateral,
			relayers,
		}
	}

	pub fn set_confirm_time(&mut self, confirm_time: Option<BlockNumber>) {
		self.confirm_time = confirm_time;
	}

	pub fn relayers_slice(&self) -> &[PriorRelayer<AccountId, BlockNumber, Balance>] {
		self.relayers.as_ref()
	}

	pub fn fee(&self) -> Balance {
		self.relayers
			.iter()
			.last()
			.map(|r| r.fee)
			.unwrap_or_default()
	}

	pub fn is_confirmed(&self) -> bool {
		self.confirm_time.is_some()
	}

	pub fn range_end(&self) -> Option<BlockNumber> {
		self.relayers.iter().last().map(|r| r.valid_range.end)
	}

	pub fn delivery_delay(&self) -> Option<BlockNumber> {
		if let (Some(confirm_time), Some(range_end)) = (self.confirm_time, self.range_end()) {
			if confirm_time > range_end {
				return Some(confirm_time - range_end);
			}
		}
		None
	}

	pub fn required_delivery_relayer_for_time(
		&self,
		message_confirm_time: BlockNumber,
	) -> Option<(AccountId, Balance)> {
		for prior_relayer in self.relayers.iter() {
			if prior_relayer.valid_range.contains(&message_confirm_time) {
				return Some((prior_relayer.id.clone(), prior_relayer.fee));
			}
		}
		None
	}

	#[cfg(test)]
	pub fn relayer_valid_range(&self, id: AccountId) -> Option<Range<BlockNumber>> {
		for prior_relayer in self.relayers.iter() {
			if prior_relayer.id == id {
				return Some(prior_relayer.valid_range.clone());
			}
		}
		None
	}
}

/// Relayers selected by the fee market. Each prior relayer has a valid slot, if the order can finished in time,
/// will be rewarded with more percentage. PriorRelayer are responsible for the messages relay in most time.
#[derive(Clone, Debug, Default, Encode, Decode, TypeInfo)]
pub struct PriorRelayer<AccountId, BlockNumber, Balance> {
	pub id: AccountId,
	pub fee: Balance,
	pub valid_range: Range<BlockNumber>,
}

impl<'a, AccountId, BlockNumber, Balance> PriorRelayer<AccountId, BlockNumber, Balance>
where
	BlockNumber: sp_std::ops::Add<Output = BlockNumber> + Clone,
{
	pub fn new(
		id: AccountId,
		fee: Balance,
		start_time: BlockNumber,
		slot_time: BlockNumber,
	) -> Self {
		Self {
			id,
			fee,
			valid_range: Range {
				start: start_time.clone(),
				end: start_time + slot_time,
			},
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	pub type AccountId = u64;
	pub type Balance = u64;
	pub const TEST_LANE_ID: LaneId = [0, 0, 0, 1];
	pub const TEST_MESSAGE_NONCE: MessageNonce = 0;

	#[test]
	fn test_multi_relayers_sort() {
		let r1 = Relayer::<AccountId, Balance>::new(1, 100, 30);
		let r2 = Relayer::<AccountId, Balance>::new(2, 100, 40);
		assert!(r1 < r2);

		let r3 = Relayer::<AccountId, Balance>::new(3, 150, 30);
		let r4 = Relayer::<AccountId, Balance>::new(4, 100, 30);
		assert!(r3 < r4);
	}

	#[test]
	fn test_assign_order_relayers_one() {
		let mut assigned_relayers = Vec::new();
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(1, 100, 30));
		let order = Order::new(
			TEST_LANE_ID,
			TEST_MESSAGE_NONCE,
			100,
			100,
			assigned_relayers,
			50,
		);
		assert_eq!(order.relayer_valid_range(1).unwrap(), (100..150));
	}

	#[test]
	fn test_assign_order_relayers_two() {
		let mut assigned_relayers = Vec::new();
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(1, 100, 30));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(2, 100, 30));
		let order = Order::new(
			TEST_LANE_ID,
			TEST_MESSAGE_NONCE,
			100,
			100,
			assigned_relayers,
			50,
		);
		assert_eq!(order.relayer_valid_range(1).unwrap(), (100..150));
		assert_eq!(order.relayer_valid_range(2).unwrap(), (150..200));
	}

	#[test]
	fn test_assign_order_relayers_three() {
		let mut assigned_relayers = Vec::new();
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(1, 100, 30));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(2, 100, 40));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(3, 100, 80));
		let order = Order::new(
			TEST_LANE_ID,
			TEST_MESSAGE_NONCE,
			100,
			100,
			assigned_relayers,
			50,
		);
		assert_eq!(order.relayer_valid_range(1).unwrap(), (100..150));
		assert_eq!(order.relayer_valid_range(2).unwrap(), (150..200));
		assert_eq!(order.relayer_valid_range(3).unwrap(), (200..250));
		assert_eq!(order.range_end(), Some(250));
		assert_eq!(order.fee(), 80);
	}

	#[test]
	fn test_assign_order_relayers_four() {
		let mut assigned_relayers = Vec::new();
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(1, 100, 30));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(2, 100, 30));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(3, 100, 30));
		assigned_relayers.push(Relayer::<AccountId, Balance>::new(4, 100, 30));
		let order = Order::new(
			TEST_LANE_ID,
			TEST_MESSAGE_NONCE,
			100,
			100,
			assigned_relayers,
			50,
		);
		assert_eq!(order.relayer_valid_range(1).unwrap(), (100..150));
		assert_eq!(order.relayer_valid_range(2).unwrap(), (150..200));
		assert_eq!(order.relayer_valid_range(3).unwrap(), (200..250));
		assert_eq!(order.relayer_valid_range(4).unwrap(), (250..300));
	}
}
