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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod evm;
pub mod macros;
pub mod structs;
#[cfg(feature = "testing")]
pub mod testing;
pub mod traits;

pub mod balance {
	pub use crate::structs::{StakingLock, Unbonding};
}

// TODO: Should we move this to `s2s-primitives`?
pub mod s2s {
	// --- crates.io ---
	use codec::Encode;
	// --- darwinia-network ---
	use ethereum_primitives::H256;
	// --- paritytech ---
	use bp_messages::{LaneId, MessageNonce};
	use bp_runtime::{derive_account_id, ChainId, SourceAccount};
	use frame_support::ensure;
	use sp_runtime::{
		traits::{BadOrigin, Convert},
		DispatchError,
	};
	use sp_std::cmp::PartialEq;

	pub trait LatestMessageNoncer {
		fn outbound_latest_generated_nonce(lane_id: LaneId) -> MessageNonce;
		fn inbound_latest_received_nonce(lane_id: LaneId) -> MessageNonce;
	}

	pub fn ensure_source_account<AccountId, Converter>(
		chain_id: ChainId,
		source_account: AccountId,
		derived_account: &AccountId,
	) -> Result<(), DispatchError>
	where
		AccountId: PartialEq + Encode,
		Converter: Convert<H256, AccountId>,
	{
		let hex_id =
			derive_account_id::<AccountId>(chain_id, SourceAccount::Account(source_account));
		let target_id = Converter::convert(hex_id);
		ensure!(&target_id == derived_account, BadOrigin);
		Ok(())
	}
}
