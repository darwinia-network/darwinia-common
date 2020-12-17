// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia.  If not, see <https://www.gnu.org/licenses/>.

//! Relay Authorities Primitives

// --- std ---
use core::fmt::Debug;
// --- crates ---
use codec::{Decode, Encode, FullCodec};
// --- substrate ---
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub trait Sign<BlockNumber> {
	type Signature: Clone + Debug + PartialEq + FullCodec;
	type Message: Clone + Debug + Default + PartialEq + FullCodec;
	type Signer: Clone + Debug + PartialEq + FullCodec;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message;

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool;
}

pub trait RelayAuthorityProtocol<BlockNumber> {
	fn new_mmr_to_sign(block_number: BlockNumber);
}
// Only for test
impl<BlockNumber> RelayAuthorityProtocol<BlockNumber> for () {
	fn new_mmr_to_sign(_: BlockNumber) {}
}

pub trait MMR<BlockNumber, Root> {
	fn get_root(block_number: BlockNumber) -> Option<Root>;
}
// Only for test
impl<BlockNumber, Root> MMR<BlockNumber, Root> for () {
	fn get_root(_: BlockNumber) -> Option<Root> {
		None
	}
}

// Avoid duplicate type
// Use `RelayAuthority` instead `Authority`
#[derive(Clone, Encode, Decode, RuntimeDebug)]
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
