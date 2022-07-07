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
use ethereum::TransactionAction;
use scale_info::TypeInfo;
// --- paritytech ---
use darwinia_ethereum::{EthereumBlockHashMapping, RawOrigin};
use fp_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	pallet_prelude::Weight,
	traits::{Everything, FindAuthor, GenesisBuild, LockIdentifier},
	weights::GetDispatchInfo,
	ConsensusEngineId, PalletId, StorageHasher, Twox128,
};
use frame_system::mocking::*;
use pallet_evm::FeeCalculator;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	transaction_validity::{InvalidTransaction, TransactionValidity, TransactionValidityError},
	AccountId32, Perbill, Permill, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use crate::{StateStorage, StorageFilterT};
use darwinia_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot,
};
use darwinia_evm::{runner::stack::Runner, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_evm_precompile_utils::test_helper::{
	address_build, create_function_encode_bytes, AccountInfo, LegacyUnsignedTransaction,
};
use darwinia_support::evm::DeriveSubstrateAddress;
use pallet_fee_market::{BalanceOf, Config, Slasher};

type Block = MockBlock<Test>;
type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u64;

use pallet_fee_market::Instance1 as F1;

darwinia_support::impl_test_account_data! {}

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
	fn derive_substrate_address(address: H160) -> AccountId32 {
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

pub struct StorageFilter;
impl StorageFilterT for StorageFilter {
	fn allow(prefix: &[u8]) -> bool {
		prefix != Twox128::hash(b"EVM") && prefix != Twox128::hash(b"Ethereum")
	}
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
		sp_std::vec![1].into_iter().map(|x| H160::from_low_u64_be(x)).collect()
	}
}

impl<R> PrecompileSet for MockPrecompiles<R>
where
	StateStorage<R, StorageFilter>: Precompile,
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
			a if a == to_address(1) => Some(<StateStorage<R, StorageFilter>>::execute(
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
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type ChainId = ChainId;
	type Event = Event;
	type FeeCalculator = FixedGasPrice;
	type FindAuthor = FindAuthorTruncated;
	type GasWeightMapping = ();
	type IntoAccountId = HashedConverter;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type Runner = Runner<Self>;
}

frame_support::parameter_types! {
	pub const MockPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl darwinia_ethereum::Config for Test {
	type Event = Event;
	type PalletId = MockPalletId;
	type StateRoot = IntermediateStateRoot;
}

frame_support::parameter_types! {
	// Shared configurations.
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const MinimumRelayFee: Balance = 15;
	pub const CollateralPerOrder: Balance = 50;
	pub const Slot: u64 = 600;
	pub const AssignedRelayersRewardRatio: Permill = Permill::from_percent(60);
	pub const MessageRelayersRewardRatio: Permill = Permill::from_percent(80);
	pub const ConfirmRelayersRewardRatio: Permill = Permill::from_percent(20);
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
}

pub struct FeeMarketSlasher;
impl<T: Config<I>, I: 'static> Slasher<T, I> for FeeMarketSlasher {
	fn slash(_: BalanceOf<T, I>, _: T::BlockNumber) -> BalanceOf<T, I> {
		todo!("Not implemented for the test");
	}
}

impl Config<F1> for Test {
	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Currency = Ring;
	type Event = Event;
	type LockId = FeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
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
		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: darwinia_ethereum::{Pallet, Call, Storage, Config, Event<T>, Origin},
		FeeMarketInstance1: pallet_fee_market::<Instance1>::{Pallet, Call, Storage, Event<T>},
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
	let pairs = (0..accounts_len).map(|i| address_build(i as u8)).collect::<Vec<_>>();
	let balances: Vec<_> = (0..accounts_len).map(|i| (pairs[i].account_id.clone(), 500)).collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));

	(pairs, ext.into())
}

#[cfg(test)]
mod tests {
	use super::*;
	// --- crates.io ---
	use array_bytes::{bytes2hex, hex2bytes_unchecked};
	use ethabi::{Param, ParamType, StateMutability, Token};
	// --- paritytech ---
	use fp_evm::CallOrCreateInfo;
	use frame_support::{assert_ok, Blake2_128Concat, StorageHasher, Twox128};
	use sp_core::H160;
	use sp_std::str::FromStr;

	macro_rules! prepare {
		($a1:expr, $a2:expr, $a3:expr, $a4: expr) => {
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed($a1.clone().account_id),
				100,
				None,
			));
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed($a2.clone().account_id),
				100,
				Some(20),
			));
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed($a3.clone().account_id),
				100,
				Some(30),
			));
			assert_eq!(FeeMarketInstance1::market_fee(), Some(30));

			// Deploy test contract
			let unsign_tx = LegacyUnsignedTransaction::new(
				0,
				1,
				300000,
				TransactionAction::Create,
				0,
				hex2bytes_unchecked(BYTECODE),
			);
			let tx = unsign_tx.sign_with_chain_id(&$a4.private_key, 42);
			assert_ok!(Ethereum::execute($a4.address, &tx.into(), None));
		};
	}

	// SPDX-License-Identifier: MIT
	// pragma solidity >=0.8.10;

	// interface IStateStorage {
	//     function state_storage(bytes memory) external view returns (bytes memory);
	// }

	// contract TestStateStorage {
	//     function state_storage(bytes memory key) public view returns (bytes memory) {
	//         address addr = 0x0000000000000000000000000000000000000001;
	//         return IStateStorage(addr).state_storage(key);
	//     }
	// }

	const BYTECODE: &'static str = "608060405234801561001057600080fd5b5061042b806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c806378943fb714610030575b600080fd5b61004a60048036038101906100459190610249565b610060565b604051610057919061031a565b60405180910390f35b60606000600190508073ffffffffffffffffffffffffffffffffffffffff166378943fb7846040518263ffffffff1660e01b81526004016100a1919061031a565b600060405180830381865afa1580156100be573d6000803e3d6000fd5b505050506040513d6000823e3d601f19601f820116820180604052508101906100e791906103ac565b915050919050565b6000604051905090565b600080fd5b600080fd5b600080fd5b600080fd5b6000601f19601f8301169050919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b6101568261010d565b810181811067ffffffffffffffff821117156101755761017461011e565b5b80604052505050565b60006101886100ef565b9050610194828261014d565b919050565b600067ffffffffffffffff8211156101b4576101b361011e565b5b6101bd8261010d565b9050602081019050919050565b82818337600083830152505050565b60006101ec6101e784610199565b61017e565b90508281526020810184848401111561020857610207610108565b5b6102138482856101ca565b509392505050565b600082601f8301126102305761022f610103565b5b81356102408482602086016101d9565b91505092915050565b60006020828403121561025f5761025e6100f9565b5b600082013567ffffffffffffffff81111561027d5761027c6100fe565b5b6102898482850161021b565b91505092915050565b600081519050919050565b600082825260208201905092915050565b60005b838110156102cc5780820151818401526020810190506102b1565b838111156102db576000848401525b50505050565b60006102ec82610292565b6102f6818561029d565b93506103068185602086016102ae565b61030f8161010d565b840191505092915050565b6000602082019050818103600083015261033481846102e1565b905092915050565b600061034f61034a84610199565b61017e565b90508281526020810184848401111561036b5761036a610108565b5b6103768482856102ae565b509392505050565b600082601f83011261039357610392610103565b5b81516103a384826020860161033c565b91505092915050565b6000602082840312156103c2576103c16100f9565b5b600082015167ffffffffffffffff8111156103e0576103df6100fe565b5b6103ec8482850161037e565b9150509291505056fea2646970667358221220f62ecc9b0279c6231c740c77273268e7f4338aae4a244ac7ff9a2ffbc5046a4064736f6c634300080d0033";

	#[test]
	fn get_state_storage_works() {
		let (pairs, mut ext) = new_test_ext(4);
		let (a1, a2, a3, a4) = (&pairs[0], &pairs[1], &pairs[2], &pairs[3]);
		ext.execute_with(|| {
			prepare!(a1, a2, a3, a4);
			let contract = H160::from_str("0x35ffc084a84df2c259518c91c0f8b473c4f8d017").unwrap();

			let mut key = vec![0u8; 32];
			key[0..16].copy_from_slice(&Twox128::hash(b"FeeMarketInstance1"));
			key[16..32].copy_from_slice(&Twox128::hash(b"AssignedRelayers"));

			// Call state_storage
			let call_function = create_function_encode_bytes(
				"state_storage".to_owned(),
				vec![Param { name: "key".to_owned(), kind: ParamType::Bytes, internal_type: None }],
				vec![Param { name: "res".to_owned(), kind: ParamType::Bytes, internal_type: None }],
				true,
				StateMutability::NonPayable,
				&[Token::Bytes(key)],
			)
			.unwrap();
			let unsign_tx = LegacyUnsignedTransaction::new(
				1,
				1,
				300000,
				TransactionAction::Call(contract),
				0,
				hex2bytes_unchecked(&bytes2hex("0x", &call_function)),
			);
			let tx = unsign_tx.sign_with_chain_id(&a4.private_key, 42);
			let result =
				Ethereum::execute(a4.address, &tx.into(), None).map(|(_, _, res)| match res {
					CallOrCreateInfo::Call(info) => info.value,
					CallOrCreateInfo::Create(_) => todo!(),
				});
			assert!(result.unwrap().len() != 0);
		});
	}

	#[test]
	fn storage_filter_works() {
		let (pairs, mut ext) = new_test_ext(4);
		let (a1, a2, a3, a4) = (&pairs[0], &pairs[1], &pairs[2], &pairs[3]);
		ext.execute_with(|| {
			prepare!(a1, a2, a3, a4);
			let contract = H160::from_str("0x35ffc084a84df2c259518c91c0f8b473c4f8d017").unwrap();

			let mut key = Vec::new();
			key.extend_from_slice(&Twox128::hash(b"EVM"));
			key.extend_from_slice(&Twox128::hash(b"AccountCodes"));
			key.extend_from_slice(&Blake2_128Concat::hash(&Encode::encode(&contract)));

			// Call state_storage
			let call_function = create_function_encode_bytes(
				"state_storage".to_owned(),
				vec![Param { name: "key".to_owned(), kind: ParamType::Bytes, internal_type: None }],
				vec![Param { name: "res".to_owned(), kind: ParamType::Bytes, internal_type: None }],
				true,
				StateMutability::NonPayable,
				&[Token::Bytes(key)],
			)
			.unwrap();
			let unsign_tx = LegacyUnsignedTransaction::new(
				1,
				1,
				300000,
				TransactionAction::Call(contract),
				0,
				hex2bytes_unchecked(&bytes2hex("0x", &call_function)),
			);
			let tx = unsign_tx.sign_with_chain_id(&a4.private_key, 42);
			let result =
				Ethereum::execute(a4.address, &tx.into(), None).map(|(_, _, res)| match res {
					CallOrCreateInfo::Call(info) => info.value,
					CallOrCreateInfo::Create(_) => todo!(),
				});
			assert_eq!(
				ethabi::decode(&[ParamType::String], &result.unwrap()[4..]).unwrap()[0],
				Token::String("Read restriction".to_string())
			);
		});
	}
}
