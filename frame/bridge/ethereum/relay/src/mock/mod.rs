//! Mock file for ethereum-relay.

// --- crates ---
use codec::Error;
// --- substrate ---
use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill, RuntimeDebug};
// --- darwinia ---
use crate::*;
use array_bytes::hex_bytes_unchecked;

// Static codec header source
mod test_data {
	mod header_thing_0;
	mod header_thing_1;
	mod header_thing_2;
	mod header_thing_3;

	pub use self::{
		header_thing_0::HEADER_THING_CODEC_0, header_thing_1::HEADER_THING_CODEC_1,
		header_thing_2::HEADER_THING_CODEC_2, header_thing_3::HEADER_THING_CODEC_3,
	};
}

// Types
type AccountId = u64;
type BlockNumber = u64;
type Balance = u128;

pub type RingInstance = darwinia_balances::Instance0;
pub type KtonInstance = darwinia_balances::Instance1;

pub type System = frame_system::Module<Test>;
pub type EthereumRelay = Module<Test>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		frame_system::System,
	}
}

darwinia_support::impl_account_data! {
	pub struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const EthereumRelayModuleId: ModuleId = ModuleId(*b"da/ethrl");
}
impl Trait for Test {
	type ModuleId = EthereumRelayModuleId;
	type Event = ();
	type Currency = Ring;
	type RelayerGame = UnusedRelayerGame;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = ();
	type WeightInfo = ();
}

pub struct ExtBuilder {}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
	}
}
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		GenesisConfig::<Test> {
			dags_merkle_roots_loader: DagsMerkleRootsLoader::from_file(
				"../../../../bin/node-template/node/res/dags_merkle_roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.unwrap();
		storage.into()
	}
}

pub struct UnusedRelayerGame;
impl RelayerGameProtocol for UnusedRelayerGame {
	type Relayer = u64;
	type TcBlockNumber = u64;

	fn submit_proposal(_: Self::Relayer, _: Vec<RawHeaderThing>) -> DispatchResult {
		unimplemented!()
	}
	fn approve_pending_header(_: Self::TcBlockNumber) -> DispatchResult {
		unimplemented!()
	}
	fn reject_pending_header(_: Self::TcBlockNumber) -> DispatchResult {
		unimplemented!()
	}
}

pub fn header_things() -> Result<[EthHeaderThing; 4], Error> {
	Ok([
		EthHeaderThing::decode(&mut &*hex_bytes_unchecked(test_data::HEADER_THING_CODEC_0))?,
		EthHeaderThing::decode(&mut &*hex_bytes_unchecked(test_data::HEADER_THING_CODEC_1))?,
		EthHeaderThing::decode(&mut &*hex_bytes_unchecked(test_data::HEADER_THING_CODEC_2))?,
		EthHeaderThing::decode(&mut &*hex_bytes_unchecked(test_data::HEADER_THING_CODEC_3))?,
	])
}
