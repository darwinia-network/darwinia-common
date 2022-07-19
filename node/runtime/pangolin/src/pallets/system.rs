// --- core ---
use core::{fmt::Debug, marker::PhantomData};
// --- crates.io ---
use codec::Codec;
use scale_info::StaticTypeInfo;
// --- paritytech ---
use frame_support::{traits::Contains, weights::constants::RocksDbWeight};
use frame_system::Config;
use sp_runtime::{traits::LookupError, MultiAddress};
use sp_version::RuntimeVersion;
// --- darwinia-network ---
use crate::*;
use darwinia_support::evm::{ConcatConverter, DeriveSubstrateAddress};
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

pub struct DarwiniaAccountLookup<AccountId>(PhantomData<AccountId>);
impl<AccountId> StaticLookup for DarwiniaAccountLookup<AccountId>
where
	AccountId: Clone + Debug + From<[u8; 32]> + PartialEq + Codec,
	MultiAddress<AccountId, ()>: Codec + StaticTypeInfo,
{
	type Source = MultiAddress<AccountId, ()>;
	type Target = AccountId;

	fn lookup(x: Self::Source) -> Result<Self::Target, LookupError> {
		match x {
			MultiAddress::Id(i) => Ok(i),
			MultiAddress::Address20(address) =>
				Ok(ConcatConverter::derive_substrate_address(&H160(address))),
			_ => Err(LookupError),
		}
	}

	fn unlookup(x: Self::Target) -> Self::Source {
		MultiAddress::Id(x)
	}
}

frame_support::parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 42;
}

impl Config for Runtime {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = BaseFilter;
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
	type Lookup = DarwiniaAccountLookup<AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	type Version = Version;
}
