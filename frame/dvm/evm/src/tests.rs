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
use std::{collections::BTreeMap, str::FromStr};
// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	assert_ok,
	traits::{ConstU32, Everything, GenesisBuild},
	ConsensusEngineId,
};
use frame_system::mocking::*;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::{
	self as darwinia_evm,
	runner::{stack::Runner, Runner as RunnerT},
	*,
};
use darwinia_support::evm::ConcatConverter;

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Balance = u64;

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
	pub const ExistentialDeposit: u64 = 0;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
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
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}
impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

/// Fixed gas price of `0`.
pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		// Gas price is always one token per gas.
		1_000_000_000u128.into()
	}
}

pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(H160::from_str("1234500000000000000000000000000000000000").unwrap())
	}
}

impl<T> OnChargeEVMTransaction<T> for ()
where
	T: Config,
{
	type LiquidityInfo = U256;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, Error<T>> {
		EVMCurrencyAdapter::<()>::withdraw_fee(who, fee)
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) {
		<EVMCurrencyAdapter<()> as OnChargeEVMTransaction<T>>::correct_and_deposit_fee(
			who,
			corrected_fee,
			already_withdrawn,
		)
	}

	fn pay_priority_fee(tip: U256) {
		let author = Pallet::<T>::find_author();
		let account_balance = T::RingBalanceAdapter::evm_balance(&author);
		let new_account_balance = account_balance.saturating_add(tip);
		T::RingBalanceAdapter::mutate_evm_balance(&author, new_account_balance);
	}
}

/// Ensure that the origin is root.
pub struct EnsureAddressRoot<AccountId>(sp_std::marker::PhantomData<AccountId>);
impl<OuterOrigin, AccountId> EnsureAddressOrigin<OuterOrigin> for EnsureAddressRoot<AccountId>
where
	OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>> + From<RawOrigin<AccountId>>,
{
	type Success = ();

	fn try_address_origin(_address: &H160, origin: OuterOrigin) -> Result<(), OuterOrigin> {
		origin.into().and_then(|o| match o {
			RawOrigin::Root => Ok(()),
			r => Err(OuterOrigin::from(r)),
		})
	}
}

pub struct MockBalanceAdapter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> CurrencyAdapt<T> for MockBalanceAdapter<T> {
	fn evm_transfer(
		source: &T::AccountId,
		target: &T::AccountId,
		value: U256,
	) -> Result<(), ExitError> {
		if value == U256::zero() || source == target {
			return Ok(());
		}
		let source_balance = Self::account_balance(source);
		let new_source_balance = source_balance.saturating_sub(value);
		Self::mutate_account_balance(source, new_source_balance);

		let target_balance = Self::account_balance(target);
		let new_target_balance = target_balance.saturating_add(value);
		Self::mutate_account_balance(target, new_target_balance);

		Ok(())
	}

	fn ensure_can_withdraw(_: &T::AccountId, _: U256, _: WithdrawReasons) -> Result<(), ExitError> {
		Ok(())
	}

	fn account_balance(account_id: &T::AccountId) -> U256 {
		frame_support::storage::unhashed::get(&account_id.encode()).unwrap_or_default()
	}

	fn mutate_account_balance(account_id: &T::AccountId, balance: U256) {
		frame_support::storage::unhashed::put(&account_id.encode(), &balance);
	}

	fn evm_total_supply() -> U256 {
		U256::default()
	}
}

impl Config for Test {
	type BlockGasLimit = ();
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressRoot<Self::AccountId>;
	type ChainId = ();
	type Event = Event;
	type FeeCalculator = FixedGasPrice;
	type FindAuthor = FindAuthorTruncated;
	type GasWeightMapping = ();
	type IntoAccountId = ConcatConverter<Self::AccountId>;
	type KtonBalanceAdapter = MockBalanceAdapter<Self>;
	type OnChargeTransaction = ();
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type RingBalanceAdapter = MockBalanceAdapter<Self>;
	type Runner = Runner<Self>;
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
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut accounts = BTreeMap::new();
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000001").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0x00, // STOP
			],
		},
	);
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000002").unwrap(),
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::from(1000000),
			storage: Default::default(),
			code: vec![
				0xff, // INVALID
			],
		},
	);

	accounts.insert(
		H160::default(), // root
		GenesisAccount {
			nonce: U256::from(1),
			balance: U256::max_value(),
			storage: Default::default(),
			code: vec![],
		},
	);

	<darwinia_evm::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
		&darwinia_evm::GenesisConfig { accounts },
		&mut t,
	)
	.unwrap();
	t.into()
}

#[test]
fn fail_call_return_ok() {
	new_test_ext().execute_with(|| {
		<Test as Config>::RingBalanceAdapter::mutate_evm_balance(
			&H160::default(),
			U256::max_value(),
		);

		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_000_000_000),
			None,
			None,
			Vec::new(),
		));

		assert_ok!(EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::from(1_000_000_000),
			None,
			None,
			Vec::new(),
		));
	});
}

#[test]
fn fee_deduction() {
	new_test_ext().execute_with(|| {
		// Create an EVM address and the corresponding Substrate address that will be charged fees and refunded
		let evm_addr = H160::from_str("1000000000000000000000000000000000000003").unwrap();
		// Seed account
		<Test as Config>::RingBalanceAdapter::mutate_evm_balance(
			&evm_addr,
			U256::from(100),
		);
		assert_eq!(
			<Test as Config>::RingBalanceAdapter::evm_balance(&evm_addr),
			U256::from(100)
		);

		// Deduct fees as 10 units
		let imbalance = <<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::withdraw_fee(&evm_addr, U256::from(10)).unwrap();
		assert_eq!(
			<Test as Config>::RingBalanceAdapter::evm_balance(&evm_addr),
			U256::from(90)
		);

		// Refund fees as 5 units
		<<Test as Config>::OnChargeTransaction as OnChargeEVMTransaction<Test>>::correct_and_deposit_fee(&evm_addr, U256::from(5), imbalance);
		assert_eq!(
			<Test as Config>::RingBalanceAdapter::evm_balance(&evm_addr),
			U256::from(95)
		);
	});
}

#[test]
fn find_author() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		assert_eq!(author, H160::from_str("1234500000000000000000000000000000000000").unwrap());
	});
}

#[test]
fn author_should_get_tip() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(2_000_000_000),
			Some(U256::from(1)),
			None,
			Vec::new(),
		);
		let after_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		assert_eq!(after_tip, (before_tip + 21000));
	});
}

#[test]
fn author_same_balance_without_tip() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::default(),
			1000000,
			U256::default(),
			None,
			None,
			Vec::new(),
		);
		let after_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		assert_eq!(after_tip, before_tip);
	});
}

#[test]
fn refunds_should_work() {
	new_test_ext().execute_with(|| {
		let before_call = <Test as Config>::RingBalanceAdapter::evm_balance(&H160::default());
		// Gas price is not part of the actual fee calculations anymore, only the base fee.
		//
		// Because we first deduct max_fee_per_gas * gas_limit (2_000_000_000 * 1000000) we need
		// to ensure that the difference (max fee VS base fee) is refunded.
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(2_000_000_000),
			None,
			None,
			Vec::new(),
		);
		let total_cost =
			(U256::from(21_000) * <Test as Config>::FeeCalculator::min_gas_price()) + U256::from(1);
		let after_call = <Test as Config>::RingBalanceAdapter::evm_balance(&H160::default());
		assert_eq!(after_call, before_call - total_cost);
	});
}

#[test]
fn refunds_and_priority_should_work() {
	new_test_ext().execute_with(|| {
		let author = EVM::find_author();
		let before_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		let before_call = <Test as Config>::RingBalanceAdapter::evm_balance(&H160::default());
		// We deliberately set a base fee + max tip > max fee.
		// The effective priority tip will be 1GWEI instead 1.5GWEI:
		// 		(max_fee_per_gas - base_fee).min(max_priority_fee)
		//		(2 - 1).min(1.5)
		let tip = U256::from(1_500_000_000);
		let max_fee_per_gas = U256::from(2_000_000_000);
		let used_gas = U256::from(21_000);
		let _ = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			max_fee_per_gas,
			Some(tip),
			None,
			Vec::new(),
		);
		let base_fee = <Test as Config>::FeeCalculator::min_gas_price();
		let actual_tip = (max_fee_per_gas - base_fee).min(tip) * used_gas;
		let total_cost = (used_gas * base_fee) + U256::from(actual_tip) + U256::from(1);
		// The tip is deducted but never refunded to the caller.
		let after_call = <Test as Config>::RingBalanceAdapter::evm_balance(&H160::default());
		assert_eq!(after_call, before_call - total_cost);

		let after_tip = <Test as Config>::RingBalanceAdapter::evm_balance(&author);
		assert_eq!(after_tip, (before_tip + actual_tip.low_u128()));
	});
}

#[test]
fn handle_sufficient_reference() {
	new_test_ext().execute_with(|| {
		let addr = H160::from_str("1230000000000000000000000000000000000001").unwrap();
		let addr_2 = H160::from_str("1234000000000000000000000000000000000001").unwrap();
		let substrate_addr =
			<Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(&addr);
		let substrate_addr_2 =
			<Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(&addr_2);

		// Sufficients should increase when creating EVM accounts.
		let _ = <crate::AccountCodes<Test>>::insert(addr, &vec![0]);
		let account = frame_system::Account::<Test>::get(substrate_addr);
		// Using storage is not correct as it leads to a sufficient reference mismatch.
		assert_eq!(account.sufficients, 0);

		// Using the create / remove account functions is the correct way to handle it.
		EVM::create_account(&addr_2, vec![1, 2, 3]);
		let account_2 = frame_system::Account::<Test>::get(substrate_addr_2.clone());
		// We increased the sufficient reference by 1.
		assert_eq!(account_2.sufficients, 1);
		EVM::remove_account(&addr_2);
		let account_2 = frame_system::Account::<Test>::get(substrate_addr_2);
		// We decreased the sufficient reference by 1 on removing the account.
		assert_eq!(account_2.sufficients, 0);
	});
}

#[test]
fn call_should_fail_with_priority_greater_than_max_fee() {
	new_test_ext().execute_with(|| {
		// Max priority greater than max fee should fail.
		let tip: u128 = 1_100_000_000;
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(1_000_000_000),
			Some(U256::from(tip)),
			None,
			Vec::new(),
		);
		assert!(result.is_err());
	});
}

#[test]
fn call_should_succeed_with_priority_equal_to_max_fee() {
	new_test_ext().execute_with(|| {
		let tip: u128 = 1_000_000_000;
		// Mimics the input for pre-eip-1559 transaction types where `gas_price`
		// is used for both `max_fee_per_gas` and `max_priority_fee_per_gas`.
		let result = EVM::call(
			Origin::root(),
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1),
			1000000,
			U256::from(1_000_000_000),
			Some(U256::from(tip)),
			None,
			Vec::new(),
		);
		assert!(result.is_ok());
	});
}

#[test]
fn runner_non_transactional_calls_with_non_balance_accounts_is_ok_without_gas_price() {
	// Expect to skip checks for gas price and account balance when both:
	// 	- The call is non transactional (`is_transactional == false`).
	// 	- The `max_fee_per_gas` is None.
	new_test_ext().execute_with(|| {
		let non_balance_account =
			H160::from_str("7700000000000000000000000000000000000001").unwrap();
		assert_eq!(EVM::account_basic(&non_balance_account).balance, U256::zero());
		let _ = <Test as Config>::Runner::call(
			non_balance_account,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			None,
			None,
			None,
			Vec::new(),
			false,
			&<Test as Config>::config().clone(),
		)
		.expect("Non transactional call succeeds");
		assert_eq!(EVM::account_basic(&non_balance_account).balance, U256::zero());
	});
}

#[test]
fn runner_non_transactional_calls_with_non_balance_accounts_is_err_with_gas_price() {
	// In non transactional calls where `Some(gas_price)` is defined, expect it to be
	// checked against the `BaseFee`, and expect the account to have enough balance
	// to pay for the call.
	new_test_ext().execute_with(|| {
		let non_balance_account =
			H160::from_str("7700000000000000000000000000000000000001").unwrap();
		assert_eq!(EVM::account_basic(&non_balance_account).balance, U256::zero());
		let res = <Test as Config>::Runner::call(
			non_balance_account,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			None,
			None,
			Vec::new(),
			false,
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}

#[test]
#[ignore]
fn runner_transactional_call_with_zero_gas_price_fails() {
	// Transactional calls are rejected when `max_fee_per_gas == None`.
	new_test_ext().execute_with(|| {
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			None,
			None,
			None,
			Vec::new(),
			true,
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}

#[test]
fn runner_max_fee_per_gas_gte_max_priority_fee_per_gas() {
	// Transactional and non transactional calls enforce `max_fee_per_gas >=
	// max_priority_fee_per_gas`.
	new_test_ext().execute_with(|| {
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			Some(U256::from(2_000_000_000)),
			None,
			Vec::new(),
			true,
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
		let res = <Test as Config>::Runner::call(
			H160::default(),
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			Vec::new(),
			U256::from(1u32),
			1000000,
			Some(U256::from(1_000_000_000)),
			Some(U256::from(2_000_000_000)),
			None,
			Vec::new(),
			false,
			&<Test as Config>::config().clone(),
		);
		assert!(res.is_err());
	});
}
