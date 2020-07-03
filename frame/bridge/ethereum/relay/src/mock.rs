//! Mock file for ethereum-relay.
// --- std ---
use std::fs::File;
// --- crates ---
use serde::Deserialize;
// --- substrate ---
use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_io;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill, RuntimeDebug};
// --- darwinia ---
use crate::*;
use array_bytes::hex_bytes_unchecked;

type AccountId = u64;
type BlockNumber = u64;
type Balance = u128;

pub type RingInstance = darwinia_balances::Instance0;
pub type KtonInstance = darwinia_balances::Instance1;

pub type System = frame_system::Module<Test>;

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
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
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
}

impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = ();
}

impl_outer_origin! {
	pub enum Origin for Test  where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		frame_system::System,
	}
}

impl Trait for Test {
	type Event = ();
}

pub struct ExtBuilder {}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		storage.into()
	}
}

#[derive(Debug, Deserialize)]
pub struct EthHeaderJson {
	eth_header: String,
	ethash_proof: String,
	mmr_root: String,
	mmr_proof: Vec<String>,
}

pub struct EthHeaderRaw {
	pub eth_header: EthHeader,
	pub ethash_proof: DoubleNodeWithMerkleProof,
	pub mmr_root: H256,
	pub mmr_proof: Vec<H256>,
}

impl EthHeaderRaw {
	pub fn from_file(path: &str) -> Self {
		let eth_header_json: EthHeaderJson =
			serde_json::from_reader(File::open(path).unwrap()).unwrap();
		let eth_header = EthHeader::decode(&mut &*hex_bytes_unchecked(eth_header_json.eth_header))
			.unwrap_or_default();
		let ethash_proof = DoubleNodeWithMerkleProof::decode(&mut &*hex_bytes_unchecked(
			eth_header_json.ethash_proof,
		))
		.unwrap_or_default();
		let mmr_root =
			H256::decode(&mut &*hex_bytes_unchecked(eth_header_json.mmr_root)).unwrap_or_default();
		let mmr_proof: Vec<H256> = eth_header_json
			.mmr_proof
			.iter()
			.map(|h| H256::decode(&mut &*hex_bytes_unchecked(h)).unwrap_or_default())
			.collect();
		EthHeaderRaw {
			eth_header,
			ethash_proof,
			mmr_root,
			mmr_proof,
		}
	}
}
