// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::traits::InstanceFilter;
use pallet_proxy::Config;
use sp_runtime::RuntimeDebug;
// --- darwinia-network ---
use crate::{weights::pallet_proxy::WeightInfo, *};

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
	EthereumBridge,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				Call::System(..) |
							Call::Babe(..) |
							Call::Timestamp(..) |
							// Specifically omitting the entire Balances pallet
							Call::Authorship(..) |
							Call::Staking(..) |
							Call::Session(..) |
							Call::Grandpa(..) |
							Call::ImOnline(..) |
							Call::Democracy(..) |
							Call::Council(..) |
							Call::TechnicalCommittee(..) |
							Call::PhragmenElection(..) |
							Call::TechnicalMembership(..) |
							Call::Treasury(..) |
							Call::KtonTreasury(..) |
							Call::Tips(..) |
							Call::Bounties(..) |
							Call::Sudo(..) |
							// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
							Call::Vesting(pallet_vesting::Call::vest{ .. }) |
							Call::Vesting(pallet_vesting::Call::vest_other{ .. }) |
							Call::Utility(..)|
							Call::Identity(..)|
							Call::Society(..)|
							// Specifically omitting Recovery `create_recovery`, `initiate_recovery`
							Call::Recovery(pallet_recovery::Call::as_recovered{ .. }) |
							Call::Recovery(pallet_recovery::Call::vouch_recovery{ .. }) |
							Call::Recovery(pallet_recovery::Call::claim_recovery{ .. }) |
							Call::Recovery(pallet_recovery::Call::close_recovery{ .. }) |
							Call::Recovery(pallet_recovery::Call::remove_recovery{ .. }) |
							Call::Recovery(pallet_recovery::Call::cancel_recovered{ .. }) |
							Call::Scheduler(..)|
							Call::Proxy(..)|
							Call::Multisig(..)|
							Call::EcdsaAuthority(..) /* Specifically omitting the entire
				                             * TronBacking pallet
				                             * Specifically omitting the entire EVM
				                             * pallet
				                             * Specifically omitting the entire
				                             * Ethereum pallet */
			),
			ProxyType::Governance => matches!(
				c,
				Call::Democracy(..)
					| Call::Council(..) | Call::TechnicalCommittee(..)
					| Call::PhragmenElection(..)
					| Call::Treasury(..) | Call::KtonTreasury(..)
					| Call::Tips(..) | Call::Bounties(..)
			),
			ProxyType::Staking => matches!(c, Call::Staking(..)),
			ProxyType::EthereumBridge => matches!(c, Call::EcdsaAuthority(..)),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

frame_support::parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = pangolin_deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = pangolin_deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = pangolin_deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = pangolin_deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

impl Config for Runtime {
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type Call = Call;
	type CallHasher = Hashing;
	type Currency = Ring;
	type Event = Event;
	type MaxPending = MaxPending;
	type MaxProxies = MaxProxies;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type ProxyType = ProxyType;
	type WeightInfo = WeightInfo<Self>;
}
