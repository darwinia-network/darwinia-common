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

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use fp_evm::{
	CallOrCreateInfo, Context, ExitRevert, FeeCalculator, Precompile, PrecompileFailure,
	PrecompileResult, PrecompileSet,
};
use frame_support::{
	pallet_prelude::Weight,
	traits::{ConstU32, Everything, FindAuthor, GenesisBuild},
	weights::GetDispatchInfo,
	ConsensusEngineId, PalletId,
};
use frame_system::mocking::*;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	transaction_validity::{InvalidTransaction, TransactionValidity, TransactionValidityError},
	AccountId32, Perbill, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*, str::FromStr};
// --- darwinia-network ---
use crate::*;
use darwinia_ethereum::{
	adapter::{CurrencyAdapter, KtonRemainBalance, RingRemainBalance},
	EthereumBlockHashMapping, IntermediateStateRoot, Log, RawOrigin, Transaction,
	TransactionAction,
};
use darwinia_evm::{runner::stack::Runner, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_evm_precompile_utils::test_helper::{
	address_build, AccountInfo, LegacyUnsignedTransaction,
};
use darwinia_support::evm::DeriveSubstrateAddress;

type Block = MockBlock<Test>;
type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u128;

darwinia_support::impl_test_account_data! {}

pub const INITIAL_BALANCE: Balance = 1_000;
pub const PRECOMPILE_ADDR: &str = "0x000000000000000000000000000000000000000a";

frame_support::parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
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
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaxConsumers = ConstU32<16>;
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
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 10;
	pub const ExistentialDeposit: u64 = 0;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
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
impl DeriveSubstrateAddress<AccountId32> for HashedConverter {
	fn derive_substrate_address(address: &H160) -> AccountId32 {
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

pub struct MockPrecompiles<Runtime>(PhantomData<Runtime>);
impl<Runtime> MockPrecompiles<Runtime>
where
	Runtime: darwinia_ethereum::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn used_addresses() -> [H160; 1] {
		[addr(10)]
	}
}

pub(crate) const TOKEN_NAME: &str = "MockERC20";
pub(crate) const TOKEN_SYMBOL: &str = "MOCK";
pub(crate) const TOKEN_DECIMAL: u8 = 18;
pub struct MockERC20MetaData;
impl Erc20Metadata for MockERC20MetaData {
	fn name() -> &'static str {
		TOKEN_NAME
	}

	fn symbol() -> &'static str {
		TOKEN_SYMBOL
	}

	fn decimals() -> u8 {
		TOKEN_DECIMAL
	}
}

impl<Runtime> PrecompileSet for MockPrecompiles<Runtime>
where
	KtonERC20<Runtime, MockERC20MetaData>: Precompile,
	Runtime: darwinia_ethereum::Config,
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

		// Filter known precompile addresses except Ethereum officials
		if self.is_precompile(address) && address > to_address(9) && address != context.address {
			return Some(Err(PrecompileFailure::Revert {
				exit_status: ExitRevert::Reverted,
				output: b"cannot be called with DELEGATECALL or CALLCODE".to_vec(),
				cost: 0,
			}));
		};

		match address {
			_ if address == to_address(10) =>
				Some(<KtonERC20<Runtime, MockERC20MetaData>>::execute(
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

impl darwinia_evm::Config for Test {
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type ChainId = ChainId;
	type Event = Event;
	type FeeCalculator = FixedGasPrice;
	type FindAuthor = FindAuthorTruncated;
	type GasWeightMapping = ();
	type IntoAccountId = HashedConverter;
	type KtonBalanceAdapter = CurrencyAdapter<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type RingBalanceAdapter = CurrencyAdapter<Self, Ring, RingRemainBalance>;
	type Runner = Runner<Self>;
}

impl darwinia_ethereum::Config for Test {
	type Event = Event;
	type StateRoot = IntermediateStateRoot;
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
			Call::Ethereum(ref call) => Some(validate_self_contained_inner(self, call, info)),
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

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let pairs = (0..accounts_len).map(|i| address_build(i as u8)).collect::<Vec<_>>();

	let balances: Vec<_> =
		(0..accounts_len).map(|i| (pairs[i].account_id.clone(), INITIAL_BALANCE)).collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances: balances.clone() }
		.assimilate_storage(&mut t)
		.unwrap();
	darwinia_balances::GenesisConfig::<Test, KtonInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));

	(pairs, ext)
}

pub fn construct_tx_asserter(nonce: u64, input: Vec<u8>, account: &AccountInfo) -> Asserter {
	let tx = LegacyUnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: <Test as darwinia_evm::Config>::FeeCalculator::min_gas_price(),
		gas_limit: U256::from(1_000_000),
		action: TransactionAction::Call(H160::from_str(PRECOMPILE_ADDR).unwrap()),
		value: U256::zero(),
		input,
	}
	.sign_with_chain_id(&account.private_key, <Test as darwinia_evm::Config>::ChainId::get());

	Asserter { sender: account.address, tx, executed_value: None, logs: None }
}

#[derive(Debug, Clone)]
pub struct Asserter {
	pub sender: H160,
	pub tx: Transaction,
	pub executed_value: Option<Vec<u8>>,
	pub logs: Option<Vec<Log>>,
}

impl Asserter {
	pub fn execute(mut self) -> Self {
		let info =
			Ethereum::execute(self.sender, &self.tx.clone().into(), None).map(|(_, _, res)| {
				match res {
					CallOrCreateInfo::Call(info) => info,
					CallOrCreateInfo::Create(_) => todo!(),
				}
			});
		self.executed_value = info.clone().ok().map(|info| info.value);
		self.logs = info.ok().map(|info| info.logs);
		self
	}

	pub fn assert_executed_value(&self, actual: &[u8]) -> &Self {
		assert_eq!(self.executed_value, Some(actual.to_vec()));
		self
	}

	pub fn assert_has_log(&self, log: &Log) -> &Self {
		assert!(self.logs.as_ref().unwrap().contains(log));
		self
	}

	pub fn assert_revert(&self, revert_value: &[u8]) -> &Self {
		assert_eq!(self.executed_value.clone().unwrap()[4..], revert_value.to_vec());
		self
	}
}
