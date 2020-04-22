//! Mock file for eth-relay.

// --- std ---
use std::{cell::RefCell, fs::File};
// --- crates ---
use serde::Deserialize;
// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
// --- darwinia ---
use crate::*;
use darwinia_support::bytes_thing::{fixed_hex_bytes_unchecked, hex_bytes_unchecked};
use eth_primitives::receipt::LogEntry;

type AccountId = u64;
type BlockNumber = u64;

pub type System = frame_system::Module<Test>;
pub type EthRelay = Module<Test>;

impl_outer_origin! {
	pub enum Origin for Test {}
}

thread_local! {
	static ETH_NETWORK: RefCell<EthNetworkType> = RefCell::new(EthNetworkType::Ropsten);
}

#[derive(Debug)]
pub struct BlockWithProofs {
	pub proof_length: u64,
	pub merkle_root: H128,
	pub header_rlp: Vec<u8>,
	pub merkle_proofs: Vec<H128>,
	pub elements: Vec<H256>,
}
impl BlockWithProofs {
	pub fn from_file(path: &str) -> Self {
		#[derive(Deserialize)]
		struct RawBlockWithProofs {
			proof_length: u64,
			merkle_root: String,
			header_rlp: String,
			merkle_proofs: Vec<String>,
			elements: Vec<String>,
		}

		fn zero_padding(mut s: String, hex_len: usize) -> String {
			let len = hex_len << 1;
			if s.starts_with("0x") {
				let missing_zeros = len + 2 - s.len();
				if missing_zeros != 0 {
					for _ in 0..missing_zeros {
						s.insert(2, '0');
					}
				}
			} else {
				let missing_zeros = len - s.len();
				if missing_zeros != 0 {
					for _ in 0..missing_zeros {
						s.insert(0, '0');
					}
				}
			}

			s
		}

		let raw_block_with_proofs: RawBlockWithProofs =
			serde_json::from_reader(File::open(path).unwrap()).unwrap();

		BlockWithProofs {
			proof_length: raw_block_with_proofs.proof_length,
			merkle_root: fixed_hex_bytes_unchecked!(&raw_block_with_proofs.merkle_root, 16).into(),
			header_rlp: hex_bytes_unchecked(&raw_block_with_proofs.header_rlp),
			merkle_proofs: raw_block_with_proofs
				.merkle_proofs
				.iter()
				.cloned()
				.map(|raw_merkle_proof| {
					fixed_hex_bytes_unchecked!(&zero_padding(raw_merkle_proof, 16), 16).into()
				})
				.collect(),
			elements: raw_block_with_proofs
				.elements
				.iter()
				.cloned()
				.map(|raw_element| {
					fixed_hex_bytes_unchecked!(&zero_padding(raw_element, 32), 32).into()
				})
				.collect(),
		}
	}

	pub fn to_double_node_with_merkle_proof_vec(&self) -> Vec<DoubleNodeWithMerkleProof> {
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

		let h512s = combine_dag_h256_to_h512(self.elements.clone());
		h512s
			.iter()
			.zip(h512s.iter().skip(1))
			.enumerate()
			.filter(|(i, _)| i % 2 == 0)
			.map(|(i, (a, b))| DoubleNodeWithMerkleProof {
				dag_nodes: [*a, *b],
				proof: self.merkle_proofs
					[i / 2 * self.proof_length as usize..(i / 2 + 1) * self.proof_length as usize]
					.to_vec(),
			})
			.collect()
	}
}

pub struct EthNetwork;
impl Get<EthNetworkType> for EthNetwork {
	fn get() -> EthNetworkType {
		ETH_NETWORK.with(|v| v.borrow().to_owned())
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

impl Trait for Test {
	type Event = ();
	type EthNetwork = EthNetwork;
}

pub struct ExtBuilder {
	eth_network: EthNetworkType,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			eth_network: EthNetworkType::Ropsten,
		}
	}
}

impl ExtBuilder {
	pub fn eth_network(mut self, eth_network: EthNetworkType) -> Self {
		self.eth_network = eth_network;
		self
	}
	pub fn set_associated_consts(&self) {
		ETH_NETWORK.with(|v| v.replace(self.eth_network.clone()));
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_consts();

		let mut storage = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		GenesisConfig::<Test> {
			number_of_blocks_finality: 30,
			number_of_blocks_safe: 10,
			dag_merkle_roots: DagMerkleRoots::load_genesis(
				"../../../../bin/node-template/node/res/dag_merkle_roots.json",
				"DAG_MERKLE_ROOTS_PATH",
			),
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}

/// To help reward miners for when duplicate block solutions are found
/// because of the shorter block times of Ethereum (compared to other cryptocurrency).
/// An uncle is a smaller reward than a full block.
///
/// stackoverflow: https://ethereum.stackexchange.com/questions/34/what-is-an-uncle-ommer-block
///
/// returns: [origin, grandpa, uncle, parent, current]
pub fn mock_canonical_relationship() -> [EthHeader; 5] {
	let mut headers = HEADERS.split("@next");
	[
		EthHeader::from_str_unchecked(headers.next().unwrap()),
		EthHeader::from_str_unchecked(headers.next().unwrap()),
		EthHeader::from_str_unchecked(headers.next().unwrap()),
		EthHeader::from_str_unchecked(headers.next().unwrap()),
		EthHeader::from_str_unchecked(headers.next().unwrap()),
	]
}

/// mock canonical receipt
pub fn mock_canonical_receipt() -> EthReceiptProof {
	// fn mock_receipt_from_source(o: &mut Object) -> Option<EthReceiptProof> {
	// 	Some(EthReceiptProof {
	// 		index: o.get("index")?.as_str()?[2..].parse::<u64>().unwrap(),
	// 		proof: hex(&o.get("proof")?.as_str()?)?,
	// 		header_hash: H256::from(bytes!(&o.get("header_hash")?, 32)),
	// 	})
	// }

	let receipt: serde_json::Value = serde_json::from_str(RECEIPT).unwrap();
	EthReceiptProof {
		index: receipt["index"]
			.as_str()
			.unwrap()
			.trim_start_matches("0x")
			.parse()
			.unwrap(),
		proof: hex_bytes_unchecked(receipt["proof"].as_str().unwrap()),
		header_hash: fixed_hex_bytes_unchecked!(receipt["header_hash"].as_str().unwrap(), 32)
			.into(),
	}
}

/// mock log events
pub fn mock_receipt_logs() -> Vec<LogEntry> {
	let logs: serde_json::Value = serde_json::from_str(EVENT_LOGS).unwrap();
	logs["logs"]
		.as_array()
		.unwrap()
		.iter()
		.map(|log| LogEntry {
			address: fixed_hex_bytes_unchecked!(log["address"].as_str().unwrap(), 20).into(),
			topics: log["topics"]
				.as_array()
				.unwrap()
				.iter()
				.map(|topic| fixed_hex_bytes_unchecked!(topic.as_str().unwrap(), 32).into())
				.collect(),
			data: hex_bytes_unchecked(log["data"].as_str().unwrap()),
		})
		.collect()
}

pub const MAINNET_GENESIS_HEADER: &'static str = r#"
{
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
	"transactions": [omitted],
	"transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
	"uncles": [omitted]
}
"#;

/// # Genealogical Tree
///
/// | pos     | height  | tx                                                                 |
/// |---------|---------|--------------------------------------------------------------------|
/// | origin  | 7575765 |                                                                    |
/// | grandpa | 7575766 | 0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889 |
/// | uncle   | 7575766 |                                                                    |
/// | parent  | 7575767 |                                                                    |
/// | current | 7575768 | 0xfc836bf547f1e035e837bf0a8d26e432aa26da9659db5bf6ba69b0341d818778 |
pub const HEADERS: &'static str = r#"
{
	"difficulty": "0x234ac172",
	"extraData": "0xde830207028f5061726974792d457468657265756d86312e34312e30826c69",
	"gasLimit": "0x7a121d",
	"gasUsed": "0x1b8855",
	"hash": "0x253c1f8ed3051930949251bcf786d4ecfe379c001202d07aeb8a68ba15588f1d",
	"logsBloom": "0x0006000000400004000000000800000ac000000200208000040000100084410200017001004000090100600000002800000041020002400000000000200000c81080602800004000000200080020000828200000110320001000000008008420000000400200a0008c0000380410084040200201040001000014045011001010000408000000a80000000010020002000000049000000000800a5000080000000000008010000000820041040014000100000004000000000040000002000000000000221000404028000002048200080000000000000000000001000108204002000200000012000000808000008200a0020000001000800000000080000000",
	"miner": "0x05fc5a079e0583b8a07526023a16e2022c4c6296",
	"mixHash": "0xe582018f215ce844c7e0b9bd10ee8ab89cad57dc01f3aec080bff11134cc5573",
	"nonce": "0xe55fdb2d73c14cee",
	"number": "0x7398d5",
	"parentHash": "0xccd3a54b1bb11a8fa7eb82c6885c3bdcc9884cb0229cb9a70683d58bfe78e80c",
	"receiptsRoot": "0x6c57de9ea8a275b131b344d60bbdef1ea1465753cba5924be631116fc9994d8b",
	"sha3Uncles": "0xec428257d3daf5aa3a394665c7ab79e14a51116178653038fd2d5c23bb011833",
	"size": "0x1b0b",
	"stateRoot": "0xbd3b97632b55686763748c69dec192fa2b5067c92cc0e3b5e19afad6bf43ed04",
	"timestamp": "0x5e78f257",
	"totalDifficulty": "0x6b2dd4a2c4f47d",
	"transactions": [omitted],
	"transactionsRoot": "0x1d096373d65213a55a03f1edd066091ef245054ddbd827a4679f19983b2d8ae6",
	"uncles": [omitted]
}
@next
{
	"difficulty": "592679970",
	"extraData": "0xde830207028f5061726974792d457468657265756d86312e34312e30826c69",
	"gasLimit": 8000029,
	"gasUsed": 1673785,
	"hash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
	"logsBloom": "0x00000000000000000000002040000000c000000000202010080002100084400000401001000000020000400040002000000000000002c040000809000000004010800020000040000000020a1002000040000000800100000000000000000000000000000200a0008000000800000804482000010400000000000010100010000000080000000001000000000000000000000480004004000008000000200000000000002200000000000000000000000000000000000200000000000000000002000002100000012000200000040008080001000000000800200000000060000108000080001000000002000000000000000000001000020010000000000000",
	"miner": "0x05FC5a079e0583B8A07526023A16E2022c4C6296",
	"mixHash": "0xd1716ffbdb6b77a6a1a76bca2e4b5c6c5079689c4402cc0df583c08737a3957e",
	"nonce": "0x1a711f7039202c30",
	"number": 7575766,
	"parentHash": "0x253c1f8ed3051930949251bcf786d4ecfe379c001202d07aeb8a68ba15588f1d",
	"receiptsRoot": "0xa4d62fe6b519fe3e2fbeb4862bb7151340c638f59dc5865974a4064d97d30b36",
	"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
	"size": 5099,
	"stateRoot": "0x9a1a18d30c00565b3f3c4a13829c9ca4adabb3c0080bf707cf738e35c46cc4db",
	"timestamp": 1584984668,
	"totalDifficulty": "30168214387853471",
	"transactions": [omitted],
	"transactionsRoot": "0x770bedcea35a614a3bc56e5047c5731215a989380cef38bfc017ec8459d9af72",
	"uncles": [omitted]
}
@next
{
	"difficulty": "592679970",
	"extraData": "0xde830207028f5061726974792d457468657265756d86312e34312e30826c69",
	"gasLimit": 8000029,
	"gasUsed": 1673785,
	"hash": "0x44a9de57eb3fde9e2f11491bde0f6292ca533cd015d72a6ae877890c63c3c62f",
	"logsBloom": "0x00000000000000000000002040000000c000000000202010080002100084400000401001000000020000400040002000000000000002c040000809000000004010800020000040000000020a1002000040000000800100000000000000000000000000000200a0008000000800000804482000010400000000000010100010000000080000000001000000000000000000000480004004000008000000200000000000002200000000000000000000000000000000000200000000000000000002000002100000012000200000040008080001000000000800200000000060000108000080001000000002000000000000000000001000020010000000000000",
	"miner": "0x05FC5a079e0583B8A07526023A16E2022c4C6296",
	"mixHash": "0xff7a54c198bd9dd3fc363020e550dbfc633fccdad934eff4ff84850b3e39ed48",
	"nonce": "0x1a711f703aa5d304",
	"number": 7575766,
	"parentHash": "0x253c1f8ed3051930949251bcf786d4ecfe379c001202d07aeb8a68ba15588f1d",
	"receiptsRoot": "0xa4d62fe6b519fe3e2fbeb4862bb7151340c638f59dc5865974a4064d97d30b36",
	"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
	"size": 549,
	"stateRoot": "0x9a1a18d30c00565b3f3c4a13829c9ca4adabb3c0080bf707cf738e35c46cc4db",
	"timestamp": 1584984668,
	"totalDifficulty": "30168214387853471",
	"transactions": [omitted],
	"transactionsRoot": "0x770bedcea35a614a3bc56e5047c5731215a989380cef38bfc017ec8459d9af72",
	"uncles": [omitted]
}
@next
{
	"difficulty": "592679970",
	"extraData": "0xde8302050d8f5061726974792d457468657265756d86312e33382e30826c69",
	"gasLimit": 8000029,
	"gasUsed": 3006750,
	"hash": "0x05f153ee818d06794a0fb0443bfc428e3cf68a96a30c24e88325f2aa1659294d",
	"logsBloom": "0x800400000200040000001040000004008000002200000000000000000584010000030009000000010800200000000000004008008002000000000000060000f0508020080200000000000908000000080800000000000400908000000000801000000000020060000c00002400000800402002000401000000142010510010000014080000008000000000000200020004000480010000008000000108010000000000000008100000004560001400010004000c0000000008000000000000000008010610000004a0a0010010000008000000000000000000000020090820400000000004000010000080000000000140000001000000002000000880000000",
	"miner": "0x635B4764D1939DfAcD3a8014726159abC277BecC",
	"mixHash": "0x8f99e71c2111c2cd241c29d2bddfbb5899a09fb9a6e453fe0aee22f939eb6b95",
	"nonce": "0x6ba0ad07a53bf14d",
	"number": 7575767,
	"parentHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
	"receiptsRoot": "0x371ffbfd85a76511d9d6b1162f93cfe70e9d31aaa2773e035d84b94a2c6d4699",
	"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
	"size": 9558,
	"stateRoot": "0x5df70e19f157bc2804aff4e17f752e3b57a77cb8ffa967e50ee9709a1f73a459",
	"timestamp": 1584984682,
	"totalDifficulty": "30168214980533441",
	"transactions": [omitted],
	"transactionsRoot": "0x6d42a354ae9d5d9d103f640042889f06bc98217bf0d635fb39dd531120efe836",
	"uncles": [omitted]
}
@next
{
	"difficulty": "592969364",
	"extraData": "0xde830207028f5061726974792d457468657265756d86312e34312e30826c69",
	"gasLimit": 8000029,
	"gasUsed": 1933244,
	"hash": "0xda6db2ef04f5f7ba97b16de78b3706b174926168e3e6b83d4923b19188a07e4f",
	"logsBloom": "0x080040000000000000000020080000004000000000202010080000100004400000400001000000020000400040000000000000000000804000000900000000000000000000004000000003081000080000000000000000000000000000000000000000000200a0008010008800000804082000010000800000000010004000000080080000000001000000000000000000000000004004000008000040200000000000000200000000000000000000000000000000000200000000000000000002000002080000010002000000040008000001000000010800200000000060000000000000001080000000100000000000000000001000020000808000000000",
	"miner": "0x05FC5a079e0583B8A07526023A16E2022c4C6296",
	"mixHash": "0x9a39a843c6dd051877c97c90fada4f50976bbd33adb6cc341aadb0131e418731",
	"nonce": "0x6a8de7b9f4efeb04",
	"number": 7575768,
	"parentHash": "0x05f153ee818d06794a0fb0443bfc428e3cf68a96a30c24e88325f2aa1659294d",
	"receiptsRoot": "0xa190cfab34c8a3519edca74aeb813751e4af6863c8bdb4afb8c692872fa6c031",
	"sha3Uncles": "0x805fc9304943c784d4b9ed2c20383bc334a7ea4c8d046031d1fb986b224a7e2f",
	"size": 5809,
	"stateRoot": "0x7a9892ed9ab322eb44e564184377610645715f576ecc22706e7265e28ca870c8",
	"timestamp": 1584984686,
	"totalDifficulty": "30168215573502805",
	"transactions": [omitted],
	"transactionsRoot": "0xba329c4119380c14d9fbc91dcbef180d5b184514760e4fb70a35ba720958029c",
	"uncles": [omitted]
}
"#;

/// common receipt
pub const RECEIPT: &'static str = r#"{
	"index": "0x3",
	"proof": "0xf90639f90636b853f851a0e2dde80962d77a47a1eab063cc8a378f739d23df6e29593b9a213416656c68c180808080808080a02afdbac54a1d63d1329fd2ce2cac4041e26a23aee0509d76b23b0dbedf44a5f38080808080808080b8d3f8d180a068902a3cc3e2192a6ccc54eef093708d56e13136dd90b08efb0f1dc3305df2d7a085a0404ca58ca11c6e2d7dc0bdc7eacbfa284097940cd23f1a4200476c4ecd0fa0d569ad3746049c498094c5e3e1a28a498e5525732684eba723ff6539d3cac009a0385871245210d867025a2f2ea5143b884b67c3a2ba2f76561b982de230492012a086c057ea140bacf807b4c0d93efabf320ebe30a43d997259241832aea2ac26c6a02e10c5544b0b294153fafe7b444127463c08e9de37b60fc300070287d9b8d5b080808080808080808080b90509f9050620b90502f904ff018314ec7fb9010000000000000000000000002000000000400000000000201008000000000000000040000000000002000000000000000000000000000080400000080000000000000000000000000000000208100000000000000000000000000000000000000000000000020000000000000000000804080000000000000000000010000000000000000000000001000000000000000000000000004000000000000000200000000000000000000000000000000000000000000000000200000000000000000000000002000000000000000000040000000001000000000800200000000060000000000000000000000000000000000000000000000000020000000000000000f903f4f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa00000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23a0000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6ea00000000000000000000000000000000000000000000000000de0b6b3a7640000f87a94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a0000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6ea00000000000000000000000000000000000000000000000000de0b6b3a7640000f89b94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6ea00000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000de0b6b3a7640000f9011c94dbc888d701167cbfb86486c516aafbefc3a4de6ef863a038045eaef0a21b74ff176350f18df02d9041a25d6694b5f63e9474b7b6cd6b94a0000000000000000000000000b52fbe2b925ab79a821b261c82c5ba0814aaa5e0a00000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23b8a00000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000212ad7b504ddbe25a05647312daa8d0bbbafba360686241b7e193ca90f9b01f95faa00000000000000000000000000000000000000000000000000000000000000f9011c94b52fbe2b925ab79a821b261c82c5ba0814aaa5e0f863a09bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1a00000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23a0000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6eb8a00000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000212ad7b504ddbe25a05647312daa8d0bbbafba360686241b7e193ca90f9b01f95faa00000000000000000000000000000000000000000000000000000000000000",
	"header_hash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768"
}"#;

/// event logs
pub const EVENT_LOGS: &'static str = r#"
{
	"logs": [
		{
			"address": "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
			"blockHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
			"blockNumber": 7575766,
			"data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000",
			"logIndex": 8,
			"removed": false,
			"topics": [
				"0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
				"0x0000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23",
				"0x000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6e"
			],
			"transactionHash": "0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889",
			"transactionIndex": 3,
			"id": "log_d6f43e1c"
		},
		{
			"address": "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
			"blockHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
			"blockNumber": 7575766,
			"data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000",
			"logIndex": 9,
			"removed": false,
			"topics": [
				"0xcc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5",
				"0x000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6e"
			],
			"transactionHash": "0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889",
			"transactionIndex": 3,
			"id": "log_a2379338"
		},
		{
			"address": "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
			"blockHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
			"blockNumber": 7575766,
			"data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000",
			"logIndex": 10,
			"removed": false,
			"topics": [
				"0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
				"0x000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6e",
				"0x0000000000000000000000000000000000000000000000000000000000000000"
			],
			"transactionHash": "0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889",
			"transactionIndex": 3,
			"id": "log_acf4e896"
		},
		{
			"address": "0xdBC888D701167Cbfb86486C516AafBeFC3A4de6e",
			"blockHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
			"blockNumber": 7575766,
			"data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000212ad7b504ddbe25a05647312daa8d0bbbafba360686241b7e193ca90f9b01f95faa00000000000000000000000000000000000000000000000000000000000000",
			"logIndex": 11,
			"removed": false,
			"topics": [
				"0x38045eaef0a21b74ff176350f18df02d9041a25d6694b5f63e9474b7b6cd6b94",
				"0x000000000000000000000000b52fbe2b925ab79a821b261c82c5ba0814aaa5e0",
				"0x0000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23"
			],
			"transactionHash": "0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889",
			"transactionIndex": 3,
			"id": "log_44dceebb"
		},
		{
			"address": "0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
			"blockHash": "0xb49cc783d8da7896e5dc50fc2a927b80dcef6ebb36738a3f0aeaf3b4f970e768",
			"blockNumber": 7575766,
			"data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000212ad7b504ddbe25a05647312daa8d0bbbafba360686241b7e193ca90f9b01f95faa00000000000000000000000000000000000000000000000000000000000000",
			"logIndex": 12,
			"removed": false,
			"topics": [
				"0x9bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1",
				"0x0000000000000000000000002c7536e3605d9c16a7a3d7b1898e529396a65c23",
				"0x000000000000000000000000dbc888d701167cbfb86486c516aafbefc3a4de6e"
			],
			"transactionHash": "0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889",
			"transactionIndex": 3,
			"id": "log_840077b9"
		}
	]
}
"#;
