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

//! Relay Authorities Primitives

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use codec::{Decode, Encode, FullCodec};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{de::DeserializeOwned, Serialize};
// --- paritytech ---
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::prelude::*;

pub type OpCode = [u8; 4];
pub type Term = u32;

pub trait Sign<BlockNumber> {
	type Signature: Clone + Debug + PartialEq + FullCodec + TypeInfo;
	type Message: Clone + Debug + Default + PartialEq + FullCodec + TypeInfo;
	#[cfg(feature = "std")]
	type Signer: Clone
		+ Debug
		+ Default
		+ Ord
		+ PartialEq
		+ FullCodec
		+ TypeInfo
		+ DeserializeOwned
		+ Serialize;
	#[cfg(not(feature = "std"))]
	type Signer: Clone + Debug + Default + Ord + PartialEq + FullCodec + TypeInfo;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message;

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool;
}

pub trait RelayAuthorityProtocol<BlockNumber> {
	type Signer;

	fn schedule_mmr_root(block_number: BlockNumber) -> DispatchResult;

	fn check_authorities_change_to_sync(
		term: Term,
		authorities: Vec<Self::Signer>,
	) -> DispatchResult;

	fn sync_authorities_change() -> DispatchResult;
}

// Avoid duplicate type
// Use `RelayAuthority` instead `Authority`
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct RelayAuthority<AccountId, Signer, RingBalance, BlockNumber> {
	pub account_id: AccountId,
	pub signer: Signer,
	pub stake: RingBalance,
	pub term: BlockNumber,
}
impl<AccountId, Signer, RingBalance, BlockNumber> PartialEq
	for RelayAuthority<AccountId, Signer, RingBalance, BlockNumber>
where
	AccountId: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.account_id == other.account_id
	}
}
impl<AccountId, Signer, RingBalance, BlockNumber> PartialEq<AccountId>
	for RelayAuthority<AccountId, Signer, RingBalance, BlockNumber>
where
	AccountId: PartialEq,
{
	fn eq(&self, account_id: &AccountId) -> bool {
		&self.account_id == account_id
	}
}
