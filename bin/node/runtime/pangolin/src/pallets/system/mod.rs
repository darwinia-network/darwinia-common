pub use pangolin_runtime_system_params::*;

// --- substrate ---
use frame_support::weights::constants::RocksDbWeight;
use frame_system::{weights::SubstrateWeight, Config};
use sp_runtime::traits::{AccountIdLookup, BlakeTwo256};
use sp_version::RuntimeVersion;
// --- darwinia ---
use crate::*;

frame_support::parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 18;
}

impl Config for Runtime {
	type BaseCallFilter = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type Origin = Origin;
	type Call = Call;
	type Index = Nonce;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = SubstrateWeight<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}
