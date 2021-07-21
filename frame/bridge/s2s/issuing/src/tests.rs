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

// crates
use codec::{Decode, Encode};
use std::str::FromStr;
// darwinia
use crate::*;
use crate::{self as s2s_issuing};
use darwinia_evm::{
	AddressMapping, EnsureAddressTruncated, FeeCalculator, SubstrateBlockHashMapping,
};
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot,
};
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
	pub InternalTransactionGasLimit: U256 = U256::from(300_000_000);
}

impl dvm_ethereum::Config for Test {
	type Event = ();
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type InternalTransactionGasLimit = InternalTransactionGasLimit;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}

pub struct HashedAddressMapping;
impl AddressMapping<AccountId32> for HashedAddressMapping {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);
		AccountId32::from(Into::<[u8; 32]>::into(data))
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
}
impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type AddressMapping = HashedAddressMapping;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = ();
	type Precompiles = ();
	type FindAuthor = ();
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type IssuingHandler = ();
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const MillauChainId: bp_runtime::ChainId = *b"mcid";
	pub RootAccountForPayments: Option<AccountId32> = Some([1;32].into());
	pub RawCallGasLimit: U256 = U256::from(300_000_000);
}

pub struct AccountIdConverter;
impl Convert<H256, AccountId32> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
pub struct MockMessagePayload {
	spec_version: u32,
	weight: u64,
	call: Vec<u8>,
}

impl Size for MockMessagePayload {
	fn size_hint(&self) -> u32 {
		self.call.len() as _
	}
}

pub struct MillauCallEncoder;
impl EncodeCall<AccountId32, MockMessagePayload> for MillauCallEncoder {
	fn encode_remote_unlock(
		spec_version: u32,
		weight: u64,
		_token: Token,
		_recipient: RecipientAccount<AccountId32>,
	) -> Result<MockMessagePayload, ()> {
		return Ok(MockMessagePayload {
			spec_version,
			weight,
			call: vec![],
		});
	}
}

pub struct ToMillauMessageRelayCaller;
impl RelayMessageCaller<MockMessagePayload, Balance> for ToMillauMessageRelayCaller {
	fn send_message(
		_payload: MockMessagePayload,
		_fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		Ok(PostDispatchInfo {
			actual_weight: None,
			pays_fee: Pays::No,
		})
	}
}

pub struct TruncateToEthAddress;
impl ToEthAddress<AccountId32> for TruncateToEthAddress {
	fn into_ethereum_id(address: &AccountId32) -> H160 {
		let account20: &[u8] = &address.as_ref();
		H160::from_slice(&account20[..20])
	}
}

impl Config for Test {
	type Event = ();
	type PalletId = S2sRelayPalletId;
	type WeightInfo = ();
	type ReceiverAccountId = AccountId32;
	type RawCallGasLimit = RawCallGasLimit;

	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = MillauChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayload = MockMessagePayload;
	type CallEncoder = MillauCallEncoder;
	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToMillauMessageRelayCaller;
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
		S2sIssuing: s2s_issuing::{Pallet, Call, Storage, Config, Event<T>},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	let mapping_factory_address =
		H160::from_str("0000000000000000000000000000000000000002").unwrap();

	<s2s_issuing::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
		&s2s_issuing::GenesisConfig {
			mapping_factory_address,
		},
		&mut t,
	)
	.unwrap();
	t.into()
}

#[test]
fn burn_and_remote_unlock_success() {
	new_test_ext().execute_with(|| {
		let burn_info = TokenBurnInfo {
			spec_version: 0,
			weight: 100,
			token_type: 1,
			backing: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			sender: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			source: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			recipient: [1; 32].to_vec(),
			amount: U256::from(1),
			fee: U256::from(1),
		};
		assert_ok!(S2sIssuing::burn_and_remote_unlock(0, burn_info,));
	});
}

#[test]
fn check_digest() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			S2sIssuing::digest(),
			array_bytes::hex2bytes_unchecked("0xd184c5bd").as_slice()
		);
	});
}
