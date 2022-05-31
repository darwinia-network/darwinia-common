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

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::vec;

use ethereum_primitives::{header::EthereumHeader, receipt::ReceiptProof, H160, H256};
use sp_std::str::FromStr;

use darwinia_bridge_ethereum::MMRProof;
use darwinia_support::traits::EthereumReceipt as EthereumReceiptT;

use codec::{Decode, Encode};

// should mock ethereum relay first
// https://github.com/darwinia-network/darwinia-common/blob/master/frame/bridge/ethereum/relay/src/lib.rs
// replace line `let mmr_root = Self::confirmed_header_parcel_of(mmr_proof.last_leaf_index + 1)` as
// for generic simulation, we choose different mmr root for register and redeem.
// the mmr root at 10254000 is used for register transaction proof, and the other is for redeem
// transaction proof.
/*
 *
 * let mmr_root = if ethereum_header.number == 10254000 {
 *     H256::from_str("4daf1aacca87c0829a1e55f3ebcc44f49b158e92e9cbb474a60719070b225e6e").
 * unwrap() } else {
 *     H256::from_str("e860637b3d94a6606fd1d7cd7d86ca3bb37625c3b6a88de3afe118f936acbc35").
 * unwrap() };
 *
 */
benchmarks! {
	register_erc20 {
		let caller = whitelisted_caller();
		let header : EthereumHeader = Decode::decode(&mut array_bytes::hex2bytes_unchecked(mock_header::REGISTER_HEADER).as_slice()).unwrap();

		let receipt_proof = ReceiptProof {
			index: 11,
			proof: array_bytes::hex2bytes_unchecked(mock_header::REGISTER_RECEIPT_PROOF),
			header_hash: H256::from_str("0x1772823cfe05d72d414c50ace0a9c7481c367480a3eecea22aedf08fd392d452").unwrap(),
		};
		let mmr_proof = MMRProof {
			member_leaf_index: 10254000,
			last_leaf_index: 10254000,
			proof: vec![
				H256::from_str("36f3d834cbe12a5a20b063c432b88f5506bdce03b93fa3aa035a5d82fd50177c").unwrap(),
				H256::from_str("31be4a0f0d61cc2e97e19996d949259faa0b9c449acce29bc675a0cf0d429b2a").unwrap(),
				H256::from_str("21877504b36bbe17e7aaaec85960760e23145db31ccaf34869081aa0adb19824").unwrap(),
				H256::from_str("f6de10c1c6ba47d4b752c57d5833f5631d377f996cc92c8444e8d6c46976ad06").unwrap(),
				H256::from_str("6fba92edaed7f3174a5bb8b07973587d9b1d91446cd09d72ce71e4331ebf5124").unwrap(),
				H256::from_str("a7121b92d3428733616b27d0d2ba890b0dd1f3214a0843b2e87bf53f3c6687b8").unwrap(),
				H256::from_str("0d3bc625dd98dc9611fb33c0dcc1da283f662f8a9c2dc4a483430a404ad81b99").unwrap(),
				H256::from_str("c943667292d18518170e94823577ef31c9e0c895fc2610fa52d0f23200f7b455").unwrap(),
				H256::from_str("8bfdcd4ebd32d703593a682ec0dc38edd761dc55777c8c92235e4d29a87c589c").unwrap(),
				H256::from_str("1ebc80a9e579049f0e349aedcfbea7a850a0ad62d57e5970aa6cb3d02f4e99e8").unwrap(),
				H256::from_str("1cb8ff1fcb087edc5d516da846f8f4c1eb5a1799837b8fab7693ec083d65f3d2").unwrap(),
				H256::from_str("52713d8398aafc081da132f172682f9c72b36b9f7fd6def567358a989877bc2f").unwrap(),
			],
		};
		let mut proof: vec::Vec<u8> = (header, receipt_proof, mmr_proof).encode();
		let proof_thing: EthereumReceiptProofThing<T>  = Decode::decode(&mut proof.as_slice()).unwrap();
	}: _(RawOrigin::Signed(caller), proof_thing)

	redeem_erc20 {
		let caller = whitelisted_caller();
		let header : EthereumHeader = Decode::decode(&mut array_bytes::hex2bytes_unchecked(mock_header::SEND_HEADER).as_slice()).unwrap();

		let receipt_proof = ReceiptProof {
			index: 38,
			proof: array_bytes::hex2bytes_unchecked(mock_header::SEND_RECEIPT_PROOF),
			header_hash: H256::from_str("0x5922cd234fcb6cfc0ea81365858941a8b426169ce9998158313239af0ca0a763").unwrap(),
		};
		let mmr_proof = MMRProof {
			member_leaf_index: 10254219,
			last_leaf_index: 10254219,
			proof: vec![
				H256::from_str("36f3d834cbe12a5a20b063c432b88f5506bdce03b93fa3aa035a5d82fd50177c").unwrap(),
				H256::from_str("31be4a0f0d61cc2e97e19996d949259faa0b9c449acce29bc675a0cf0d429b2a").unwrap(),
				H256::from_str("21877504b36bbe17e7aaaec85960760e23145db31ccaf34869081aa0adb19824").unwrap(),
				H256::from_str("f6de10c1c6ba47d4b752c57d5833f5631d377f996cc92c8444e8d6c46976ad06").unwrap(),
				H256::from_str("6fba92edaed7f3174a5bb8b07973587d9b1d91446cd09d72ce71e4331ebf5124").unwrap(),
				H256::from_str("a7121b92d3428733616b27d0d2ba890b0dd1f3214a0843b2e87bf53f3c6687b8").unwrap(),
				H256::from_str("0d3bc625dd98dc9611fb33c0dcc1da283f662f8a9c2dc4a483430a404ad81b99").unwrap(),
				H256::from_str("c943667292d18518170e94823577ef31c9e0c895fc2610fa52d0f23200f7b455").unwrap(),
				H256::from_str("8bfdcd4ebd32d703593a682ec0dc38edd761dc55777c8c92235e4d29a87c589c").unwrap(),
				H256::from_str("b2de0beaebe230e8c32d147af3fda4b695fdf7c4c3457e061d24a516fa9d24e8").unwrap(),
				H256::from_str("3ea43917f7c688caf790bd826943260a03044a0c115269313735ccfd51e8e1d7").unwrap(),
				H256::from_str("c0b5c9b33a898548d924ffce2ad7f141ae2d65e6e88b48116e8114c7a1d9c598").unwrap(),
				H256::from_str("810712e4402c52e614fdece5fa985c8ceb0a829018d9f22d52aae5f5feafd9aa").unwrap(),
				H256::from_str("ee2b452c196922e9e226e79d8a7728ead52e98c82caa2510dc8b9652138d0c0f").unwrap(),
			],
		};
		let mut proof: vec::Vec<u8> = (header, receipt_proof, mmr_proof).encode();
		let proof_thing: EthereumReceiptProofThing<T>  = Decode::decode(&mut proof.as_slice()).unwrap();
	}: _(RawOrigin::Signed(caller), proof_thing)

	// the test data is from tx:
	// https://ropsten.etherscan.io/tx/0x5999253ecbe82b26800534b78567058352cc741c3475c94dba014f5971b5933c
	deposit_burn_token_event_from_precompile {
		let factory = H160::from_str("E1586e744b99bF8e4C981DfE4dD4369d6f8Ed88A").unwrap();
		let caller = <T as darwinia_evm::Config>::DeriveSubAccount::derive_account_id(factory);
		let input = array_bytes::hex2bytes_unchecked("0x9fd728bf917e2e38000000000000000000000000b2bea2358d817dae01b0fd0dc3aecb25910e65aa000000000000000000000000a26e0ff781f2d39cc9a9e255a2e74573945c2d790000000000000000000000003bdeafc230636b5d4ecc4e5688dbbf78d68d19e6");
	}: _(RawOrigin::Signed(caller), input)

	set_mapping_factory_address {
		let address = hex_into_unchecked("0000000000000000000000000000000000000001");
	}: _(RawOrigin::Root, address)

	set_ethereum_backing_address {
		let address = hex_into_unchecked("0000000000000000000000000000000000000001");
	}: _(RawOrigin::Root, address)
}
