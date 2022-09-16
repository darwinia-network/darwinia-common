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
use array_bytes::hex2bytes;
use codec::{Decode, Encode, MaxEncodedLen};
use evm::ExitError;
use scale_info::TypeInfo;
// --- paritytech ---
use fp_evm::{FeeCalculator, GenesisAccount};
use frame_support::{
	traits::{Everything, FindAuthor, GenesisBuild, WithdrawReasons},
	ConsensusEngineId,
};
use frame_system::{mocking::*, RawOrigin};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{self as darwinia_message_gadget, *};
use darwinia_evm::{
	runner::{stack::Runner, Runner as RunnerT},
	CurrencyAdapt, EVMCurrencyAdapter, EnsureAddressOrigin, SubstrateBlockHashMapping,
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
impl<T: darwinia_evm::Config> CurrencyAdapt<T> for MockBalanceAdapter<T> {
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

impl darwinia_evm::Config for Test {
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
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type RingBalanceAdapter = MockBalanceAdapter<Self>;
	type Runner = Runner<Self>;
}

impl Config for Test {}

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
		MessageGadget: darwinia_message_gadget::{Pallet, Call, Storage, Config},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut accounts = BTreeMap::new();
	accounts.insert(
		H160::from_str("1000000000000000000000000000000000000001").unwrap(),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_call() {
		new_test_ext().execute_with(|| {
			// pragma solidity ^0.8.0;
			// 
			// contract MessageRootGetter {
			//     function commitment() public returns (bool) {
			//         return true;
			//     }
			// }
			const CONTRACT_CODE: &str = "0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c80631303a48414602d575b600080fd5b60336047565b604051603e9190605d565b60405180910390f35b60006001905090565b6057816076565b82525050565b6000602082019050607060008301846050565b92915050565b6000811515905091905056fea26469706673582212205edcbb73cc70f096b015d00b65ed893df280a01c9e90e964e8bb39957d6d3c9d64736f6c63430008070033";
			let res = <Test as darwinia_evm::Config>::Runner::create(
				H160::from_str("1000000000000000000000000000000000000001").unwrap(),
				hex2bytes(CONTRACT_CODE).unwrap(),
				U256::zero(),
				U256::from(300_000_000).low_u64(),
				Some(<Test as darwinia_evm::Config>::FeeCalculator::min_gas_price()),
				None,
				Some(U256::from(1)),
				vec![],
				true,
				<Test as darwinia_evm::Config>::config(),
			);
			let contract_address = res.unwrap().value;
			CommitmentContract::<Test>::put(contract_address);

			assert_eq!(MessageGadget::commitment_contract(), contract_address);
			assert_eq!(MessageRootGetter::<Test>::get(), Some(H256::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])));
		});
	}
}
