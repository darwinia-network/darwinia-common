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

//! Test utilities

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use ethereum::{TransactionAction, TransactionSignature};
use rlp::RlpStream;
use scale_info::TypeInfo;
use sha3::{Digest, Keccak256};
// --- paritytech ---
use darwinia_ethereum::{EthereumBlockHashMapping, RawOrigin};
use fp_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	pallet_prelude::Weight,
	traits::{Everything, FindAuthor, GenesisBuild},
	ConsensusEngineId, PalletId,
};
use frame_system::mocking::*;
use pallet_evm::FeeCalculator;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	transaction_validity::{TransactionValidity, TransactionValidityError},
	AccountId32, Perbill, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use crate::Transfer;
use darwinia_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot, Transaction,
};
use darwinia_evm::{runner::stack::Runner, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_support::evm::IntoAccountId;

type Block = MockBlock<Test>;
pub type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u64;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
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
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 10;
	pub const ExistentialDeposit: u64 = 500;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type Balance = Balance;
	type Event = Event;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = MaxLocks;
	type BalanceInfo = AccountData<Balance>;
	type OtherCurrencies = ();
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type Balance = Balance;
	type Event = Event;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = MaxLocks;
	type BalanceInfo = AccountData<Balance>;
	type OtherCurrencies = ();
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

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}
pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(address_build(0).address)
	}
}
pub struct HashedConverter;
impl IntoAccountId<AccountId32> for HashedConverter {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut raw_account = [0u8; 32];
		raw_account[0..20].copy_from_slice(&address[..]);
		raw_account.into()
	}
}

frame_support::parameter_types! {
	pub const TransactionByteFee: u64 = 1;
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
	pub PrecompilesValue: MockPrecompiles<Test> = MockPrecompiles::<_>::new();
}

pub struct MockPrecompiles<R>(PhantomData<R>);
impl<R> MockPrecompiles<R>
where
	R: darwinia_ethereum::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1, 2, 3, 4, 21]
			.into_iter()
			.map(|x| H160::from_low_u64_be(x))
			.collect()
	}
}

impl<R> PrecompileSet for MockPrecompiles<R>
where
	Transfer<R>: Precompile,
	R: darwinia_ethereum::Config,
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
			_ if address == to_address(21) => Some(<Transfer<R>>::execute(
				input, target_gas, context, is_static,
			)),
			_ => None,
		}
	}
	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}

impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = HashedConverter;
	type Event = Event;
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type FindAuthor = FindAuthorTruncated;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Runner = Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
}

frame_support::parameter_types! {
	pub const MockPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl darwinia_ethereum::Config for Test {
	type PalletId = MockPalletId;
	type Event = Event;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
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
		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: darwinia_ethereum::{Pallet, Call, Storage, Config, Event<T>, Origin},
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
			Call::Ethereum(call) => call.validate_self_contained(info),
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
		self.sign_with_chain_id(key, ChainId::get())
	}

	pub fn sign_with_chain_id(&self, key: &H256, chain_id: u64) -> Transaction {
		let hash = self.signing_hash();
		let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
		let s = libsecp256k1::sign(
			&msg,
			&libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap(),
		);
		let sig = s.0.serialize();

		let sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + chain_id * 2 + 35,
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

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
	// sc_cli::init_logger("");
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
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
}

pub type RingAccount = <Test as darwinia_evm::Config>::RingAccountBasic;
pub type KtonAccount = <Test as darwinia_evm::Config>::KtonAccountBasic;