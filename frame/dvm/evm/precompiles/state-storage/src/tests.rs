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
	ConsensusEngineId, PalletId,
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
use crate::FeeMarket;
use darwinia_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot,
};
use darwinia_evm::{runner::stack::Runner, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_evm_precompile_utils::test_helper::{
	address_build, create_function_encode_bytes, AccountInfo, LegacyUnsignedTransaction,
};
use darwinia_fee_market::{Config, RingBalance, Slasher};
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
	type OtherCurrencies = ();
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
		sp_std::vec![1, 2].into_iter().map(|x| H160::from_low_u64_be(x)).collect()
	}
}

impl<R> PrecompileSet for MockPrecompiles<R>
where
	FeeMarket<R, F1>: Precompile,
	FeeMarket<R, F2>: Precompile,
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
			a if a == to_address(1) =>
				Some(<FeeMarket<R, F1>>::execute(input, target_gas, context, is_static)),
			a if a == to_address(2) =>
				Some(<FeeMarket<R, F2>>::execute(input, target_gas, context, is_static)),
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
	type KtonCurrency = Kton;
	type PalletId = MockPalletId;
	type RingCurrency = Ring;
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
	// F1 configurations.
	pub const F1FeeMarketId: PalletId = PalletId(*b"da/feem1");
	pub const F1FeeMarketLockId: LockIdentifier = *b"da/feef1";
	// F2 configurations.
	pub const F2FeeMarketId: PalletId = PalletId(*b"da/feem2");
	pub const F2FeeMarketLockId: LockIdentifier = *b"da/feef2";
}

pub struct FeeMarketSlasher;
impl<T: Config<I>, I: 'static> Slasher<T, I> for FeeMarketSlasher {
	fn slash(_: RingBalance<T, I>, _: T::BlockNumber) -> RingBalance<T, I> {
		todo!("Not implemented for the test");
	}
}

impl Config<F1> for Test {
	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Event = Event;
	type LockId = F1FeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type PalletId = F1FeeMarketId;
	type RingCurrency = Ring;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = ();
}

impl Config<F2> for Test {
	type AssignedRelayersRewardRatio = AssignedRelayersRewardRatio;
	type CollateralPerOrder = CollateralPerOrder;
	type ConfirmRelayersRewardRatio = ConfirmRelayersRewardRatio;
	type Event = Event;
	type LockId = F2FeeMarketLockId;
	type MessageRelayersRewardRatio = MessageRelayersRewardRatio;
	type MinimumRelayFee = MinimumRelayFee;
	type PalletId = F2FeeMarketId;
	type RingCurrency = Ring;
	type Slasher = FeeMarketSlasher;
	type Slot = Slot;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = ();
}

pub use darwinia_fee_market::{Instance1 as F1, Instance2 as F2};

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
		FeeMarketInstance1: darwinia_fee_market::<Instance1>::{Pallet, Call, Storage, Event<T>},
		FeeMarketInstance2: darwinia_fee_market::<Instance2>::{Pallet, Call, Storage, Event<T>},
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
	use ethabi::StateMutability;
	// --- paritytech ---
	use fp_evm::CallOrCreateInfo;
	use frame_support::assert_ok;
	use sp_core::{H160, U256};
	use sp_std::str::FromStr;

	// SPDX-License-Identifier: MIT
	// pragma solidity >=0.8.10;

	// interface IMarketFee {
	//     function market_fee() external view returns (uint64);
	// }

	// contract TestMarketFee {
	//     function market_fee_1() public view returns (uint64) {
	//         address addr = 0x0000000000000000000000000000000000000001;
	//         return IMarketFee(addr).market_fee();
	//     }
	//     function market_fee_2() public view returns (uint64) {
	//         address addr = 0x0000000000000000000000000000000000000002;
	//         return IMarketFee(addr).market_fee();
	//     }
	// }

	const BYTECODE: &'static str = "0x608060405234801561001057600080fd5b50610183806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80635d2b05d61461003b57806387ff12b914610060575b600080fd5b610043610068565b60405167ffffffffffffffff909116815260200160405180910390f35b6100436100d7565b60008060019050806001600160a01b0316636673cb7a6040518163ffffffff1660e01b8152600401602060405180830381865afa1580156100ad573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906100d1919061011c565b91505090565b60008060029050806001600160a01b0316636673cb7a6040518163ffffffff1660e01b8152600401602060405180830381865afa1580156100ad573d6000803e3d6000fd5b60006020828403121561012e57600080fd5b815167ffffffffffffffff8116811461014657600080fd5b939250505056fea26469706673582212206b2f0c27a267a7c87c2deaa1c5086ca879a604b687b102788ddd800727ffdcb264736f6c634300080d0033";

	#[test]
	fn fetch_market_fee_in_multi_instance() {
		let (pairs, mut ext) = new_test_ext(7);
		let (a1, b1, c1, d1, e1, f1, g1) =
			(&pairs[0], &pairs[1], &pairs[2], &pairs[3], &pairs[4], &pairs[5], &pairs[6]);
		ext.execute_with(|| {
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed(a1.clone().account_id),
				100,
				None,
			));
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed(b1.clone().account_id),
				100,
				Some(20),
			));
			assert_ok!(FeeMarketInstance1::enroll_and_lock_collateral(
				Origin::signed(c1.clone().account_id),
				100,
				Some(30),
			));
			assert_eq!(FeeMarketInstance1::market_fee(), Some(30));

			assert_ok!(FeeMarketInstance2::enroll_and_lock_collateral(
				Origin::signed(d1.clone().account_id),
				100,
				None,
			));
			assert_ok!(FeeMarketInstance2::enroll_and_lock_collateral(
				Origin::signed(e1.clone().account_id),
				100,
				Some(40),
			));
			assert_ok!(FeeMarketInstance2::enroll_and_lock_collateral(
				Origin::signed(f1.clone().account_id),
				100,
				Some(50),
			));
			assert_eq!(FeeMarketInstance2::market_fee(), Some(50));

			// Deploy test contract
			let unsign_tx = LegacyUnsignedTransaction::new(
				0,
				1,
				300000,
				TransactionAction::Create,
				0,
				hex2bytes_unchecked(BYTECODE),
			);
			let tx = unsign_tx.sign_with_chain_id(&g1.private_key, 42);
			assert_ok!(Ethereum::execute(g1.address, &tx.into(), None));
			let created_addr = H160::from_str("ec3273d4fdc320f2286b14380cc835e7a5f1d845").unwrap();

			// Call market_fee_1
			let call_function = create_function_encode_bytes(
				"market_fee_1".to_owned(),
				vec![],
				vec![],
				true,
				StateMutability::NonPayable,
				&[],
			)
			.unwrap();
			let unsign_tx = LegacyUnsignedTransaction::new(
				1,
				1,
				300000,
				TransactionAction::Call(created_addr),
				0,
				hex2bytes_unchecked(bytes2hex("0x", call_function)),
			);
			let tx = unsign_tx.sign_with_chain_id(&g1.private_key, 42);
			let result =
				Ethereum::execute(g1.address, &tx.into(), None).map(|(_, _, res)| match res {
					CallOrCreateInfo::Call(info) => U256::from_big_endian(&info.value),
					CallOrCreateInfo::Create(_) => U256::default(),
				});
			assert_eq!(FeeMarketInstance1::market_fee().unwrap(), result.unwrap().as_u64());

			// Call market_fee_2
			let call_function = create_function_encode_bytes(
				"market_fee_2".to_owned(),
				vec![],
				vec![],
				true,
				StateMutability::NonPayable,
				&[],
			)
			.unwrap();
			let unsign_tx = LegacyUnsignedTransaction::new(
				2,
				1,
				300000,
				TransactionAction::Call(created_addr),
				0,
				hex2bytes_unchecked(bytes2hex("0x", call_function)),
			);
			let tx = unsign_tx.sign_with_chain_id(&g1.private_key, 42);
			let result =
				Ethereum::execute(g1.address, &tx.into(), None).map(|(_, _, res)| match res {
					CallOrCreateInfo::Call(info) => U256::from_big_endian(&info.value),
					CallOrCreateInfo::Create(_) => U256::default(),
				});
			assert_eq!(FeeMarketInstance2::market_fee().unwrap(), result.unwrap().as_u64());
		});
	}
}
