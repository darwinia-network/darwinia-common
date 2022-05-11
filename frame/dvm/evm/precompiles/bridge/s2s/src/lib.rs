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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::Encode;
use evm::ExitRevert;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{PrecompileHelper, StateMutability};
use darwinia_support::{
	evm::IntoAccountId,
	s2s::{LatestMessageNoncer, RelayMessageSender},
};
use dp_contract::{
	abi_util::{abi_decode_bytes4, abi_encode_bytes, abi_encode_u64},
	mapping_token_factory::s2s::{S2sRemoteUnlockInfo, S2sSendMessageParams},
};
use dp_s2s::{BackingParamsEncoder, CreatePayload};
// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_runtime::messages::DispatchFeePayment;
use fp_evm::{
	Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use frame_support::sp_runtime::SaturatedConversion;
use sp_core::H160;
use sp_runtime::{MultiSignature, MultiSigner};
use sp_std::vec::Vec;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	OutboundLatestGeneratedNonce = "outbound_latest_generated_nonce(bytes4)",
	InboundLatestReceivedNonce = "inbound_latest_received_nonce(bytes4)",
	EncodeUnlockFromRemoteDispatchCall =
		"encode_unlock_from_remote_dispatch_call(uint32,uint64,uint32,address,bytes,uint256)",
	EncodeSendMessageDispatchCall =
		"encode_send_message_dispatch_call(uint32,bytes4,bytes,uint256)",
}

pub struct Sub2SubBridge<T, S, P> {
	_marker: PhantomData<(T, S, P)>,
}

impl<T, S, P> Precompile for Sub2SubBridge<T, S, P>
where
	T: darwinia_evm::Config,
	S: RelayMessageSender + LatestMessageNoncer + BackingParamsEncoder,
	P: CreatePayload<T::AccountId, MultiSigner, MultiSignature>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::OutboundLatestGeneratedNonce =>
				Self::outbound_latest_generated_nonce(data, &mut helper)?,
			Action::InboundLatestReceivedNonce =>
				Self::inbound_latest_received_nonce(data, &mut helper)?,
			Action::EncodeUnlockFromRemoteDispatchCall =>
				Self::encode_unlock_from_remote_dispatch_call(data, context.caller, &mut helper)?,
			Action::EncodeSendMessageDispatchCall =>
				Self::encode_send_message_dispatch_call(data, &mut helper)?,
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output,
			logs: Default::default(),
		})
	}
}

impl<T, S, P> Sub2SubBridge<T, S, P>
where
	T: darwinia_evm::Config,
	S: RelayMessageSender + LatestMessageNoncer + BackingParamsEncoder,
	P: CreatePayload<T::AccountId, MultiSigner, MultiSignature>,
{
	fn outbound_latest_generated_nonce(
		data: &[u8],
		helper: &mut PrecompileHelper<T>,
	) -> Result<Vec<u8>, PrecompileFailure> {
		// Storage: ParityBridgeMessages OutboundLanes (r:1 w:0)
		helper.record_gas(1, 0)?;

		let lane_id = abi_decode_bytes4(data).map_err(|_| helper.revert("decode failed"))?;
		let nonce = <S as LatestMessageNoncer>::outbound_latest_generated_nonce(lane_id);
		Ok(abi_encode_u64(nonce))
	}

	fn inbound_latest_received_nonce(
		data: &[u8],
		helper: &mut PrecompileHelper<T>,
	) -> Result<Vec<u8>, PrecompileFailure> {
		// Storage: ParityBridgeMessages INboundLanes (r:1 w:0)
		helper.record_gas(1, 0)?;

		let lane_id = abi_decode_bytes4(data).map_err(|_| helper.revert("decode failed"))?;
		let nonce = <S as LatestMessageNoncer>::inbound_latest_received_nonce(lane_id);
		Ok(abi_encode_u64(nonce))
	}

	fn encode_unlock_from_remote_dispatch_call(
		data: &[u8],
		caller: H160,
		helper: &mut PrecompileHelper<T>,
	) -> Result<Vec<u8>, PrecompileFailure> {
		helper.record_gas(0, 0)?;

		let unlock_info = S2sRemoteUnlockInfo::abi_decode(data)
			.map_err(|_| helper.revert("decode unlock failed"))?;
		let payload = P::create(
			CallOrigin::SourceAccount(T::IntoAccountId::into_account_id(caller)),
			unlock_info.spec_version,
			unlock_info.weight,
			<S as BackingParamsEncoder>::encode_unlock_from_remote(
				unlock_info.original_token,
				unlock_info.amount,
				unlock_info.recipient,
			),
			DispatchFeePayment::AtSourceChain,
		)
		.map_err(|_| helper.revert("decode remote unlock failed"))?;
		Ok(abi_encode_bytes(payload.encode().as_slice()))
	}

	fn encode_send_message_dispatch_call(
		data: &[u8],
		helper: &mut PrecompileHelper<T>,
	) -> Result<Vec<u8>, PrecompileFailure> {
		helper.record_gas(0, 0)?;

		let params = S2sSendMessageParams::decode(data)
			.map_err(|_| helper.revert("decode send message info failed"))?;
		let encoded = <S as RelayMessageSender>::encode_send_message(
			params.pallet_index,
			params.lane_id,
			params.payload,
			params.fee.low_u128().saturated_into(),
		)
		.map_err(|_| helper.revert("encode send message call failed"))?;
		Ok(abi_encode_bytes(encoded.as_slice()))
	}
}
