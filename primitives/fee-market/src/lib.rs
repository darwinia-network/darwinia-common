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

use bp_messages::{LaneId, MessageNonce};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_std::{
	cmp::{Ord, Ordering, PartialEq},
	default::Default,
	ops::Range,
	vec::Vec,
};

// Fee market's order relayers assign has tightly relationship with this value.
// Changing this number should be much carefully to avoid unexpected runtime behavior.
pub const MIN_RELAYERS_NUMBER: usize = 3;

#[derive(Encode, Decode, Clone, Eq, Debug, Copy)]
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
#[derive(Clone, Encode, Decode, Default)]
pub struct Order<AccountId, BlockNumber, Balance> {
	pub lane: LaneId,
	pub message: MessageNonce,
	pub sent_time: BlockNumber,
	pub confirm_time: Option<BlockNumber>,
	pub relayers: Vec<PriorRelayer<AccountId, BlockNumber, Balance>>,
}

impl<AccountId, BlockNumber, Balance> Order<AccountId, BlockNumber, Balance>
where
	BlockNumber: sp_std::ops::Add<Output = BlockNumber> + Copy + std::ops::AddAssign,
	Balance: Copy,
	AccountId: Clone,
{
	pub fn new(
		lane: LaneId,
		message: MessageNonce,
		sent_time: BlockNumber,
		assigned_relayers: Vec<Relayer<AccountId, Balance>>,
		slot_time: BlockNumber,
	) -> Self {
		let mut relayers = Vec::with_capacity(MIN_RELAYERS_NUMBER);
		if assigned_relayers.len() == MIN_RELAYERS_NUMBER {
			let mut start_time = sent_time;
			for i in 0..MIN_RELAYERS_NUMBER {
				if let Some(r) = assigned_relayers.get(i) {
					let p = PriorRelayer::new(r.id.clone(), r.fee, start_time, slot_time);

					start_time += slot_time;
					relayers.push(p);
				}
			}
		}

		Self {
			lane,
			message,
			sent_time,
			confirm_time: None,
			relayers,
		}
	}

	pub fn set_confirm_time(&mut self, confirm_time: Option<BlockNumber>) {
		self.confirm_time = confirm_time;
	}

	pub fn relayers_slice(&self) -> &[PriorRelayer<AccountId, BlockNumber, Balance>] {
		self.relayers.as_ref()
	}

	pub fn relayers(
		&self,
	) -> (
		Option<&PriorRelayer<AccountId, BlockNumber, Balance>>,
		Option<&PriorRelayer<AccountId, BlockNumber, Balance>>,
		Option<&PriorRelayer<AccountId, BlockNumber, Balance>>,
	) {
		(
			self.relayers.get(0),
			self.relayers.get(1),
			self.relayers.get(2),
		)
	}

	pub fn is_confirmed(&self) -> bool {
		self.confirm_time.is_some()
	}
}
#[derive(Clone, Encode, Decode, Default)]
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
