// --- substrate ---
use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicsT, IdentityLookup},
	Perbill,
};
// --- darwinia ---
use crate::*;
use darwinia_eth_relay;

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		darwinia_eth_relay::EthRelay,
		darwinia_eth_offchain::EthOffchain,
	}
}

pub type EthOffchain = Module<Test>;
pub type EthRelay = darwinia_eth_relay::Module<Test>;
pub type OffchainError = Error<Test>;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction =
	frame_system::offchain::TransactionSubmitter<crypto::Public, Test, Extrinsic>;

impl frame_system::offchain::CreateTransaction<Test, Extrinsic> for Test {
	type Public = sp_core::sr25519::Public;
	type Signature = sp_core::sr25519::Signature;

	fn create_transaction<F: frame_system::offchain::Signer<Self::Public, Self::Signature>>(
		call: <Extrinsic as ExtrinsicsT>::Call,
		_public: Self::Public,
		_account: <Test as frame_system::Trait>::AccountId,
		nonce: <Test as frame_system::Trait>::Index,
	) -> Option<(
		<Extrinsic as ExtrinsicsT>::Call,
		<Extrinsic as ExtrinsicsT>::SignaturePayload,
	)> {
		Some((call, (nonce, ())))
	}
}

parameter_types! {
	pub const EthNetwork: darwinia_eth_relay::EthNetworkType = darwinia_eth_relay::EthNetworkType::Ropsten;
}

impl darwinia_eth_relay::Trait for Test {
	type Event = ();
	type EthNetwork = EthNetwork;
	type Call = Call;
}

//impl From<darwinia_eth_relay::Call<Test>> for Call<Test> {
//	fn from(_: darwinia_eth_relay::Call<Test>) -> Self {
//		unimplemented!()
//	}
//}

static mut SHADOW_SERVICE: Option<ShadowService> = None;

pub enum ShadowService {
	SCALE,
	JSON,
}

impl OffchainRequestTrait for OffchainRequest {
	fn send(&mut self) -> Option<Vec<u8>> {
		unsafe {
			match SHADOW_SERVICE {
				Some(ShadowService::SCALE) => Some(SUPPOSED_SHADOW_SCALE_RESPONSE.to_vec()),
				Some(ShadowService::JSON) => Some(SUPPOSED_SHADOW_JSON_RESPONSE.to_vec()),
				_ => None,
			}
		}
	}
}

pub(crate) fn set_shadow_service(s: Option<ShadowService>) {
	unsafe {
		SHADOW_SERVICE = s;
	}
}

parameter_types! {
	pub const FetchInterval: u64 = 3;
}
impl Trait for Test {
	type Event = ();
	type Call = Call;
	type SubmitSignedTransaction = SubmitTransaction;
	type FetchInterval = FetchInterval;
}

pub struct ExtBuilder {
	genesis_header: Option<(u64, Vec<u8>)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			genesis_header: None,
		}
	}
}

impl ExtBuilder {
	pub fn set_genesis_header(mut self) -> Self {
		let genesis_header = EthHeader::from_str_unchecked(SUPPOSED_ETHHEADER);
		self.genesis_header = Some((1, rlp::encode(&genesis_header)));
		self
	}
	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		darwinia_eth_relay::GenesisConfig::<Test> {
			genesis_header: self.genesis_header,
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.unwrap();
		storage.into()
	}
}
pub const SUPPOSED_SHADOW_SCALE_RESPONSE: &'static [u8] = br#"{"jsonrpc":"2.0","id":1,"result":{"eth_header":"0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa32442ba5500000000010000000000000005a56e2d52c817161883f50c441c3228cfe54d9f56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4211dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d4934764476574682f76312e302e302f6c696e75782f676f312e342e32d67e4d450343046425ae4271474353857ab860dbc0a1dde64b41b5cd3a532bf356e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4210000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008813000000000000000000000000000000000000000000000000000000000000000080ff030000000000000000000000000000000000000000000000000000000884a0969b900de27b6ac6a67742365dd65f55a0526c41fd18e1b16f1a1215c2e66f592488539bd4979fef1ec40188e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6","proof":"0x04000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"}}"#;
pub const SUPPOSED_ETHHEADER: &'static str = r#"
			{
				"difficulty": "0x3ff800000",
				"extraData": "0x476574682f76312e302e302f6c696e75782f676f312e342e32",
				"gasLimit": "0x1388",
				"gasUsed": "0x0",
				"hash": "0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6",
				"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
				"miner": "0x05a56e2d52c817161883f50c441c3228cfe54d9f",
				"mixHash": "0x969b900de27b6ac6a67742365dd65f55a0526c41fd18e1b16f1a1215c2e66f59",
				"nonce": "0x539bd4979fef1ec4",
				"number": "0x1",
				"parentHash": "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
				"receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x219",
				"stateRoot": "0xd67e4d450343046425ae4271474353857ab860dbc0a1dde64b41b5cd3a532bf3",
				"timestamp": "0x55ba4224",
				"totalDifficulty": "0x7ff800000",
				"transactions": [],
				"transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
				"uncles": []
			}
			"#;

pub const SUPPOSED_SHADOW_JSON_RESPONSE: &'static [u8] = br#"{"jsonrpc":"2.0","id":1,"result":{"eth_header":{"difficulty":"0x9fa52dbdada","extraData":"0xd783010302844765746887676f312e352e31856c696e7578","gasLimit":"0x2fefd8","gasUsed":"0x37881","hash":"0x26f10bfb3c09f1e1eadf856a8d75f5dbd2f88bd8eb4da8488f131835fa4a6ae3","logsBloom":"0x000000000000000000000000000000000000000000000000000000000000000000000000000000000c00000000000000000000020000000000000004000000000000000000000000000000020000000000000000000000000001000000000000004000000200000000000000000008020000020000000000000000001000000000000000000000004000040000000000000000000000000000000000000000000000000000000004001000000000000000000000000004080008000000000120000000000000000000000400000000000800000000000000000000000000200000000000001000000000000a0008000040000000000000000000000000000000","miner":"0x738db714c08b8a32a29e0e68af00215079aa9c5c","mixHash":"0xcb63ce95a3043c0f846ad6e1c3c25ec7a8cd8e09dccf02c7078669f2496f02c2","nonce":"0xfc2c4055195dac95","number":"0xeb770","parentHash":"0x28e9cc57847a0a1efd2920115ba94530ba7d29d7a7ffb15fc933302a97c73e49","receiptsRoot":"0xba124ff4744d7f59fd4f829be59f727fe17f468b34344759d4dd2ed10d6260d2","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","size":"0x792","stateRoot":"0x46f9f3d17b9bba9d551ab85a6aa6686a51590a184f5d42b98b6d8518303da470","timestamp":"0x56b66a81","totalDifficulty":"0x5d4fe4695aed3d42","transactions":[],"transactionsRoot":"0x5e7f4d048b09e832ccdb062c655def06f532ebdf02b3c0c423a65c6566220523","uncles":[]},"proof":[{"dag_nodes":["0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],"proof":["0x00000000000000000000000000000000","0x00000000000000000000000000000000"]}]}}"#;

pub const SUPPOSED_SHADOW_FAKE_RESPONSE: &'static [u8] =
	br#"{"jsonrpc":"2.0","id":1,"result":{"eth_header":{eth-content},"proof":[proof-content]}}"#;
pub const SUPPOSED_SHADOW_NON_ORDER_RESPONSE: &'static [u8] =
	br#"{"id":1,"result":{"proof":[proof-content],"eth_header":{eth-content}},"jsonrpc":"2.0"}"#;
