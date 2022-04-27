// --- paritytech ---
use frame_support::{traits::Contains, weights::constants::RocksDbWeight};
use frame_system::Config;
use sp_runtime::traits::AccountIdLookup;
use sp_version::RuntimeVersion;
// --- darwinia-network ---
use crate::*;
use module_transaction_pause::PausedTransactionFilter;

pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
	fn contains(call: &Call) -> bool {
		let is_paused = PausedTransactionFilter::<Runtime>::contains(call);

		if is_paused {
			return false;
		}

		true
	}
}

frame_support::parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 18;
}

impl Config for Runtime {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = BaseFilter;
	type BlockHashCount = BlockHashCountForPangoro;
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
	type SystemWeightInfo = ();
	type Version = Version;
}
