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
use ethereum::{TransactionAction, TransactionSignature};
use rlp::RlpStream;
use scale_info::TypeInfo;
use sha3::{Digest, Keccak256};
// --- paritytech ---
use fp_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	traits::{Everything, GenesisBuild},
	weights::GetDispatchInfo,
	PalletId,
};
use frame_system::mocking::*;
use pallet_evm::FeeCalculator;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
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
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot, RawOrigin, Transaction,
};
use darwinia_evm::{EVMCurrencyAdapter, EnsureAddressTruncated, SubstrateBlockHashMapping};
use darwinia_evm_precompile_bridge_s2s::Sub2SubBridge;
use darwinia_evm_precompile_dispatch::Dispatch;
use darwinia_support::{
	evm::IntoAccountId,
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

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type OtherCurrencies = ();
	type Balance = Balance;
	type Event = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type BalanceInfo = AccountData<Balance>;
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type OtherCurrencies = ();
	type Balance = Balance;
	type Event = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type BalanceInfo = AccountData<Balance>;
	type WeightInfo = ();
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
	type BaseCallFilter = Everything;
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
	pub const DvmPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl darwinia_ethereum::Config for Test {
	type Event = ();
	type PalletId = DvmPalletId;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}

pub struct HashedConverter;
impl IntoAccountId<AccountId32> for HashedConverter {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);
		AccountId32::from(Into::<[u8; 32]>::into(data))
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
	pub PrecompilesValue: MockPrecompiles<Test> = MockPrecompiles::<_>::new();
}
impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = HashedConverter;
	type Event = ();
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type FindAuthor = ();
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const PangoroChainId: bp_runtime::ChainId = *b"pagr";
	pub RootAccountForPayments: Option<AccountId32> = Some([1;32].into());
	pub PangoroName: Vec<u8> = (b"Pangoro").to_vec();
	pub MessageLaneId: [u8; 4] = *b"ltor";
}

pub struct MockPrecompiles<R>(PhantomData<R>);
impl<R> MockPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1, 2, 3, 4, 24, 25]
			.into_iter()
			.map(|x| H160::from_low_u64_be(x))
			.collect()
	}
}

impl<R> PrecompileSet for MockPrecompiles<R>
where
	Sub2SubBridge<R, MockS2sMessageSender, ()>: Precompile,
	Dispatch<R>: Precompile,
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
		let to_address = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == to_address(1) => {
				Some(ECRecover::execute(input, target_gas, context, is_static))
			}
			_ if address == to_address(2) => {
				Some(Sha256::execute(input, target_gas, context, is_static))
			}
			_ if address == to_address(3) => {
				Some(Ripemd160::execute(input, target_gas, context, is_static))
			}
			_ if address == to_address(4) => {
				Some(Identity::execute(input, target_gas, context, is_static))
			}
			// Darwinia precompiles
			_ if address == to_address(24) => {
				Some(<Sub2SubBridge<R, MockS2sMessageSender, ()>>::execute(
					input, target_gas, context, is_static,
				))
			}
			_ if address == to_address(25) => Some(<Dispatch<R>>::execute(
				input, target_gas, context, is_static,
			)),
			_ => None,
		}
	}
	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
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

	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type InternalTransactHandler = Ethereum;
	type BackingChainName = PangoroName;
	type MessageLaneId = MessageLaneId;
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
			call @ Call::Ethereum(darwinia_ethereum::Call::transact { .. }) => {
				Some(call.dispatch(Origin::from(RawOrigin::EthereumTransaction(info))))
			}
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
		let self_contained_validation = eth_call.validate_self_contained(signed_info).ok_or(
			TransactionValidityError::Invalid(InvalidTransaction::BadProof),
		)??;

		Ok(extra_validation.combine_with(self_contained_validation))
	} else {
		Err(TransactionValidityError::Unknown(
			sp_runtime::transaction_validity::UnknownTransaction::CannotLookup,
		))
	}
}

pub struct AccountInfo {
	pub address: H160,
	pub account_id: AccountId32,
	pub private_key: H256,
}

pub struct LegacyUnsignedTransaction {
	pub nonce: U256,
	pub gas_price: U256,
	pub gas_limit: U256,
	pub action: TransactionAction,
	pub value: U256,
	pub input: Vec<u8>,
}
impl LegacyUnsignedTransaction {
	fn signing_rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(9);
		s.append(&self.nonce);
		s.append(&self.gas_price);
		s.append(&self.gas_limit);
		s.append(&self.action);
		s.append(&self.value);
		s.append(&self.input);
		s.append(&ChainId::get());
		s.append(&0u8);
		s.append(&0u8);
	}

	fn signing_hash(&self) -> H256 {
		let mut stream = RlpStream::new();
		self.signing_rlp_append(&mut stream);
		H256::from_slice(&Keccak256::digest(&stream.out()).as_slice())
	}

	pub fn sign(&self, key: &H256) -> Transaction {
		let hash = self.signing_hash();
		let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
		let s = libsecp256k1::sign(
			&msg,
			&libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap(),
		);
		let sig = s.0.serialize();

		let sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + ChainId::get() * 2 + 35,
			H256::from_slice(&sig[0..32]),
			H256::from_slice(&sig[32..64]),
		)
		.unwrap();

		Transaction::Legacy(ethereum::LegacyTransaction {
			nonce: self.nonce,
			gas_price: self.gas_price,
			gas_limit: self.gas_limit,
			action: self.action,
			value: self.value,
			input: self.input.clone(),
			signature: sig,
		})
	}
}

fn address_build(seed: u8) -> AccountInfo {
	let raw_private_key = [seed + 1; 32];
	let secret_key = libsecp256k1::SecretKey::parse_slice(&raw_private_key).unwrap();
	let raw_public_key = &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
	let raw_address = {
		let mut s = [0; 20];
		s.copy_from_slice(&Keccak256::digest(raw_public_key)[12..]);
		s
	};
	let raw_account = {
		let mut s = [0; 32];
		s[..20].copy_from_slice(&raw_address);
		s
	};

	AccountInfo {
		private_key: raw_private_key.into(),
		account_id: raw_account.into(),
		address: raw_address.into(),
	}
}

pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
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

	let pairs = (0..accounts_len)
		.map(|i| address_build(i as u8))
		.collect::<Vec<_>>();

	let balances: Vec<_> = (0..accounts_len)
		.map(|i| (pairs[i].account_id.clone(), 100_000_000_000))
		.collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	(pairs, ext.into())
	//t.into()
}
