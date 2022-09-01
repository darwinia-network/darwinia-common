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

// --- std ---
use std::str::FromStr;
// --- crates.io ---
use array_bytes::hex2bytes_unchecked;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use fp_evm::{Context, FeeCalculator, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	traits::{Everything, GenesisBuild},
	weights::GetDispatchInfo,
	PalletId,
};
use frame_system::mocking::*;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{
	*, {self as s2s_issuing},
};
use darwinia_ethereum::{
	adapter::{CurrencyAdapter, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot, RawOrigin,
};
use darwinia_evm::{EVMCurrencyAdapter, EnsureAddressTruncated, SubstrateBlockHashMapping};
use darwinia_evm_precompile_bridge_s2s::Sub2SubBridge;
use darwinia_evm_precompile_utils::test_helper::{address_build, AccountInfo};
use darwinia_support::{
	evm::DeriveSubstrateAddress,
	s2s::{LatestMessageNoncer, RelayMessageSender},
};

type Block = MockBlock<Test>;
type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u64;

pub const MAPPING_TOKEN_FACTORY_CONTRACT_BYTECODE: &str =
	include_str!("./res/mapping_token_factory_bytecode.txt");
pub const MAPPING_TOKEN_LOGIC_CONTRACT_BYTECODE: &str =
	include_str!("./res/mapping_erc20_bytecode.txt");

darwinia_support::impl_test_account_data! {}

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

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 0;
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
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
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

frame_support::parameter_types! {
	pub const DvmPalletId: PalletId = PalletId(*b"dar/dvmp");
}
impl darwinia_ethereum::Config for Test {
	type Event = ();
	type PalletId = DvmPalletId;
	type StateRoot = IntermediateStateRoot;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}
pub struct HashedConverter;
impl DeriveSubstrateAddress<AccountId32> for HashedConverter {
	fn derive_substrate_address(address: &H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);
		AccountId32::from(Into::<[u8; 32]>::into(data))
	}
}
pub struct MockPrecompiles<R>(PhantomData<R>);
impl<R> MockPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn used_addresses() -> [H160; 6] {
		[addr(24)]
	}
}
impl<R> PrecompileSet for MockPrecompiles<R>
where
	Sub2SubBridge<R, MockS2sMessageSender, ()>: Precompile,
	R: darwinia_evm::Config,
{
	fn execute(
		&self,
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> Option<PrecompileResult> {
		match address {
			// Darwinia precompiles
			a if a == addr(24) => Some(<Sub2SubBridge<R, MockS2sMessageSender, ()>>::execute(
				input, target_gas, context, is_static,
			)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}
fn addr(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}
frame_support::parameter_types! {
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
	pub PrecompilesValue: MockPrecompiles<Test> = MockPrecompiles::<_>::new();
}
impl darwinia_evm::Config for Test {
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type ChainId = ChainId;
	type Event = ();
	type FeeCalculator = FixedGasPrice;
	type FindAuthor = ();
	type GasWeightMapping = ();
	type IntoAccountId = HashedConverter;
	type KtonBalanceAdapter = CurrencyAdapter<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type RingBalanceAdapter = CurrencyAdapter<Self, Ring, RingRemainBalance>;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const PangoroChainId: bp_runtime::ChainId = *b"pagr";
	pub RootAccountForPayments: Option<AccountId32> = Some([1;32].into());
	pub PangoroName: Vec<u8> = (b"Pangoro").to_vec();
	pub MessageLaneId: [u8; 4] = *b"ltor";
}

pub struct AccountIdConverter;
impl Convert<H256, AccountId32> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}
pub struct MockS2sMessageSender;
impl RelayMessageSender for MockS2sMessageSender {
	fn encode_send_message(
		_message_pallet_index: u32,
		_lane_id: [u8; 4],
		_payload: Vec<u8>,
		_fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		// don't send message instead system remark
		Ok(hex2bytes_unchecked("0x0001081234"))
	}
}
impl LatestMessageNoncer for MockS2sMessageSender {
	fn outbound_latest_generated_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}

	fn inbound_latest_received_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}
}

pub struct MockOutboundMessenger;
impl OutboundMessenger<AccountId32> for MockOutboundMessenger {
	fn check_lane_id(lane_id: &LaneId) -> bool {
		return *lane_id == MessageLaneId::get();
	}

	fn get_valid_message_sender(_nonce: MessageNonce) -> Result<AccountId32, &'static str> {
		let derived_substrate_account =
			darwinia_support::evm::ConcatConverter::<AccountId32>::derive_substrate_address(
				&H160::from_str("32dcab0ef3fb2de2fce1d2e0799d36239671f04a").unwrap(),
			);

		return Ok(derived_substrate_account);
	}
}

impl Config for Test {
	type BackingChainName = PangoroName;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type Event = ();
	type InternalTransactHandler = Ethereum;
	type OutboundMessenger = MockOutboundMessenger;
	type PalletId = S2sRelayPalletId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		S2sIssuing: s2s_issuing::{Pallet, Call, Storage, Config, Event<T>},
		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: darwinia_ethereum::{Pallet, Call, Storage, Config, Origin},
	}
}

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			Call::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(&self, info: &Self::SignedInfo) -> Option<TransactionValidity> {
		match self {
			Call::Ethereum(ref call) => Some(validate_self_contained_inner(&self, &call, info)),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.pre_dispatch_self_contained(info),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<sp_runtime::traits::PostDispatchInfoOf<Self>>> {
		use sp_runtime::traits::Dispatchable as _;
		match self {
			call @ Call::Ethereum(darwinia_ethereum::Call::transact { .. }) =>
				Some(call.dispatch(Origin::from(RawOrigin::EthereumTransaction(info)))),
			_ => None,
		}
	}
}

fn validate_self_contained_inner(
	call: &Call,
	eth_call: &darwinia_ethereum::Call<Test>,
	signed_info: &<Call as fp_self_contained::SelfContainedCall>::SignedInfo,
) -> TransactionValidity {
	if let darwinia_ethereum::Call::transact { ref transaction } = eth_call {
		// Previously, ethereum transactions were contained in an unsigned
		// extrinsic, we now use a new form of dedicated extrinsic defined by
		// frontier, but to keep the same behavior as before, we must perform
		// the controls that were performed on the unsigned extrinsic.
		use sp_runtime::traits::SignedExtension as _;
		let input_len = match transaction {
			darwinia_ethereum::Transaction::Legacy(t) => t.input.len(),
			darwinia_ethereum::Transaction::EIP2930(t) => t.input.len(),
			darwinia_ethereum::Transaction::EIP1559(t) => t.input.len(),
		};
		let extra_validation =
			SignedExtra::validate_unsigned(call, &call.get_dispatch_info(), input_len)?;
		// Then, do the controls defined by the ethereum pallet.
		let self_contained_validation = eth_call
			.validate_self_contained(signed_info)
			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::BadProof))??;

		Ok(extra_validation.combine_with(self_contained_validation))
	} else {
		Err(TransactionValidityError::Unknown(
			sp_runtime::transaction_validity::UnknownTransaction::CannotLookup,
		))
	}
}

pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mapping_factory_address =
		H160::from_str("0000000000000000000000000000000000000002").unwrap();

	<s2s_issuing::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
		&s2s_issuing::GenesisConfig { mapping_factory_address },
		&mut t,
	)
	.unwrap();

	let pairs = (0..accounts_len).map(|i| address_build(i as u8)).collect::<Vec<_>>();

	let balances: Vec<_> =
		(0..accounts_len).map(|i| (pairs[i].account_id.clone(), 100_000_000_000)).collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	(pairs, ext.into())
}
