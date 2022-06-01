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

// --- std ---
use std::str::FromStr;
// --- crates.io ---
use array_bytes::hex2bytes_unchecked;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use bp_messages::source_chain::SendMessageArtifacts;
use bp_runtime::{derive_account_id, SourceAccount};
use frame_support::{
	assert_err, assert_ok, dispatch::PostDispatchInfo, traits::Everything, PalletId,
};
use frame_system::{mocking::*, RawOrigin};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, DispatchErrorWithPostInfo, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{self as s2s_backing, *};
use darwinia_support::{
	evm::{ConcatConverter, DeriveEthereumAddress, DeriveSubstrateAddress},
	s2s::RelayMessageSender,
};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Balance = u64;
type AccountId<T> = <T as frame_system::Config>::AccountId;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 6000 / 2;
}
impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId32;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = ();
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

pub struct MockRelayCaller;
impl RelayMessageSender for MockRelayCaller {
	fn encode_send_message(
		_message_pallet_index: u32,
		_lane_id: [u8; 4],
		_payload: Vec<u8>,
		_fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		Ok(Vec::new())
	}
}

pub struct MockLatestMessageNoncer;
impl LatestMessageNoncer for MockLatestMessageNoncer {
	fn outbound_latest_generated_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}

	fn inbound_latest_received_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}
}

pub struct MockMessagesBridge;
impl MessagesBridge<AccountId<Test>, Balance, ()> for MockMessagesBridge {
	type Error = DispatchErrorWithPostInfo<PostDispatchInfo>;

	fn send_message(
		submitter: RawOrigin<AccountId<Test>>,
		_laneid: [u8; 4],
		_payload: (),
		fee: Balance,
	) -> Result<SendMessageArtifacts, Self::Error> {
		// send fee to fund account [2;32]
		Ring::transfer(submitter.into(), build_account(2), fee)?;
		Ok(SendMessageArtifacts { nonce: 0, weight: 0 })
	}
}

pub struct MockAccountIdConverter;
impl Convert<H256, AccountId32> for MockAccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}

frame_support::parameter_types! {
	pub const MockChainId: [u8; 4] = [0; 4];
	pub const MockId: PalletId = PalletId(*b"da/s2sba");
	pub RingMetadata: TokenMetadata = TokenMetadata::new(
		0,
		PalletId(*b"da/bring").derive_ethereum_address(),
		b"Pangoro Network Native Token".to_vec(),
		b"ORING".to_vec(),
		9);
	pub const MaxLockRingAmountPerTx: Balance = 100;
	pub const BridgePangolinLaneId: [u8; 4] = [0; 4];
}
impl Config for Test {
	type BridgedAccountIdConverter = MockAccountIdConverter;
	type BridgedChainId = MockChainId;
	type Event = ();
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type MessageLaneId = BridgePangolinLaneId;
	type MessageNoncer = MockLatestMessageNoncer;
	type MessagesBridge = MockMessagesBridge;
	type OutboundPayloadCreator = ();
	type PalletId = MockId;
	type RingCurrency = Ring;
	type RingMetadata = RingMetadata;
	type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Backing: s2s_backing::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
}

pub fn build_account(x: u8) -> AccountId32 {
	AccountId32::decode(&mut &[x; 32][..]).unwrap_or_default()
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	s2s_backing::GenesisConfig::<Test> {
		secure_limited_period: 10,
		secure_limited_ring_amount: 1_000_000,
		remote_mapping_token_factory_account: Default::default(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	// add some balance to backing account 10 ring
	let balances = vec![(Backing::pallet_account_id(), 10_000_000_000), (build_account(1), 100)];
	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut storage)
		.unwrap();

	storage.into()
}

#[test]
fn test_back_erc20_dvm_address() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			<Test as s2s_backing::Config>::RingMetadata::get().address,
			H160::from_str("0x6d6f646c64612f6272696e670000000000000000").unwrap()
		);
	});
}

#[test]
fn test_pallet_id_to_dvm_address() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			<Test as s2s_backing::Config>::PalletId::get().derive_ethereum_address(),
			H160::from_str("0x6d6f646c64612f73327362610000000000000000").unwrap()
		);
	});
}

#[test]
fn test_backing_account_id() {
	new_test_ext().execute_with(|| {
		let expected = hex2bytes_unchecked(
			// 5EYCAe5gKAhKXbKVquxUAg1Z22qvbkp8Ddmrmp5pCbKRHcs8
			"0x6d6f646c64612f73327362610000000000000000000000000000000000000000",
		);
		let expected_address = AccountId32::decode(&mut &expected[..]).unwrap_or_default();
		assert_eq!(Backing::pallet_account_id(), expected_address);
	});
}

#[test]
fn test_unlock_from_remote() {
	new_test_ext().execute_with(|| {
		// the mapping token factory contract address
		let mapping_token_factory =
			H160::from_str("0x61dc46385a09e7ed7688abe6f66bf3d8653618fd").unwrap();
		// convert dvm address to substrate address
		let remote_mapping_token_factory_account =
			ConcatConverter::<AccountId32>::derive_substrate_address(mapping_token_factory);
		// convert remote address to local derived address
		let hash = derive_account_id::<AccountId32>(
			<Test as s2s_backing::Config>::BridgedChainId::get(),
			SourceAccount::Account(remote_mapping_token_factory_account.clone()),
		);
		let derived_mapping_token_factory_address =
			<Test as s2s_backing::Config>::BridgedAccountIdConverter::convert(hash);

		// ring dvm address (original address)
		let ring_dvm_address = <Test as s2s_backing::Config>::RingMetadata::get().address;

		// Alice as recipient
		let recipient_alice = hex2bytes_unchecked(
			"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
		);
		let alice_account = AccountId32::decode(&mut &recipient_alice[..]).unwrap_or_default();

		assert_ok!(Backing::set_remote_mapping_token_factory_account(
			RawOrigin::Root.into(),
			remote_mapping_token_factory_account
		));

		assert_eq!(Ring::free_balance(alice_account.clone()), 0);
		assert_ok!(Backing::unlock_from_remote(
			Origin::signed(derived_mapping_token_factory_address.clone()),
			ring_dvm_address,
			U256::from(1_000_000),
			recipient_alice.clone()
		));
		assert_err!(
			Backing::unlock_from_remote(
				Origin::signed(derived_mapping_token_factory_address.clone()),
				ring_dvm_address,
				U256::from(1),
				recipient_alice
			),
			<Error<Test>>::RingDailyLimited
		);
		assert_eq!(Ring::free_balance(alice_account), 1_000_000);
	});
}

#[test]
fn test_lock_and_remote_issue() {
	new_test_ext().execute_with(|| {
		assert_ok!(Backing::lock_and_remote_issue(
			Origin::signed(build_account(1)),
			26100,
			40544000,
			60,
			10,
			H160::from_str("0x0000000000000000000000000000000000000001").unwrap()
		));
		assert_eq!(Ring::free_balance(build_account(1)), 30);
		assert_eq!(Ring::free_balance(build_account(2)), 10);
		assert_eq!(Ring::free_balance(Backing::pallet_account_id()), 10_000_000_060);

		assert_err!(
			Backing::lock_and_remote_issue(
				Origin::signed(build_account(1)),
				26100,
				40544000,
				<Test as s2s_backing::Config>::MaxLockRingAmountPerTx::get(),
				10,
				H160::from_str("0x0000000000000000000000000000000000000001").unwrap()
			),
			<Error<Test>>::RingLockLimited
		);
		assert_eq!(Ring::free_balance(build_account(1)), 30);
		assert_err!(
			Backing::lock_and_remote_issue(
				Origin::signed(build_account(0)),
				26100,
				40544000,
				1,
				1,
				H160::from_str("0x0000000000000000000000000000000000000001").unwrap()
			),
			<Error<Test>>::InsufficientBalance
		);
	});
}

#[test]
fn test_register_and_remote_create() {
	new_test_ext().execute_with(|| {
		assert_ok!(Backing::register_and_remote_create(
			Origin::signed(build_account(1)),
			26100,
			40544000,
			10,
		));
		assert_eq!(Ring::free_balance(build_account(1)), 90);
		assert_eq!(Ring::free_balance(build_account(2)), 10);
		assert_eq!(Ring::free_balance(Backing::pallet_account_id()), 10_000_000_000);
	});
}
