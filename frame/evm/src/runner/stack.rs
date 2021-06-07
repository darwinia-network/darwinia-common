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

//! EVM stack-based runner.

// --- crates.io ---
use evm::{
	backend::Backend as BackendT,
	executor::{StackExecutor, StackState as StackStateT, StackSubstateMetadata},
	ExitError, ExitReason, Transfer,
};
use sha3::{Digest, Keccak256};
// --- substrate ---
use frame_support::{ensure, traits::Get};
use sp_core::{H160, H256, U256};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::{collections::btree_set::BTreeSet, marker::PhantomData, mem, prelude::*};
// --- darwinia ---
use crate::{
	runner::Runner as RunnerT, AccountBasic, AccountCodes, AccountStorages, AddressMapping, Config,
	Error, Event, FeeCalculator, Pallet, PrecompileSet,
};
use dp_evm::{CallInfo, CreateInfo, ExecutionInfo, Log, Vicinity};

#[derive(Default)]
pub struct Runner<T: Config> {
	_marker: PhantomData<T>,
}
impl<T: Config> Runner<T> {
	/// Execute an EVM operation.
	pub fn execute<'config, F, R>(
		source: H160,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &'config evm::Config,
		f: F,
	) -> Result<ExecutionInfo<R>, Error<T>>
	where
		F: FnOnce(
			&mut StackExecutor<'config, SubstrateStackState<'_, 'config, T>>,
		) -> (ExitReason, R),
	{
		// Gas price check is skipped when performing a gas estimation.
		let gas_price = match gas_price {
			Some(gas_price) => {
				ensure!(
					gas_price >= T::FeeCalculator::min_gas_price(),
					Error::<T>::GasPriceTooLow
				);
				gas_price
			}
			None => Default::default(),
		};

		let vicinity = Vicinity {
			gas_price,
			origin: source,
		};

		let metadata = StackSubstateMetadata::new(gas_limit, &config);
		let state = SubstrateStackState::new(&vicinity, metadata);
		let mut executor =
			StackExecutor::new_with_precompile(state, config, T::Precompiles::execute);

		let total_fee = gas_price
			.checked_mul(U256::from(gas_limit))
			.ok_or(Error::<T>::FeeOverflow)?;
		let total_payment = value
			.checked_add(total_fee)
			.ok_or(Error::<T>::PaymentOverflow)?;
		let source_account = T::RingAccountBasic::account_basic(&source);
		ensure!(
			source_account.balance >= total_payment,
			Error::<T>::BalanceLow
		);

		if let Some(nonce) = nonce {
			ensure!(source_account.nonce == nonce, Error::<T>::InvalidNonce);
		}

		Pallet::<T>::withdraw_fee(&source, total_fee);
		let (reason, retv) = f(&mut executor);

		let used_gas = U256::from(executor.used_gas());
		let actual_fee = executor.fee(gas_price);
		log::debug!(
			target: "evm",
			"Execution {:?} [source: {:?}, value: {}, gas_limit: {}, actual_fee: {}]",
			reason,
			source,
			value,
			gas_limit,
			actual_fee
		);
		Pallet::<T>::deposit_fee(&source, total_fee.saturating_sub(actual_fee));

		let state = executor.into_state();

		for address in state.substate.deletes {
			log::debug!(
				target: "evm",
				"Deleting account at {:?}",
				address
			);
			<Pallet<T>>::remove_account(&address)
		}

		for substrate_log in &state.substate.logs {
			log::trace!(
				target: "evm",
				"Inserting log for {:?}, topics ({}) {:?}, data ({}): {:?}]",
				substrate_log.address,
				substrate_log.topics.len(),
				substrate_log.topics,
				substrate_log.data.len(),
				substrate_log.data
			);
			Pallet::<T>::deposit_event(Event::<T>::Log(Log {
				address: substrate_log.address,
				topics: substrate_log.topics.clone(),
				data: substrate_log.data.clone(),
			}));
		}

		Ok(ExecutionInfo {
			value: retv,
			exit_reason: reason,
			used_gas,
			logs: state.substate.logs,
		})
	}
}

impl<T: Config> RunnerT<T> for Runner<T> {
	type Error = Error<T>;

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CallInfo, Self::Error> {
		Self::execute(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| executor.transact_call(source, target, value, input, gas_limit),
		)
	}

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error> {
		Self::execute(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| {
				let address = executor.create_address(evm::CreateScheme::Legacy { caller: source });
				(
					executor.transact_create(source, value, init, gas_limit),
					address,
				)
			},
		)
	}

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u64,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error> {
		let code_hash = H256::from_slice(Keccak256::digest(&init).as_slice());
		Self::execute(
			source,
			value,
			gas_limit,
			gas_price,
			nonce,
			config,
			|executor| {
				let address = executor.create_address(evm::CreateScheme::Create2 {
					caller: source,
					code_hash,
					salt,
				});
				(
					executor.transact_create2(source, value, init, salt, gas_limit),
					address,
				)
			},
		)
	}
}

struct SubstrateStackSubstate<'config> {
	metadata: StackSubstateMetadata<'config>,
	deletes: BTreeSet<H160>,
	logs: Vec<Log>,
	parent: Option<Box<SubstrateStackSubstate<'config>>>,
}

impl<'config> SubstrateStackSubstate<'config> {
	pub fn metadata(&self) -> &StackSubstateMetadata<'config> {
		&self.metadata
	}

	pub fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
		&mut self.metadata
	}

	pub fn enter(&mut self, gas_limit: u64, is_static: bool) {
		let mut entering = Self {
			metadata: self.metadata.spit_child(gas_limit, is_static),
			parent: None,
			deletes: BTreeSet::new(),
			logs: Vec::new(),
		};
		mem::swap(&mut entering, self);

		self.parent = Some(Box::new(entering));

		sp_io::storage::start_transaction();
	}

	pub fn exit_commit(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot commit on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_commit(exited.metadata)?;
		self.logs.append(&mut exited.logs);
		self.deletes.append(&mut exited.deletes);

		sp_io::storage::commit_transaction();
		Ok(())
	}

	pub fn exit_revert(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot discard on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_revert(exited.metadata)?;

		sp_io::storage::rollback_transaction();
		Ok(())
	}

	pub fn exit_discard(&mut self) -> Result<(), ExitError> {
		let mut exited = *self.parent.take().expect("Cannot discard on root substate");
		mem::swap(&mut exited, self);

		self.metadata.swallow_discard(exited.metadata)?;

		sp_io::storage::rollback_transaction();
		Ok(())
	}

	pub fn deleted(&self, address: H160) -> bool {
		if self.deletes.contains(&address) {
			return true;
		}
		if let Some(parent) = self.parent.as_ref() {
			return parent.deleted(address);
		}
		false
	}

	pub fn set_deleted(&mut self, address: H160) {
		self.deletes.insert(address);
	}

	pub fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
		self.logs.push(Log {
			address,
			topics,
			data,
		});
	}
}

/// Substrate backend for EVM.
pub struct SubstrateStackState<'vicinity, 'config, T> {
	vicinity: &'vicinity Vicinity,
	substate: SubstrateStackSubstate<'config>,
	_marker: PhantomData<T>,
}

impl<'vicinity, 'config, T: Config> SubstrateStackState<'vicinity, 'config, T> {
	/// Create a new backend with given vicinity.
	pub fn new(vicinity: &'vicinity Vicinity, metadata: StackSubstateMetadata<'config>) -> Self {
		Self {
			vicinity,
			substate: SubstrateStackSubstate {
				metadata,
				deletes: BTreeSet::new(),
				logs: Vec::new(),
				parent: None,
			},
			_marker: PhantomData,
		}
	}
}

impl<'vicinity, 'config, T: Config> BackendT for SubstrateStackState<'vicinity, 'config, T> {
	fn gas_price(&self) -> U256 {
		self.vicinity.gas_price
	}
	fn origin(&self) -> H160 {
		self.vicinity.origin
	}

	fn block_hash(&self, number: U256) -> H256 {
		if number > U256::from(u32::max_value()) {
			H256::default()
		} else {
			let number = T::BlockNumber::from(number.as_u32());
			H256::from_slice(<frame_system::Pallet<T>>::block_hash(number).as_ref())
		}
	}

	fn block_number(&self) -> U256 {
		let number: u128 = <frame_system::Pallet<T>>::block_number().unique_saturated_into();
		U256::from(number)
	}

	fn block_coinbase(&self) -> H160 {
		H160::default()
	}

	fn block_timestamp(&self) -> U256 {
		let now: u128 = <pallet_timestamp::Pallet<T>>::get().unique_saturated_into();
		U256::from(now / 1000)
	}

	fn block_difficulty(&self) -> U256 {
		U256::zero()
	}

	fn block_gas_limit(&self) -> U256 {
		U256::zero()
	}

	fn chain_id(&self) -> U256 {
		U256::from(T::ChainId::get())
	}

	fn exists(&self, _address: H160) -> bool {
		true
	}

	fn basic(&self, address: H160) -> evm::backend::Basic {
		let account = T::RingAccountBasic::account_basic(&address);

		evm::backend::Basic {
			balance: account.balance,
			nonce: account.nonce,
		}
	}

	fn code(&self, address: H160) -> Vec<u8> {
		AccountCodes::<T>::get(&address)
	}

	fn storage(&self, address: H160, index: H256) -> H256 {
		AccountStorages::<T>::get(address, index)
	}

	fn original_storage(&self, _address: H160, _index: H256) -> Option<H256> {
		None
	}
}

impl<'vicinity, 'config, T: Config> StackStateT<'config>
	for SubstrateStackState<'vicinity, 'config, T>
{
	fn metadata(&self) -> &StackSubstateMetadata<'config> {
		self.substate.metadata()
	}

	fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
		self.substate.metadata_mut()
	}

	fn enter(&mut self, gas_limit: u64, is_static: bool) {
		self.substate.enter(gas_limit, is_static)
	}

	fn exit_commit(&mut self) -> Result<(), ExitError> {
		self.substate.exit_commit()
	}

	fn exit_revert(&mut self) -> Result<(), ExitError> {
		self.substate.exit_revert()
	}

	fn exit_discard(&mut self) -> Result<(), ExitError> {
		self.substate.exit_discard()
	}

	fn is_empty(&self, address: H160) -> bool {
		Pallet::<T>::is_account_empty(&address)
	}

	fn deleted(&self, address: H160) -> bool {
		self.substate.deleted(address)
	}

	fn inc_nonce(&mut self, address: H160) {
		let account_id = T::AddressMapping::into_account_id(address);
		<frame_system::Pallet<T>>::inc_account_nonce(&account_id);
	}

	fn set_storage(&mut self, address: H160, index: H256, value: H256) {
		if value == H256::default() {
			log::debug!(
				target: "evm",
				"Removing storage for {:?} [index: {:?}]",
				address,
				index,
			);
			AccountStorages::<T>::remove(address, index);
		} else {
			log::debug!(
				target: "evm",
				"Updating storage for {:?} [index: {:?}, value: {:?}]",
				address,
				index,
				value,
			);
			AccountStorages::<T>::insert(address, index, value);
		}
	}

	fn reset_storage(&mut self, address: H160) {
		AccountStorages::<T>::remove_prefix(address);
	}

	fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
		self.substate.log(address, topics, data)
	}

	fn set_deleted(&mut self, address: H160) {
		self.substate.set_deleted(address)
	}

	fn set_code(&mut self, address: H160, code: Vec<u8>) {
		log::debug!(
			target: "evm",
			"Inserting code ({} bytes) at {:?}",
			code.len(),
			address
		);
		Pallet::<T>::create_account(address, code);
	}

	fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
		// Use Ring transfer
		T::RingAccountBasic::transfer(&transfer.source, &transfer.target, transfer.value)?;

		Ok(())
	}

	fn reset_balance(&mut self, _address: H160) {
		// Do nothing on reset balance in Substrate.
		//
		// This function exists in EVM because a design issue
		// (arguably a bug) in SELFDESTRUCT that can cause total
		// issurance to be reduced. We do not need to replicate this.
	}

	fn touch(&mut self, _address: H160) {
		// Do nothing on touch in Substrate.
		//
		// EVM pallet considers all accounts to exist, and distinguish
		// only empty and non-empty accounts. This avoids many of the
		// subtle issues in EIP-161.
	}
}
