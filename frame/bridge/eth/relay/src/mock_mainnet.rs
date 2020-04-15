//! Mock file for eth-relay.

// --- third-party ---
use hex::FromHex;
use serde::{Deserialize, Deserializer};
// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
// --- darwinia ---
use crate::*;

type AccountId = u64;
type BlockNumber = u64;

#[derive(Debug)]
pub struct Hex(pub Vec<u8>);

pub struct H128T(pub H128);

pub struct H256T(pub H256);

impl From<&Vec<u8>> for H128T {
	fn from(item: &Vec<u8>) -> Self {
		let mut data = [0u8; 16];
		for i in 0..item.len() {
			data[16 - 1 - i] = item[item.len() - 1 - i];
		}
		H128T(data.into())
	}
}

impl From<&Vec<u8>> for H256T {
	fn from(item: &Vec<u8>) -> Self {
		let mut data = [0u8; 32];
		for i in 0..item.len() {
			data[32 - 1 - i] = item[item.len() - 1 - i];
		}
		H256T(data.into())
	}
}

impl<'de> Deserialize<'de> for Hex {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		let mut s = <String as Deserialize>::deserialize(deserializer)?;
		if s.starts_with("0x") {
			s = s[2..].to_string();
		}
		if s.len() % 2 == 1 {
			s.insert_str(0, "0");
		}
		Ok(Hex(Vec::from_hex(&s).map_err(|err| {
			serde::de::Error::custom(err.to_string())
		})?))
	}
}

#[derive(Debug, Deserialize)]
struct RootsCollectionRaw {
	pub dag_merkle_roots: Vec<Hex>, // H128
}

#[derive(Debug, Deserialize)]
struct RootsCollection {
	pub dag_merkle_roots: Vec<H128>,
}

impl From<RootsCollectionRaw> for RootsCollection {
	fn from(item: RootsCollectionRaw) -> Self {
		Self {
			dag_merkle_roots: item
				.dag_merkle_roots
				.iter()
				.map(|e| H128T::from(&e.0).0)
				.collect(),
		}
	}
}

#[derive(Debug, Deserialize)]
struct BlockWithProofsRaw {
	pub proof_length: u64,
	pub header_rlp: Hex,
	pub merkle_root: Hex,        // H128
	pub elements: Vec<Hex>,      // H256
	pub merkle_proofs: Vec<Hex>, // H128
}

#[derive(Debug, Deserialize)]
pub struct BlockWithProofs {
	pub proof_length: u64,
	pub header_rlp: Hex,
	pub merkle_root: H128,
	pub elements: Vec<H256>,
	pub merkle_proofs: Vec<H128>,
}

impl From<BlockWithProofsRaw> for BlockWithProofs {
	fn from(item: BlockWithProofsRaw) -> Self {
		Self {
			proof_length: item.proof_length,
			header_rlp: item.header_rlp,
			merkle_root: H128T::from(&item.merkle_root.0).0,
			elements: item.elements.iter().map(|e| H256T::from(&e.0).0).collect(),
			merkle_proofs: item
				.merkle_proofs
				.iter()
				.map(|e| H128T::from(&e.0).0)
				.collect(),
		}
	}
}

impl BlockWithProofs {
	fn combine_dag_h256_to_h512(elements: Vec<H256>) -> Vec<H512> {
		elements
			.iter()
			.zip(elements.iter().skip(1))
			.enumerate()
			.filter(|(i, _)| i % 2 == 0)
			.map(|(_, (a, b))| {
				let mut buffer = [0u8; 64];
				buffer[..32].copy_from_slice(&(a.0));
				buffer[32..].copy_from_slice(&(b.0));
				H512(buffer.into())
			})
			.collect()
	}

	pub fn to_double_node_with_merkle_proof_vec(&self) -> Vec<DoubleNodeWithMerkleProof> {
		let h512s = Self::combine_dag_h256_to_h512(self.elements.clone());
		h512s
			.iter()
			.zip(h512s.iter().skip(1))
			.enumerate()
			.filter(|(i, _)| i % 2 == 0)
			.map(|(i, (a, b))| DoubleNodeWithMerkleProof {
				dag_nodes: vec![*a, *b],
				proof: self.merkle_proofs
					[i / 2 * self.proof_length as usize..(i / 2 + 1) * self.proof_length as usize]
					.to_vec(),
			})
			.collect()
	}
}

fn read_roots_collection() -> RootsCollection {
	read_roots_collection_raw().into()
}

fn read_roots_collection_raw() -> RootsCollectionRaw {
	serde_json::from_reader(
		std::fs::File::open(std::path::Path::new("./src/data/dag_merkle_roots.json")).unwrap(),
	)
	.unwrap()
}

pub fn read_block(filename: String) -> BlockWithProofs {
	read_block_raw(filename).into()
}

fn read_block_raw(filename: String) -> BlockWithProofsRaw {
	serde_json::from_reader(std::fs::File::open(std::path::Path::new(&filename)).unwrap()).unwrap()
}

pub type System = frame_system::Module<TestMainnet>;
pub type EthRelay = Module<TestMainnet>;

impl_outer_origin! {
	pub enum Origin for TestMainnet {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestMainnet;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for TestMainnet {
	type Origin = Origin;
	type Call = ();
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
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

parameter_types! {
	pub const EthNetwork: crate::EthNetworkType = crate::EthNetworkType::Mainnet;
}
impl Trait for TestMainnet {
	type Event = ();
	type EthNetwork = EthNetwork;
}

pub fn new_mainnet_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default()
		.build_storage::<TestMainnet>()
		.unwrap();

	GenesisConfig::<TestMainnet> {
		number_of_blocks_finality: 30,
		number_of_blocks_safe: 10,
		dag_merkle_roots: read_roots_collection().dag_merkle_roots,
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}
