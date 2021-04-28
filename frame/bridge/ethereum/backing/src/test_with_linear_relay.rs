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

//! Tests for ethereum-backing.

// --- crates.io ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{assert_err, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::{traits::Dispatchable, AccountId32, RuntimeDebug};
// --- darwinia ---
use crate::{pallet::*, *};
use darwinia_staking::{RewardDestination, StakingBalance, StakingLedger, TimeDepositItem};
use darwinia_support::balance::*;
use ethereum_primitives::{
	header::EthereumHeader, receipt::EthereumReceiptProof, EthereumNetworkType,
};

decl_tests!(EthereumRelay: darwinia_ethereum_linear_relay::{Pallet, Call, Storage});

frame_support::parameter_types! {
	pub const EthereumLinearRelayModuleId: ModuleId = ModuleId(*b"da/ethli");
	pub const EthereumNetwork: EthereumNetworkType = EthereumNetworkType::Ropsten;
}
impl darwinia_ethereum_linear_relay::Config for Test {
	type ModuleId = EthereumLinearRelayModuleId;
	type Event = ();
	type EthereumNetwork = EthereumNetwork;
	type Call = Call;
	type Currency = Ring;
	type WeightInfo = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
	fn default() -> Self {
		Self
	}
}
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		darwinia_ethereum_backing::GenesisConfig::<Test> {
			token_redeem_address: array_bytes::hex2array_unchecked!(
				"0x49262B932E439271d05634c32978294C7Ea15d0C",
				20
			)
			.into(),
			deposit_redeem_address: array_bytes::hex2array_unchecked!(
				"0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e",
				20
			)
			.into(),
			set_authorities_address: array_bytes::hex2array_unchecked!(
				"0xE4A2892599Ad9527D76Ce6E26F93620FA7396D85",
				20
			)
			.into(),
			ring_token_address: array_bytes::hex2array_unchecked!(
				"0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
				20
			)
			.into(),
			kton_token_address: array_bytes::hex2array_unchecked!(
				"0x1994100c58753793D52c6f457f189aa3ce9cEe94",
				20
			)
			.into(),
			backed_ring: 20000000000000,
			backed_kton: 5000000000000,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

#[test]
fn genesis_linear_config_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(EthereumBacking::pot::<Ring>(), 20000000000000);
		assert_eq!(EthereumBacking::pot::<Kton>(), 5000000000000);
	});
}

#[test]
fn verify_linear_parse_token_redeem_proof() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(EthereumRelay::set_number_of_blocks_safe(RawOrigin::Root.into(), 0));

			// https://ropsten.etherscan.io/tx/0x1d3ef601b9fa4a7f1d6259c658d0a10c77940fa5db9e10ab55397eb0ce88807d
			let proof_record = EthereumReceiptProof {
				index: 0x12,
				proof: array_bytes::hex2bytes_unchecked("0xf90654f90651b873f871a08905d6a9a81124e73b632ff8e0ac638331d4aa0f89bc5b296b5132ab1e6db295a0296366ce16b627f71457cefa27e7cbd6aa3f13ce1e2225bb06236089d5363667808080808080a0c73f3d756add498b44b70ae5d5b917fcc8c3adb72f10cc5cd245a862d9a2a17d8080808080808080b873f871a039af78839760433d410ab11ef453e8656451a51575e0be71fa7a72b54bfc296aa098d3ce8768b102d89494e55dde408e28d1ec148affca70a955d2b30bd9fcf008a078eacce43297ddfa328b51d92ccd8666abf1df855cef7ab3b1f4dbd16874ff658080808080808080808080808080b90564f9056120b9055df9055a01830e921db9010000000000008000000000002000000004400000000000001000000000000000000000000000000000000010000000000000000000000000400000000000000000000000000000000010000008000000000010000000000000000000000000000000000000020000000104000000000800080000000000000000000010000000400000000000000001000000000000008000000002004000000010000000200000000000000000000000000000800000000000000000000200080000000000000000000002000000000000000000040000000001000000000800000000000020000000000000000000000000000010000000000000000080000000000000000000f9044ff89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000001121d33597384000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a00000000000000000000000007f5b598827359939606b3525712fb124a1c7851da00000000000000000000000000000000000000000000000001bc16d674ec80000f87a94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000001121d33597384000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000001121d33597384000f8fc9449262b932e439271d05634c32978294c7ea15d0cf863a0c9dcda609937876978d7e0aa29857cb187aea06ad9e843fd23fd32108da73f10a0000000000000000000000000b52fbe2b925ab79a821b261c82c5ba0814aaa5e0a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5b8800000000000000000000000000000000000000000000000001121d3359738400000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318f8fc94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a09bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0cb8800000000000000000000000000000000000000000000000001121d3359738400000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
				header_hash: array_bytes::hex2array_unchecked!("0xabf627ce77d9f92a40f34e3cace721c3f089000dae820d00d3e99314c263a0c3", 32).into()
			};

			let header : EthereumHeader = serde_json::from_str(r#"{"parent_hash":"0xd55ce7660d0161c38b34015ce5468e1661f1c77865f23415e246ac9ccf7b2b22","timestamp":1599124448,"number":8610261,"author":"0xad87c0e80ab5e13f15757d5139cc6c6fcb823be3","transactions_root":"0x6cf40dbc3f8ce55ffc0f863a65ffd285da787b77af952c319e2577c5ab278a3a","uncles_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","extra_data":"0xde830207028f5061726974792d457468657265756d86312e34312e30826c69","state_root":"0xf108de6fa8fbf18e795cb3e1911d32e7d26178a69bb2f365662c8cba58bd7159","receipts_root":"0xfdb6d2cdb6bc9e78711a008b2912e62db28de4520dfcd49bd6f7086718c5d0cb","log_bloom":"0x00000000008000000000002000000004400000000000001000000000000000002000000000080000000010000040000000000001000000400000000000000000000000000000000011000008000000000210000000000000400000000000000000000000020000008104000000080800080000000000000000000030000000400004000000000001000000040000008000000002004000810010000000200000002000000000200000000000800000040010000400000200080000000000000000000402000000000200010000040000000001000001200800000000000020000000000000000000000000000010010000800000000080010000000004000000","gas_used":954909,"gas_limit":8000029,"difficulty":515540132,"seal":["0xa0deadf98810a6ccfb8d00e8f6bc7ad7f5d62d5a42760c0d9db8a549df76697704","0x88bf7326c26c57c69c"],"hash":"0xabf627ce77d9f92a40f34e3cace721c3f089000dae820d00d3e99314c263a0c3"}"#).unwrap();

			assert_ok!(EthereumRelay::init_genesis_header(&header, 31419688206738532));

			let expect_account_id = EthereumBacking::account_id_try_from_bytes(
				&array_bytes::hex2bytes_unchecked("0xe44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
			).unwrap();

			assert_eq!(
				EthereumBacking::parse_token_redeem_proof(&proof_record),
				Ok((expect_account_id, (true, 1234500000), 0)),
			);
		});
}

#[test]
fn verify_linear_redeem_ring() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(EthereumRelay::set_number_of_blocks_safe(RawOrigin::Root.into(), 0));

			// https://ropsten.etherscan.io/tx/0x1d3ef601b9fa4a7f1d6259c658d0a10c77940fa5db9e10ab55397eb0ce88807d
			let proof_record = EthereumReceiptProof {
				index: 0x12,
				proof: array_bytes::hex2bytes_unchecked("0xf90654f90651b873f871a08905d6a9a81124e73b632ff8e0ac638331d4aa0f89bc5b296b5132ab1e6db295a0296366ce16b627f71457cefa27e7cbd6aa3f13ce1e2225bb06236089d5363667808080808080a0c73f3d756add498b44b70ae5d5b917fcc8c3adb72f10cc5cd245a862d9a2a17d8080808080808080b873f871a039af78839760433d410ab11ef453e8656451a51575e0be71fa7a72b54bfc296aa098d3ce8768b102d89494e55dde408e28d1ec148affca70a955d2b30bd9fcf008a078eacce43297ddfa328b51d92ccd8666abf1df855cef7ab3b1f4dbd16874ff658080808080808080808080808080b90564f9056120b9055df9055a01830e921db9010000000000008000000000002000000004400000000000001000000000000000000000000000000000000010000000000000000000000000400000000000000000000000000000000010000008000000000010000000000000000000000000000000000000020000000104000000000800080000000000000000000010000000400000000000000001000000000000008000000002004000000010000000200000000000000000000000000000800000000000000000000200080000000000000000000002000000000000000000040000000001000000000800000000000020000000000000000000000000000010000000000000000080000000000000000000f9044ff89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000001121d33597384000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a00000000000000000000000007f5b598827359939606b3525712fb124a1c7851da00000000000000000000000000000000000000000000000001bc16d674ec80000f87a94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000001121d33597384000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000001121d33597384000f8fc9449262b932e439271d05634c32978294c7ea15d0cf863a0c9dcda609937876978d7e0aa29857cb187aea06ad9e843fd23fd32108da73f10a0000000000000000000000000b52fbe2b925ab79a821b261c82c5ba0814aaa5e0a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5b8800000000000000000000000000000000000000000000000001121d3359738400000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318f8fc94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a09bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0cb8800000000000000000000000000000000000000000000000001121d3359738400000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
				header_hash: array_bytes::hex2array_unchecked!("0xabf627ce77d9f92a40f34e3cace721c3f089000dae820d00d3e99314c263a0c3", 32).into()
			};

			let header : EthereumHeader = serde_json::from_str(r#"{"parent_hash":"0xd55ce7660d0161c38b34015ce5468e1661f1c77865f23415e246ac9ccf7b2b22","timestamp":1599124448,"number":8610261,"author":"0xad87c0e80ab5e13f15757d5139cc6c6fcb823be3","transactions_root":"0x6cf40dbc3f8ce55ffc0f863a65ffd285da787b77af952c319e2577c5ab278a3a","uncles_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","extra_data":"0xde830207028f5061726974792d457468657265756d86312e34312e30826c69","state_root":"0xf108de6fa8fbf18e795cb3e1911d32e7d26178a69bb2f365662c8cba58bd7159","receipts_root":"0xfdb6d2cdb6bc9e78711a008b2912e62db28de4520dfcd49bd6f7086718c5d0cb","log_bloom":"0x00000000008000000000002000000004400000000000001000000000000000002000000000080000000010000040000000000001000000400000000000000000000000000000000011000008000000000210000000000000400000000000000000000000020000008104000000080800080000000000000000000030000000400004000000000001000000040000008000000002004000810010000000200000002000000000200000000000800000040010000400000200080000000000000000000402000000000200010000040000000001000001200800000000000020000000000000000000000000000010010000800000000080010000000004000000","gas_used":954909,"gas_limit":8000029,"difficulty":515540132,"seal":["0xa0deadf98810a6ccfb8d00e8f6bc7ad7f5d62d5a42760c0d9db8a549df76697704","0x88bf7326c26c57c69c"],"hash":"0xabf627ce77d9f92a40f34e3cace721c3f089000dae820d00d3e99314c263a0c3"}"#).unwrap();

			assert_ok!(EthereumRelay::init_genesis_header(&header, 31419688206738532));

			let expect_account_id = EthereumBacking::account_id_try_from_bytes(
				&array_bytes::hex2bytes_unchecked("0xe44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
			).unwrap();
			let id1 = AccountId32::from([0; 32]);
			let ring_locked_before = EthereumBacking::pot::<Ring>();
			let _ = Ring::deposit_creating(&expect_account_id, 1);

			assert_ok!(EthereumBacking::redeem(
				Origin::signed(id1.clone()),
				RedeemFor::Token,
				proof_record.clone()
			));
			assert_eq!(Ring::free_balance(&expect_account_id), 1234500000 + 1);

			let ring_locked_after = EthereumBacking::pot::<Ring>();
			assert_eq!(ring_locked_after + 1234500000, ring_locked_before);

			// shouldn't redeem twice
			assert_err!(
				EthereumBacking::redeem(Origin::signed(id1.clone()), RedeemFor::Token, proof_record),
				<Error<Test>>::AssetAR,
			);
		});
}

#[test]
fn verify_linear_redeem_kton() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(EthereumRelay::set_number_of_blocks_safe(RawOrigin::Root.into(), 0));

			// https://ropsten.etherscan.io/tx/0x2878ae39a9e0db95e61164528bb1ec8684be194bdcc236848ff14d3fe5ba335d
			// darwinia: 5FP2eFNSVxJzSrE3N2NEVFPhUU34VzYFD6DDtRXbYzTdwPn8
			// hex: 0x92ae5b41feba5ee68a61449c557efa9e3b894a6461c058ec2de45429adb44546
			// amount: 0.123456789123456789 KTON
			let proof_record = EthereumReceiptProof {
				index: 0x4,
				proof: array_bytes::hex2bytes_unchecked("0xf90654f90651b853f851a0924e7317d57b9cb7ebf90321fdc9f800b94b64adbaae8da31dab0142e8c079ea80808080808080a0e58215be848c1293dd381210359d84485553000a82b67410406d183b42adbbdd8080808080808080b893f89180a0d1d9123dac06536f593ff89d28ac2373b3bc603fbee756e6054d6b2162e99337a01d2879f862c4f4f818f91e74fa43188c36a344c93132783006864e506a656076a0002cd3adf59d2aaa8313a0bbaa8fd411921077bfa1edcbe04018f7339bd273faa03bddfc128660298a289de46a9301b7f03cbe5364c22aedbc28f472bdbc318778808080808080808080808080b90564f9056120b9055df9055a0183032c33b901000000000000800000000000000000000440000000000000100000000000000000000000000000000000001000000000000000000000000040000000000000000000000000000040001020000800000000001000000000000000000004000000000000000002000000010400000000080008000000000000000000001000000040000000000000000000000000000000c000000002004000000010000000200000000000000000000000000000800000000000000000000202080000000000000000000002000000000000000000040000000001000000000000000000000020000000000000000000000000010010000000000000000080000000000000000000f9044ff89b941994100c58753793d52c6f457f189aa3ce9cee94f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000000000703b4d2a5000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a00000000000000000000000007f5b598827359939606b3525712fb124a1c7851da00000000000000000000000000000000000000000000000001bc16d674ec80000f87a941994100c58753793d52c6f457f189aa3ce9cee94f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000000000703b4d2a5000f89b941994100c58753793d52c6f457f189aa3ce9cee94f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa000000000000000000000000049262b932e439271d05634c32978294c7ea15d0ca00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000703b4d2a5000f8fc9449262b932e439271d05634c32978294c7ea15d0cf863a0c9dcda609937876978d7e0aa29857cb187aea06ad9e843fd23fd32108da73f10a00000000000000000000000001994100c58753793d52c6f457f189aa3ce9cee94a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5b8800000000000000000000000000000000000000000000000000000703b4d2a500000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318f8fc941994100c58753793d52c6f457f189aa3ce9cee94f863a09bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1a0000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5a000000000000000000000000049262b932e439271d05634c32978294c7ea15d0cb8800000000000000000000000000000000000000000000000000000703b4d2a500000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
				header_hash: array_bytes::hex2array_unchecked!("0x5f80ee45d62872fa4b0dbf779a8eb380e166ac80616d05875dd3e80e5fc40839", 32).into()
			};

			let header : EthereumHeader = serde_json::from_str(r#"{"parent_hash":"0x734ea7bd03f510e7dd0acc85a7ceb777d5d7d5ad5650785536fc09179a250143","timestamp":1599124483,"number":8610265,"author":"0x52351e33b3c693cc05f21831647ebdab8a68eb95","transactions_root":"0xd1c259805e50c5c4aa21b92f31d8a59f025f457a90a250ca516e8930d7bb05ec","uncles_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","extra_data":"0x6c6f746f706f6f6c","state_root":"0xabb0917ee898c71f3826d97415ff9161b37d03146b54e1c7d536fabb23352a5c","receipts_root":"0x9873421df4b9e11b174bc08e4d79d9a65032ba702ea8d3e0e6c6e9217d1cbf03","log_bloom":"0x0000000000800000000000000000000440000000000000100000000000000000000000000000000000001000000000000000000000000040000000000000000000000000000040001020000800000000001000000000000000000004000000000000000002000000010400000000080008000000000000000000001000000040000000000000000000000000000000c000000002004000000010000000200000000000000000000000000000800000000000000000000202080000000000000000000002000000000000000000040000000001000000000000000000000020000000000000000000000000010010000000000000000080000000000000000000","gas_used":207923,"gas_limit":8000000,"difficulty":516043711,"seal":["0xa0a7ea34046ebd043b4bca8836bc8968113d026d6c852b80a83a9b06db2db938e5","0x8800118000012d2ccd"],"hash":"0x5f80ee45d62872fa4b0dbf779a8eb380e166ac80616d05875dd3e80e5fc40839"}"#).unwrap();

			assert_ok!(EthereumRelay::init_genesis_header(&header, 31419690269906095));

			let expect_account_id = EthereumBacking::account_id_try_from_bytes(
				&array_bytes::hex2bytes_unchecked("0xe44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
			).unwrap();
			// 0.123456789123456789 KTON
			assert_eq!(
				EthereumBacking::parse_token_redeem_proof(&proof_record),
				Ok((expect_account_id.clone(), (false, 123400), 0)),
			);

			let id1 = AccountId32::from([0; 32]);
			let kton_locked_before = EthereumBacking::pot::<Kton>();
			let _ = Kton::deposit_creating(&expect_account_id, 1);

			assert_ok!(EthereumBacking::redeem(
				Origin::signed(id1.clone()),
				RedeemFor::Token,
				proof_record.clone()
			));
			assert_eq!(Kton::free_balance(&expect_account_id), 123400 + 1);

			let kton_locked_after = EthereumBacking::pot::<Kton>();
			assert_eq!(kton_locked_after + 123400, kton_locked_before);

			// shouldn't redeem twice
			assert_err!(
				EthereumBacking::redeem(Origin::signed(id1.clone()), RedeemFor::Token, proof_record),
				<Error<Test>>::AssetAR,
			);
		});
}

#[test]
fn verify_linear_redeem_deposit() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(EthereumRelay::set_number_of_blocks_safe(RawOrigin::Root.into(), 0));

			// 1234ring -> 0.1234kton

			// _depositID    2
			// 0: address: 0xcC5E48BEb33b83b8bD0D9d9A85A8F6a27C51F5C5  _depositor
			// 1: uint128: 1001000000000000000000 _value
			// 2: uint128: 12 _months
			// 3: uint256: 1599125470 _startAt
			// 4: uint256: 1000 _unitInterest
			// 5: bool: false
			//  _data     0x92ae5b41feba5ee68a61449c557efa9e3b894a6461c058ec2de45429adb44546

			// transfer：https://ropsten.etherscan.io/tx/0x5a7004126466ce763501c89bcbb98d14f3c328c4b310b1976a38be1183d91919
			let proof_record = EthereumReceiptProof {
				index: 0x2e,
				proof: array_bytes::hex2bytes_unchecked("0xf9065ff9065cb8b3f8b1a00465eebe6e5de09530e54a14732f416c6dd3df3e3d4de7e224057775e176005fa0364264753139e9b26d4caa5550e780af82744f1fa0212fb4d944843f40941767a0d5e53b63048540c8faf6b8cb035d471603e21ed03641b2750fae5b6029968928a0a2c1de87659c963a8a8970108e0087f59b102253748e0655f60005fc5f21463780808080a0985911552c5e2f0c8f1d3b43d8c68cb6cd0e23292e7a01ed65865d9c35b5364c8080808080808080b90214f90211a09a09b69760a6f7754adb10479eed2baa872b4994161458511eda43e19ada21d1a0778f7b69876965bfc8363c672c21912ca5f92f93980999e1da19b9849c075d45a0c120f0350037e907148309c3a2e5b07bfe220e630f67c1bd7d834b40e0c6e484a0138d6d6f0e5bcf7a46b8881303caeb3bbbd3cb32d654c0294172d44d3fad3ad2a0e0665be49fab75e8c174394d55813a6fc460d64e396ef4881a55a44b89e1fae3a00227d0ab9af8d74eddbf2f8ee9bb66f7ba856ae09571f9a24272f88854d9a771a00b40ca0d746f1610bcd9d1733cdbb4a03a495cc03e6b36586e78b7f752bf8908a02c5ae2bf60a79068b949a1a5cc831f468e8092615aa61b9c8e42f21449bad981a0d6fb9b1a137c24ab6aeadfd96385eeeeebf93061285e3d2e72c5227532d2f50ba0f8206f39008acc6ce0b550424a79d05daabe982f12aaaa499ebf6165c271721ba07e1caebbd285e75cceecfa73f76dc0d5c9b3a4471d37249a1f0c8f9f9f08404aa044e197285d11b10d4d1db722bc28eb33904014c70e1f0a48dc2557be28ee836da0d2255e960373f9d0a8af01d7a2eaa2ef61d4626be9a2ea25b92cd00b7eb2d370a0ac6e37eb951f28e057c20f2be5ec4929b03f523d6fb7728701dea74be9f7c7cba06929256f0c8f74646539b0407b77c905451e1b74bc1f968a73b4c51b4889c537a0fa4f77e7a759b28b652a748dff3f136fa4ed34458fd7e212af8614235a14b27380b9038df9038a20b90386f90383018371c62eb9010000000000000000400000200000000000000000000000001000000000000000000000000140000000000000000400000000000000000000000000000000000000000000000000008000000008000000000000000000000000000000000000000000000000020000000000000000000800080000000000000000000010000000001000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000800008000000208000000000000000000000002000000000000000000040000000001000000000000000000020020000000000000000000080000000000000000000000000000000000000000000000f90278f87a94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a00000000000000000000000006ef538314829efa8386fc43386cb13b4e0a67d1ea000000000000000000000000000000000000000000000003643aa647986040000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa00000000000000000000000006ef538314829efa8386fc43386cb13b4e0a67d1ea00000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000003643aa647986040000f9015c946ef538314829efa8386fc43386cb13b4e0a67d1ef842a0e77bf2fa8a25e63c1e5e29e1b2fcb6586d673931e020c4e3ffede453b830fb12a0000000000000000000000000000000000000000000000000000000000000001eb90100000000000000000000000000cc5e48beb33b83b8bd0d9d9a85a8f6a27c51f5c5000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000005f50b7de00000000000000000000000000000000000000000000000000000000000003e800000000000000000000000000000000000000000000003643aa64798604000000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000020e44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
				header_hash: array_bytes::hex2array_unchecked!("0x202591d2a7bff469ec186e3583e37b9c4bce2db847612ff975180436b5a4a1f1", 32).into()
			};

			let header : EthereumHeader = serde_json::from_str(r#"{"parent_hash":"0x870d2655fe393a3d12f595bce56ce5115907af9735cc8de8d95c05e7467d1321","timestamp":1599126312,"number":8610453,"author":"0x52351e33b3c693cc05f21831647ebdab8a68eb95","transactions_root":"0xd5a01c26eeb8e826613e4da09ac06f305fab7d183a84d417559139db814e2a23","uncles_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","extra_data":"0x6c6f746f706f6f6c","state_root":"0x6d63278b3c9dae252148948ddd0db13ee1f7c81dc557681db22e422d37d9abf1","receipts_root":"0x707af65a83888ce8d9696941a122d3429c0600fb09c4644ab2162b1382099928","log_bloom":"0x00000048240080400004204082080d00000800004000001001810000028001408040000140002800000020000404010014020000100300000000000000200000240000000120808100004008400000020083826000000000800020008000100000000024021842400000042048000820080004000008400000000010001880581090880302008080004001208220000000000363084002480080200200201800220080001800004000040009002000100840008a00408208000004c00881000f00000102000500004000100000070a408100c9844020001000100000030020000010020000021520080810020000000082002020000000400000002080000000","gas_used":7545472,"gas_limit":8000000,"difficulty":535275643,"seal":["0xa01548d3ecae87cb01cb142984086dc9f772b9d9094884269b1e3644ec108e52b5","0x880011800002104414"],"hash":"0x202591d2a7bff469ec186e3583e37b9c4bce2db847612ff975180436b5a4a1f1"}"#).unwrap();

			assert_ok!(EthereumRelay::init_genesis_header(&header, 31419789208662997));

			let ring_locked_before = EthereumBacking::pot::<Ring>();
			let expect_account_id = EthereumBacking::account_id_try_from_bytes(
				&array_bytes::hex2bytes_unchecked("0xe44664996ab7b5d86c12e9d5ac3093f5b2efc9172cb7ce298cd6c3c51002c318"),
			).unwrap();
			let id1 = AccountId32::from([0; 32]);
			let controller = AccountId32::from([1; 32]);
			let _ = Ring::deposit_creating(&expect_account_id, 1);

			assert_ok!(Call::from(<darwinia_staking::Call<Test>>::bond(
				controller.clone(),
				StakingBalance::RingBalance(1),
				RewardDestination::Controller,
				0,
			)).dispatch(Origin::signed(expect_account_id.clone())));
			assert_ok!(EthereumBacking::redeem(
				Origin::signed(id1.clone()),
				RedeemFor::Deposit,
				proof_record.clone()
			));
			assert_eq!(Ring::free_balance(&expect_account_id), 1001000000000 + 1);

			let ring_locked_after = EthereumBacking::pot::<Ring>();
			assert_eq!(ring_locked_after + 1001000000000, ring_locked_before);

			let staking_ledger = Staking::ledger(&controller);
			assert_eq!(staking_ledger, Some(StakingLedger {
				stash: expect_account_id,
				active_ring: 1001000000001,
				active_deposit_ring: 1001000000000,
				deposit_items: vec![TimeDepositItem {
					value: 1001000000000,
					start_time: 1599125470000,
					expire_time: 1630229470000,
				}],
				ring_staking_lock: StakingLock { staking_amount: 1001000000001, unbondings: vec![] },
				..Default::default()
			}));

			// shouldn't redeem twice
			assert_err!(
				EthereumBacking::redeem(Origin::signed(id1.clone()), RedeemFor::Deposit, proof_record),
				<Error<Test>>::AssetAR,
			);
		});
}
