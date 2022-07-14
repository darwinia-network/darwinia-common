// --- paritytech ---
use frame_support::{traits::Everything, weights::constants::RocksDbWeight};
use frame_system::Config;
use sp_runtime::traits::AccountIdLookup;
use sp_version::RuntimeVersion;
// --- darwinia-network ---
use crate::*;
use crate::weights::frame_system::WeightInfo;

frame_support::parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 42;
}

impl Config for Runtime {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = BlockHashCountForPangolin;
	type BlockLength = RuntimeBlockLength;
	type BlockNumber = BlockNumber;
	type BlockWeights = RuntimeBlockWeights;
	type Call = Call;
	type DbWeight = RocksDbWeight;
	type Event = Event;
	type Hash = Hash;
	type Hashing = Hashing;
	type Header = Header;
	type Index = Nonce;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = WeightInfo<Runtime>;
	type Version = Version;
}
