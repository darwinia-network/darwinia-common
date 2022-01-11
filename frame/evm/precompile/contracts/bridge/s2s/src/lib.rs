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
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
// --- darwinia-network ---
use darwinia_evm_precompile_utils::DvmInputParser;
use darwinia_support::{
	evm::IntoAccountId,
	s2s::{LatestMessageNoncer, RelayMessageSender},
};
use dp_contract::{
	abi_util::{abi_decode_bytes4, abi_encode_bytes, abi_encode_u64},
	mapping_token_factory::s2s::{S2sRemoteUnlockInfo, S2sSendMessageParams},
	s2s_backing::{S2sIssueTokenParams, S2sRegisterTokenParams},
};
use dp_s2s::{CallParams, CreatePayload};
// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_runtime::messages::DispatchFeePayment;
use fp_evm::Precompile;
use frame_support::sp_runtime::SaturatedConversion;
use sp_core::H160;
use sp_std::vec::Vec;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	OutboundLatestGeneratedNonce = "outbound_latest_generated_nonce(bytes4)",
	InboundLatestReceivedNonce = "inbound_latest_received_nonce(bytes4)",
	EncodeSendMessageDispatchCall =
		"encode_send_message_dispatch_call(uint32,bytes4,bytes,uint256)",
	// issuing used
	EncodeUnlockFromRemoteDispatchCall =
		"encode_unlock_from_remote_dispatch_call(uint32,uint64,uint32,address,bytes,uint256)",
	// backing used
	EncodeRegisterFromRemoteDispatchCall = "encode_register_from_remote_dispatch_call",
	EncodeIssueFromRemoteDispatchCall = "encode_issue_from_remote_dispatch_call",
}

/// The contract address: 0000000000000000000000000000000000000018
pub struct Sub2SubBridge<T, S> {
	_marker: PhantomData<(T, S)>,
}

impl<T, S> Precompile for Sub2SubBridge<T, S>
where
	T: from_substrate_issuing::Config,
	T: to_substrate_backing::Config,
	S: RelayMessageSender + LatestMessageNoncer,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		let dvm_parser = DvmInputParser::new(&input)?;

		let output = match Action::from_u32(dvm_parser.selector)? {
			Action::OutboundLatestGeneratedNonce => {
				Self::outbound_latest_generated_nonce(&dvm_parser)?
			}
			Action::InboundLatestReceivedNonce => Self::inbound_latest_received_nonce(&dvm_parser)?,
			Action::EncodeUnlockFromRemoteDispatchCall => {
				Self::encode_unlock_from_remote_dispatch_call(&dvm_parser, context.caller)?
			}
			Action::EncodeSendMessageDispatchCall => {
				Self::encode_send_message_dispatch_call(&dvm_parser)?
			}
			Action::EncodeRegisterFromRemoteDispatchCall => {
				Self::encode_register_from_remote_dispatch_call(&dvm_parser, context.caller)?
			}
			Action::EncodeIssueFromRemoteDispatchCall => {
				Self::encode_issue_from_remote_dispatch_call(&dvm_parser, context.caller)?
			}
		};

		// estimate a cost for this encoder process
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: 20000,
			output,
			logs: Default::default(),
		})
	}
}

impl<T, S> Sub2SubBridge<T, S>
where
	T: from_substrate_issuing::Config,
	T: to_substrate_backing::Config,
	S: RelayMessageSender + LatestMessageNoncer,
{
	fn outbound_latest_generated_nonce(dvm_parser: &DvmInputParser) -> Result<Vec<u8>, ExitError> {
		let lane_id = abi_decode_bytes4(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode lane id failed".into()))?;
		let nonce = <S as LatestMessageNoncer>::outbound_latest_generated_nonce(lane_id);
		Ok(abi_encode_u64(nonce))
	}

	fn inbound_latest_received_nonce(dvm_parser: &DvmInputParser) -> Result<Vec<u8>, ExitError> {
		let lane_id = abi_decode_bytes4(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode lane id failed".into()))?;
		let nonce = <S as LatestMessageNoncer>::inbound_latest_received_nonce(lane_id);
		Ok(abi_encode_u64(nonce))
	}

	fn encode_unlock_from_remote_dispatch_call(
		dvm_parser: &DvmInputParser,
		caller: H160,
	) -> Result<Vec<u8>, ExitError> {
		let unlock_info = S2sRemoteUnlockInfo::abi_decode(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode unlock info failed".into()))?;
		let payload = <T as from_substrate_issuing::Config>::OutboundPayloadCreator::create(
			CallOrigin::SourceAccount(T::IntoAccountId::into_account_id(caller)),
			unlock_info.spec_version,
			unlock_info.weight,
			CallParams::S2sBackingPalletUnlockFromRemote(
				unlock_info.original_token,
				unlock_info.amount,
				unlock_info.recipient,
			),
			DispatchFeePayment::AtSourceChain,
		)
		.map_err(|_| ExitError::Other("encode remote unlock failed".into()))?;
		Ok(abi_encode_bytes(payload.encode().as_slice()))
	}

	fn encode_send_message_dispatch_call(
		dvm_parser: &DvmInputParser,
	) -> Result<Vec<u8>, ExitError> {
		let params = S2sSendMessageParams::decode(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode send message info failed".into()))?;
		let encoded = <S as RelayMessageSender>::encode_send_message(
			params.pallet_index,
			params.lane_id,
			params.payload,
			params.fee.low_u128().saturated_into(),
		)
		.map_err(|_| ExitError::Other("encode send message call failed".into()))?;
		Ok(abi_encode_bytes(encoded.as_slice()))
	}

	fn encode_register_from_remote_dispatch_call(
		dvm_parser: &DvmInputParser,
		caller: H160,
	) -> Result<Vec<u8>, ExitError> {
		let register_info = S2sRegisterTokenParams::abi_decode(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode register info failed".into()))?;
		let payload = <T as to_substrate_backing::Config>::OutboundPayloadCreator::create(
			CallOrigin::SourceAccount(T::IntoAccountId::into_account_id(caller)),
			register_info.spec_version,
			register_info.weight,
			CallParams::S2sIssuingPalletRegisterFromRemote(register_info.token_metadata),
			DispatchFeePayment::AtSourceChain,
		)
		.map_err(|_| ExitError::Other("encode remote register failed".into()))?;
		Ok(abi_encode_bytes(payload.encode().as_slice()))
	}

	fn encode_issue_from_remote_dispatch_call(
		dvm_parser: &DvmInputParser,
		caller: H160,
	) -> Result<Vec<u8>, ExitError> {
		let issue_info = S2sIssueTokenParams::abi_decode(dvm_parser.input)
			.map_err(|_| ExitError::Other("decode register info failed".into()))?;
		let payload = <T as to_substrate_backing::Config>::OutboundPayloadCreator::create(
			CallOrigin::SourceAccount(T::IntoAccountId::into_account_id(caller)),
			issue_info.spec_version,
			issue_info.weight,
			CallParams::S2sIssuingPalletIssueFromRemote(
				issue_info.token,
				issue_info.amount,
				issue_info.recipient,
			),
			DispatchFeePayment::AtSourceChain,
		)
		.map_err(|_| ExitError::Other("encode remote issue failed".into()))?;
		Ok(abi_encode_bytes(payload.encode().as_slice()))
	}
}
