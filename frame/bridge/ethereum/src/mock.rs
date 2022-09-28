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

//! Mock file for ethereum-relay.

// --- crates.io ---
use codec::MaxEncodedLen;
// --- paritytech ---
use frame_support::traits::{ConstU32, Everything, OnInitialize};
use frame_system::{mocking::*, EnsureRoot};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, RuntimeDebug};
// --- darwinia-network ---
use crate::{self as darwinia_bridge_ethereum, *};

pub type Block = MockBlock<Test>;
pub type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

darwinia_support::impl_test_account_data! {}

impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = ();
	type Hash = H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
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

impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

pub struct UnusedTechnicalMembership;
impl SortedMembers<AccountId> for UnusedTechnicalMembership {
	fn sorted_members() -> Vec<AccountId> {
		vec![1, 2, 3]
	}
}
frame_support::parameter_types! {
	pub const EthereumRelayPalletId: PalletId = PalletId(*b"da/ethrl");
	pub const EthereumRelayBridgeNetwork: EthereumNetwork = EthereumNetwork::Mainnet;
	pub static BestConfirmedBlockNumber: EthereumBlockNumber = 0;
	pub static ConfirmPeriod: BlockNumber = 0;
}
impl Config for Test {
	type ApproveOrigin = EnsureRoot<AccountId>;
	type ApproveThreshold = ();
	type BridgedNetwork = EthereumRelayBridgeNetwork;
	type Call = Call;
	type ConfirmPeriod = ConfirmPeriod;
	type Currency = Ring;
	type Event = ();
	type PalletId = EthereumRelayPalletId;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RejectThreshold = ();
	type RelayerGame = UnusedRelayerGame;
	type TechnicalMembership = UnusedTechnicalMembership;
	type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage},
		EthereumRelay: darwinia_bridge_ethereum::{Pallet, Call, Storage, Config<T>},
	}
}

pub struct ExtBuilder {
	best_confirmed_block_number: EthereumBlockNumber,
	confirm_period: BlockNumber,
}
impl ExtBuilder {
	pub fn best_confirmed_block_number(
		mut self,
		best_confirmed_block_number: EthereumBlockNumber,
	) -> Self {
		self.best_confirmed_block_number = best_confirmed_block_number;

		self
	}

	pub fn confirm_period(mut self, confirm_period: BlockNumber) -> Self {
		self.confirm_period = confirm_period;

		self
	}

	pub fn set_associated_constants(&self) {
		BEST_CONFIRMED_BLOCK_NUMBER.with(|v| v.replace(self.best_confirmed_block_number));
		CONFIRM_PERIOD.with(|v| v.replace(self.confirm_period));
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_constants();

		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		darwinia_bridge_ethereum::GenesisConfig::<Test> {
			genesis_header_parcel: r#"{
				"header": {
					"difficulty": "0x400000000",
					"extraData": "0x11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
					"gasLimit": "0x1388",
					"gasUsed": "0x0",
					"hash": "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
					"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
					"miner": "0x0000000000000000000000000000000000000000",
					"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"nonce": "0x0000000000000042",
					"number": "0x0",
					"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
					"receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"size": "0x21c",
					"stateRoot": "0xd7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544",
					"timestamp": "0x0",
					"totalDifficulty": "0x400000000",
					"transactions": [],
					"transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
					"uncles": []
				},
				"parent_mmr_root": "0x0000000000000000000000000000000000000000000000000000000000000000"
			}"#.into(),
			dags_merkle_roots_loader: DagsMerkleRootsLoader::from_file(
				"../../../../bin/res/ethereum/dags-merkle-roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			best_confirmed_block_number: BEST_CONFIRMED_BLOCK_NUMBER.with(|v| *v.borrow()),
			confirm_period: CONFIRM_PERIOD.with(|v| *v.borrow()),
		}
	}
}

// TODO https://github.com/darwinia-network/darwinia-common/issues/754
pub struct UnusedRelayerGame;
impl RelayerGameProtocol for UnusedRelayerGame {
	type RelayHeaderId = EthereumBlockNumber;
	type RelayHeaderParcel = EthereumRelayHeaderParcel;
	type RelayProofs = EthereumRelayProofs;
	type Relayer = AccountId;

	fn get_affirmed_relay_header_parcels(
		_: &RelayAffirmationId<Self::RelayHeaderId>,
	) -> Option<Vec<Self::RelayHeaderParcel>> {
		// This is mocked for test `pre_verify_should_work`
		Some(Default::default())
	}

	fn best_confirmed_header_id_of(_: &Self::RelayHeaderId) -> Self::RelayHeaderId {
		BEST_CONFIRMED_BLOCK_NUMBER.with(|v| *v.borrow())
	}

	fn affirm(
		_: &Self::Relayer,
		_: Self::RelayHeaderParcel,
		_: Option<Self::RelayProofs>,
	) -> Result<Self::RelayHeaderId, DispatchError> {
		unimplemented!()
	}

	fn dispute_and_affirm(
		_: &Self::Relayer,
		_: Self::RelayHeaderParcel,
		_: Option<Self::RelayProofs>,
	) -> Result<(Self::RelayHeaderId, u32), DispatchError> {
		unimplemented!()
	}

	fn complete_relay_proofs(
		_: RelayAffirmationId<Self::RelayHeaderId>,
		_: Vec<Self::RelayProofs>,
	) -> DispatchResult {
		unimplemented!()
	}

	fn extend_affirmation(
		_: &Self::Relayer,
		_: RelayAffirmationId<Self::RelayHeaderId>,
		_: Vec<Self::RelayHeaderParcel>,
		_: Option<Vec<Self::RelayProofs>>,
	) -> Result<(Self::RelayHeaderId, u32, u32), DispatchError> {
		unimplemented!()
	}
}

pub fn run_to_block(n: BlockNumber) {
	// EthereumRelay::on_finalize(System::block_number());

	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		EthereumRelay::on_initialize(b);

		// if b != n {
		// 	EthereumRelay::on_finalize(System::block_number());
		// }
	}
}
