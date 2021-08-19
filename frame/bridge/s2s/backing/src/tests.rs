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

// crates
use codec::{Decode, Encode};
use std::str::FromStr;
// darwinia
use crate::*;
use crate::{self as s2s_backing};
use darwinia_support::s2s::RelayMessageCaller;
// substrate
use frame_support::{assert_ok, traits::GenesisBuild, weights::PostDispatchInfo, PalletId};
use frame_system::mocking::*;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, DispatchErrorWithPostInfo, RuntimeDebug,
};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Balance = u64;
pub type AccountId<T> = <T as frame_system::Config>::AccountId;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
	type Balance = Balance;
	type Event = ();
	type BalanceInfo = AccountData<Balance>;
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
	type Balance = Balance;
	type Event = ();
	type BalanceInfo = AccountData<Balance>;
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 6000 / 2;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

frame_support::parameter_types! {
	pub const MockChainId: [u8; 4] = [0; 4];
	pub const MockId: PalletId = PalletId(*b"da/s2sba");
	pub const RingLockLimit: Balance = 1_000_000_000;
}

pub struct MockRelayCaller;
impl RelayMessageCaller<(), Balance> for MockRelayCaller {
	fn send_message(
		_payload: (),
		_fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		Ok(().into())
	}
}

pub struct MockCallEncoder;
impl EncodeCall<AccountId<Test>, ()> for MockCallEncoder {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(_spec_version: u32, _weight: u64, _token: Token) -> () {
		()
	}
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		_spec_version: u32,
		_weight: u64,
		_token: Token,
		_recipient: RecipientAccount<AccountId<Test>>,
	) -> Result<(), ()> {
		Ok(())
	}
}

impl Config for Test {
	type Event = ();
	type WeightInfo = ();
	type PalletId = MockId;
	type RingLockMaxLimit = RingLockLimit;
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = ();
	type BridgedChainId = MockChainId;
	type OutboundPayload = ();
	type CallEncoder = MockCallEncoder;
	type FeeAccount = ();
	type MessageSender = MockRelayCaller;
}

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Backing: s2s_backing::{Pallet, Call, Storage, Event<T>},
	}
}
