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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as BscBridge;

use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use bsc_primitives::BSCHeader;
use codec::Decode;
use frame_benchmarking::benchmarks;
use frame_support::traits::Get;
use frame_system::RawOrigin;
use sp_std::{vec, vec::Vec};

benchmarks! {
	verify_and_update_authority_set_signed {
		let genesis_header = serde_json::from_str(r#"{
				"difficulty": "0x2",
				"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c0670403d7dfc4c816a313885fe04b850f96f27b2e9fd88b147c882ad7caf9b964abfe6543625fcca73b56fe29d3046831574b0681d52bf5383d6f2187b6276c100",
				"gasLimit": "0x38ff37a",
				"gasUsed": "0x1364017",
				"logsBloom": "0x2c30123db854d838c878e978cd2117896aa092e4ce08f078424e9ec7f2312f1909b35e579fb2702d571a3be04a8f01328e51af205100a7c32e3dd8faf8222fcf03f3545655314abf91c4c0d80cea6aa46f122c2a9c596c6a99d5842786d40667eb195877bbbb128890a824506c81a9e5623d4355e08a16f384bf709bf4db598bbcb88150abcd4ceba89cc798000bdccf5cf4d58d50828d3b7dc2bc5d8a928a32d24b845857da0b5bcf2c5dec8230643d4bec452491ba1260806a9e68a4a530de612e5c2676955a17400ce1d4fd6ff458bc38a8b1826e1c1d24b9516ef84ea6d8721344502a6c732ed7f861bb0ea017d520bad5fa53cfc67c678a2e6f6693c8ee",
				"miner": "0xe9ae3261a475a27bb1028f140bc2a7c843318afd",
				"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
				"nonce": "0x0000000000000000",
				"number": "0x7594c8",
				"parentHash": "0x5cb4b6631001facd57be810d5d1383ee23a31257d2430f097291d25fc1446d4f",
				"receiptsRoot": "0x1bfba16a9e34a12ff7c4b88be484ccd8065b90abea026f6c1f97c257fdb4ad2b",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"stateRoot": "0xa6cd7017374dfe102e82d2b3b8a43dbe1d41cc0e4569f3dc45db6c4e687949ae",
				"timestamp": "0x60ac7137",
				"transactionsRoot": "0x657f5876113ac9abe5cf0460aa8d6b3b53abfc336cea4ab3ee594586f8b584ca"
			}"#).unwrap();

		let header = serde_json::from_str::<BSCHeader>(
			r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c675b589d9452d45327429ff925359ca25b1cc0245ffb869dbbcffb5a0d3c72f103a1dcb28b105926c636747dbc265f8dda0090784be3febffdd7909aa6f416d200",
			"gasLimit": "0x391a17f",
			"gasUsed": "0x151a7b2",
			"hash": "0x2af8376a302e60d766a74c4b4bbc98be08611865f3545da840062eabac511aff",
			"logsBloom": "0x4f7a466ebd89d672e9d73378d03b85204720e75e9f9fae20b14a6c5faf1ca5f8dd50d5b1077036e1596ef22860dca322ddd28cc18be6b1638e5bbddd76251bde57fc9d06a7421b5b5d0d88bcb9b920adeed3dbb09fd55b16add5f588deb6bcf64bbd59bfab4b82517a1c8fc342233ba17a394a6dc5afbfd0acfc443a4472212640cf294f9bd864a4ac85465edaea789a007e7f17c231c4ae790e2ced62eaef10835c4864c7e5b64ad9f511def73a0762450659825f60ceb48c9e88b6e77584816a2eb57fdaba54b71d785c8b85de3386e544ccf213ecdc942ef0193afae9ecee93ff04ff9016e06a03393d4d8ae14a250c9dd71bf09fee6de26e54f405d947e1",
			"miner": "0x72b61c6014342d914470ec7ac2975be345796c2b",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759590",
			"parentHash": "0x898c926e404409d6151d0e0ea156770fdaa2b31f8115b5f20bcb1b6cb4dc34c3",
			"receiptsRoot": "0x04aea8f3d2471b7ae64bce5dde7bb8eafa4cf73c65eab5cc049f92b3fda65dcc",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x5d03a66ae7fdcc6bff51e4c0cf40c6ec2d291090bddd9073ca4203d84b099bb9",
			"timestamp": "0x60ac738f",
			"totalDifficulty": "0xea4b80",
			"transactionsRoot": "0xb3db66bc49eac913dbdbe8aeaaee891762a6c5c28990c3f5f161726a8cb1c41d"
		}"#,
		).unwrap();

		let initial_authority_set =
				<BscBridge<T>>::extract_authorities(&genesis_header).unwrap();

		Authorities::<T>::put(&initial_authority_set);
		FinalizedAuthority::<T>::put(&initial_authority_set);
		FinalizedCheckpoint::<T>::put(&genesis_header);
		AuthoritiesOfRound::<T>::insert(
			&genesis_header.number / T::BSCConfiguration::get().epoch_length,
			(0u32..initial_authority_set.len() as u32).collect::<Vec<u32>>(),
			);
		let caller: T::AccountId = T::AccountId::decode(&mut &[0; 32][..]).unwrap_or_default();

	}:_(RawOrigin::Signed(caller), vec![header])
}
