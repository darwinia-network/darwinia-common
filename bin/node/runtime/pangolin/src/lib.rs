//! The Darwinia Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

pub mod constants {
	// --- darwinia ---
	use crate::*;

	pub const NANO: Balance = 1;
	pub const MICRO: Balance = 1_000 * NANO;
	pub const MILLI: Balance = 1_000 * MICRO;
	pub const COIN: Balance = 1_000 * MILLI;

	pub const CAP: Balance = 10_000_000_000 * COIN;
	pub const TOTAL_POWER: Power = 1_000_000_000;

	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
	pub const BLOCKS_PER_SESSION: BlockNumber = 10 * MINUTES;
	pub const SESSIONS_PER_ERA: SessionIndex = 6;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = 60 * MINUTES;
	pub const DAYS: BlockNumber = 24 * HOURS;

	// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 20 * COIN + (bytes as Balance) * 100 * MICRO
	}
}

pub mod impls {
	//! Some configurable implementations as associated type for the substrate runtime.

	pub mod relay {
		// --- darwinia ---
		use crate::*;
		use darwinia_relay_primitives::relayer_game::*;
		use ethereum_primitives::EthereumBlockNumber;

		pub struct EthereumRelayerGameAdjustor;
		impl AdjustableRelayerGame for EthereumRelayerGameAdjustor {
			type Moment = BlockNumber;
			type Balance = Balance;
			type RelayHeaderId = EthereumBlockNumber;

			fn max_active_games() -> u8 {
				32
			}

			fn affirm_time(round: u32) -> Self::Moment {
				match round {
					// 1.5 mins
					0 => 15,
					// 0.5 mins
					_ => 5,
				}
			}

			fn complete_proofs_time(round: u32) -> Self::Moment {
				match round {
					// 1.5 mins
					0 => 15,
					// 0.5 mins
					_ => 5,
				}
			}

			fn update_sample_points(sample_points: &mut Vec<Vec<Self::RelayHeaderId>>) {
				sample_points.push(vec![sample_points.last().unwrap().last().unwrap() - 1]);
			}

			fn estimate_stake(round: u32, affirmations_count: u32) -> Self::Balance {
				match round {
					0 => match affirmations_count {
						0 => 1000 * COIN,
						_ => 1500 * COIN,
					},
					_ => 100 * COIN,
				}
			}
		}
	}

	// --- crates ---
	use smallvec::smallvec;
	// --- substrate ---
	use frame_support::{
		traits::{Currency, Imbalance, OnUnbalanced},
		weights::{WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	};
	// --- darwinia ---
	use crate::*;

	darwinia_support::impl_account_data! {
		struct AccountData<Balance>
		for
			RingInstance,
			KtonInstance
		where
			Balance = Balance
		{
			// other data
		}
	}

	pub struct Author;
	impl OnUnbalanced<NegativeImbalance> for Author {
		fn on_nonzero_unbalanced(amount: NegativeImbalance) {
			Ring::resolve_creating(&Authorship::author(), amount);
		}
	}

	pub struct DealWithFees;
	impl OnUnbalanced<NegativeImbalance> for DealWithFees {
		fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
			if let Some(fees) = fees_then_tips.next() {
				// for fees, 80% to treasury, 20% to author
				let mut split = fees.ration(80, 20);
				if let Some(tips) = fees_then_tips.next() {
					// for tips, if any, 80% to treasury, 20% to author (though this can be anything)
					tips.ration_merge_into(80, 20, &mut split);
				}
				Treasury::on_unbalanced(split.0);
				Author::on_unbalanced(split.1);
			}
		}
	}

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, frame_system::MaximumBlockWeight]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// in Crab, extrinsic base weight (smallest non-zero weight) is mapped to 100 MILLI:
			let p = 100 * MILLI;
			let q = Balance::from(ExtrinsicBaseWeight::get());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational_approximation(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}

pub mod wasm {
	//! Make the WASM binary available.

	#[cfg(all(feature = "std", any(target_arch = "x86_64", target_arch = "x86")))]
	include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

	#[cfg(all(feature = "std", not(any(target_arch = "x86_64", target_arch = "x86"))))]
	pub const WASM_BINARY: &[u8] = include_bytes!("../../../../wasm/pangolin_runtime.compact.wasm");
	#[cfg(all(feature = "std", not(any(target_arch = "x86_64", target_arch = "x86"))))]
	pub const WASM_BINARY_BLOATY: &[u8] = include_bytes!("../../../../wasm/pangolin_runtime.wasm");

	#[cfg(feature = "std")]
	/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
	pub fn wasm_binary_unwrap() -> &'static [u8] {
		#[cfg(all(feature = "std", any(target_arch = "x86_64", target_arch = "x86")))]
		return WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is \
							built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
							production chains. Please rebuild with the flag disabled.",
		);
		#[cfg(all(feature = "std", not(any(target_arch = "x86_64", target_arch = "x86"))))]
		return WASM_BINARY;
	}
}

/// Weights for pallets used in the runtime.
mod weights;

// --- darwinia ---
use darwinia_evm::{
	Account as EVMAccount, AccountBasicMapping, EnsureAddressTruncated, FeeCalculator,
};
pub use darwinia_staking::StakerStatus;
pub use wasm::*;

// --- crates ---
use codec::{Decode, Encode};
use static_assertions::const_assert;
// --- substrate ---
use frame_support::{
	construct_runtime, debug, parameter_types,
	traits::{
		ChangeMembers, Currency, FindAuthor, InstanceFilter, KeyOwnerProofSystem, LockIdentifier,
		Randomness, U128CurrencyToVote,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		Weight,
	},
	ConsensusEngineId,
};
use frame_system::{EnsureOneOf, EnsureRoot};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo as TransactionPaymentRuntimeDispatchInfo;
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{
	crypto::{KeyTypeId, Public},
	u32_trait::{_1, _2, _3, _5},
	OpaqueMetadata, H160, H256, U256,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, ConvertInto, IdentityLookup, NumberFor, OpaqueKeys,
		SaturatedConversion, Saturating,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedPointNumber, ModuleId, OpaqueExtrinsic, Perbill, Percent, Permill,
	Perquintill, RuntimeDebug,
};
use sp_staking::SessionIndex;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia ---
use constants::*;
use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo as BalancesRuntimeDispatchInfo;
use darwinia_evm::Runner;
use darwinia_header_mmr_rpc_runtime_api::RuntimeDispatchInfo as HeaderMMRRuntimeDispatchInfo;
use darwinia_relay_primitives::relay_authorities::OpCode;
use darwinia_staking::EraIndex;
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo as StakingRuntimeDispatchInfo;
use drml_primitives::*;
use dvm_ethereum::{
	account_basic::DVMAccountBasicMapping,
	precompiles::{ConcatAddressMapping, NativeTransfer},
};
use dvm_rpc_runtime_api::TransactionStatus;
use impls::*;

/// The address format for describing accounts.
type Address = AccountId;
/// Block type as expected by this runtime.
type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Executive: handles dispatch to the various modules.
type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
	// CustomOnRuntimeUpgrade,
>;
/// The payload being signed in transactions.
type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

type Ring = Balances;

type NegativeImbalance = <darwinia_balances::Module<Runtime, RingInstance> as Currency<
	<Runtime as frame_system::Trait>::AccountId,
>>::NegativeImbalance;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("Pangolin"),
	impl_name: create_runtime_str!("Pangolin"),
	authoring_version: 1,
	spec_version: 10,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

const AVERAGE_ON_INITIALIZE_WEIGHT: Perbill = Perbill::from_percent(10);
parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight =
		AvailableBlockRatio::get().saturating_sub(AVERAGE_ON_INITIALIZE_WEIGHT)
		* MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const Version: RuntimeVersion = VERSION;
}
const_assert!(
	AvailableBlockRatio::get().deconstruct() >= AVERAGE_ON_INITIALIZE_WEIGHT.deconstruct()
);
impl frame_system::Trait for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = Nonce;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = RocksDbWeight;
	type BlockExecutionWeight = BlockExecutionWeight;
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxScheduledPerBlock: u32 = 50;
}
impl pallet_scheduler::Trait for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumBlockWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
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
				Call::Offences(..) |
				Call::Session(..) |
				Call::Grandpa(..) |
				Call::ImOnline(..) |
				Call::AuthorityDiscovery(..) |
				Call::Democracy(..) |
				Call::Council(..) |
				Call::TechnicalCommittee(..) |
				Call::ElectionsPhragmen(..) |
				Call::TechnicalMembership(..) |
				Call::Treasury(..) |
				Call::Claims(..) |
				Call::Scheduler(..) |
				Call::Proxy(..) |
				// Specifically omitting the entire Sudo pallet
				// Specifically omitting the entire CrabIssuing pallet
				// Specifically omitting the entire CrabBacking pallet
				Call::EthereumRelay(..) // Specifically omitting the entire EthereumBacking pallet
				                        // Specifically omitting the entire TronBacking pallet
				                        // Specifically omitting the entire CrabIssuing pallet
				                        // Specifically omitting the entire EthereumRelayAuthorities pallet
			),
			ProxyType::Governance => matches!(
				c,
				Call::Democracy(..)
					| Call::Council(..) | Call::TechnicalCommittee(..)
					| Call::ElectionsPhragmen(..)
					| Call::Treasury(..)
			),
			ProxyType::Staking => matches!(c, Call::Staking(..)),
			ProxyType::EthereumBridge => matches!(
				c,
				Call::EthereumBacking(..)
					| Call::EthereumRelay(..)
					| Call::EthereumRelayAuthorities(..)
			),
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
parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}
impl pallet_proxy::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
}

parameter_types! {
	pub const EpochDuration: u64 = BLOCKS_PER_SESSION as _;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}
impl pallet_babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::IdentificationTuple;
	type HandleEquivocation =
		pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}
impl pallet_timestamp::Trait for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

type RingInstance = darwinia_balances::Instance0;
parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * COIN;
	pub const MaxLocks: u32 = 50;
}
impl darwinia_balances::Trait<RingInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = (Kton,);
	type WeightInfo = weights::darwinia_balances::WeightInfo<Runtime>;
}
type KtonInstance = darwinia_balances::Instance1;
impl darwinia_balances::Trait<KtonInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = (Ring,);
	type WeightInfo = weights::darwinia_balances::WeightInfo<Runtime>;
}

/// Parameterized slow adjusting fee updated based on
/// https://w3f-research.readthedocs.io/en/latest/polkadot/Token%20Economics.html#-2.-slow-adjusting-mechanism
pub type SlowAdjustingFeeUpdate<R> =
	TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MICRO;
	/// The portion of the `AvailableBlockRatio` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero`.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}
impl pallet_transaction_payment::Trait for Runtime {
	type Currency = Ring;
	type OnTransactionPayment = DealWithFees;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}
impl pallet_authorship::Trait for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ImOnline);
}

parameter_types! {
	pub const StakingModuleId: ModuleId = ModuleId(*b"da/staki");
	pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDurationInEra: EraIndex = 14 * DAYS
		/ (SESSIONS_PER_ERA as BlockNumber * BLOCKS_PER_SESSION);
	pub const BondingDurationInBlockNumber: BlockNumber = 14 * DAYS;
	pub const SlashDeferDuration: EraIndex = 14 * DAYS
		/ (SESSIONS_PER_ERA as BlockNumber * BLOCKS_PER_SESSION) - 1;
	// quarter of the last session will be for election.
	pub const ElectionLookahead: BlockNumber = BLOCKS_PER_SESSION / 4;
	pub const MaxIterations: u32 = 5;
	pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
	pub const MaxNominatorRewardedPerValidator: u32 = 128;
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub OffchainSolutionWeightLimit: Weight = MaximumExtrinsicWeight::get()
		.saturating_sub(BlockExecutionWeight::get())
		.saturating_sub(ExtrinsicBaseWeight::get());
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl darwinia_staking::Trait for Runtime {
	type Event = Event;
	type ModuleId = StakingModuleId;
	type UnixTime = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = EnsureRootOrHalfCouncil;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority;
	// The unsigned solution weight targeted by the OCW. We set it to the maximum possible value of
	// a single extrinsic.
	type OffchainSolutionWeightLimit = OffchainSolutionWeightLimit;
	type RingCurrency = Ring;
	type RingRewardRemainder = Treasury;
	// send the slashed funds to the treasury.
	type RingSlash = Treasury;
	// rewards are minted from the void
	type RingReward = ();
	type KtonCurrency = Kton;
	// send the slashed funds to the treasury.
	type KtonSlash = Treasury;
	// rewards are minted from the void
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
	type WeightInfo = weights::darwinia_staking::WeightInfo<Runtime>;
}

parameter_types! {
	pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MaximumBlockWeight::get();
}
impl pallet_offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
	type WeightSoftLimit = OffencesWeightSoftLimit;
}

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification = darwinia_staking::Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = darwinia_staking::ExposureOf<Runtime>;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub grandpa: Grandpa,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}
parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}
impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = darwinia_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	type KeyOwnerProofSystem = Historical;
	type HandleEquivocation =
		pallet_grandpa::EquivocationHandler<Self::KeyOwnerIdentification, Offences>;
	type WeightInfo = ();
}

parameter_types! {
	pub const SessionDuration: BlockNumber = BLOCKS_PER_SESSION as _;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}
impl pallet_im_online::Trait for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type SessionDuration = SessionDuration;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
}

impl pallet_authority_discovery::Trait for Runtime {}

type EnsureRootOrHalfCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>,
>;
parameter_types! {
	pub const LaunchPeriod: BlockNumber = 7 * DAYS;
	pub const VotingPeriod: BlockNumber = 7 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub const MinimumDeposit: Balance = 1 * MILLI;
	pub const EnactmentPeriod: BlockNumber = 8 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
	pub const PreimageByteDeposit: Balance = 10 * NANO;
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}
impl darwinia_democracy::Trait for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Ring;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EnsureRootOrHalfCouncil;
	/// A majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EnsureRootOrHalfCouncil;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cooloff period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type MaxProposals = MaxProposals;
	type WeightInfo = weights::darwinia_democracy::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}
// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());
type CouncilCollective = pallet_collective::Instance0;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
}
type TechnicalCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<TechnicalCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: LockIdentifier = *b"da/phrel";
	pub const CandidacyBond: Balance = 1 * COIN;
	pub const VotingBond: Balance = 5 * MILLI;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
	/// Daily council elections.
	pub const TermDuration: BlockNumber = 24 * HOURS;
}
impl darwinia_elections_phragmen::Trait for Runtime {
	type Event = Event;
	type ModuleId = ElectionsPhragmenModuleId;
	type Currency = Ring;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type LoserCandidate = Treasury;
	type BadReport = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type WeightInfo = weights::darwinia_elections_phragmen::WeightInfo<Runtime>;
}

pub struct MembershipChangedGroup;
impl ChangeMembers<AccountId> for MembershipChangedGroup {
	fn change_members_sorted(
		incoming: &[AccountId],
		outgoing: &[AccountId],
		sorted_new: &[AccountId],
	) {
		TechnicalCommittee::change_members_sorted(incoming, outgoing, sorted_new);
		EthereumRelay::change_members_sorted(incoming, outgoing, sorted_new);
	}
}
type EnsureRootOrMoreThanHalfCouncil = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;
impl pallet_membership::Trait<pallet_membership::Instance0> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrMoreThanHalfCouncil;
	type RemoveOrigin = EnsureRootOrMoreThanHalfCouncil;
	type SwapOrigin = EnsureRootOrMoreThanHalfCouncil;
	type ResetOrigin = EnsureRootOrMoreThanHalfCouncil;
	type PrimeOrigin = EnsureRootOrMoreThanHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = MembershipChangedGroup;
}

type ApproveOrigin = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
>;
parameter_types! {
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"da/trsry");
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 20 * COIN;
	pub const KtonProposalBondMinimum: Balance = 20 * COIN;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * COIN;
	pub const DataDepositPerByte: Balance = 1 * MILLI;
	pub const BountyDepositBase: Balance = 1 * COIN;
	pub const BountyDepositPayoutDelay: BlockNumber = 4 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 2 * COIN;
}
impl darwinia_treasury::Trait for Runtime {
	type ModuleId = TreasuryModuleId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = EnsureRootOrMoreThanHalfCouncil;
	type Tippers = ElectionsPhragmen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type OnSlashRing = Treasury;
	type OnSlashKton = Treasury;
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type MaximumReasonLength = MaximumReasonLength;
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyValueMinimum = BountyValueMinimum;
	type RingBurnDestination = ();
	type KtonBurnDestination = ();
	type WeightInfo = weights::darwinia_treasury::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ClaimsModuleId: ModuleId = ModuleId(*b"da/claim");
	pub Prefix: &'static [u8] = b"Pay RINGs to the template account:";
}
impl darwinia_claims::Trait for Runtime {
	type Event = Event;
	type ModuleId = ClaimsModuleId;
	type Prefix = Prefix;
	type RingCurrency = Ring;
	type MoveClaimOrigin = EnsureRootOrMoreThanHalfCouncil;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * MILLI;
}
impl darwinia_vesting::Trait for Runtime {
	type Event = Event;
	type Currency = Ring;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const CrabIssuingModuleId: ModuleId = ModuleId(*b"da/crais");
}
impl darwinia_crab_issuing::Trait for Runtime {
	type Event = Event;
	type ModuleId = CrabIssuingModuleId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}

parameter_types! {
	pub const CrabBackingModuleId: ModuleId = ModuleId(*b"da/crabk");
}
impl darwinia_crab_backing::Trait for Runtime {
	type ModuleId = CrabBackingModuleId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}

parameter_types! {
	pub const EthereumBackingModuleId: ModuleId = ModuleId(*b"da/ethbk");
	pub const EthereumBackingFeeModuleId: ModuleId = ModuleId(*b"da/ethfe");
	pub const RingLockLimit: Balance = 10_000_000 * COIN;
	pub const KtonLockLimit: Balance = 1000 * COIN;
	pub const AdvancedFee: Balance = 50 * COIN;
	pub const SyncReward: Balance = 1000 * COIN;
}
impl darwinia_ethereum_backing::Trait for Runtime {
	type ModuleId = EthereumBackingModuleId;
	type FeeModuleId = EthereumBackingFeeModuleId;
	type Event = Event;
	type RedeemAccountId = AccountId;
	type EthereumRelay = EthereumRelay;
	type OnDepositRedeem = Staking;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type RingLockLimit = RingLockLimit;
	type KtonLockLimit = KtonLockLimit;
	type AdvancedFee = AdvancedFee;
	type SyncReward = SyncReward;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
}

type EnsureRootOrHalfTechnicalComittee = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, TechnicalCollective>,
>;
parameter_types! {
	pub const EthereumRelayModuleId: ModuleId = ModuleId(*b"da/ethrl");
	pub const EthereumNetwork: ethereum_primitives::EthereumNetworkType = ethereum_primitives::EthereumNetworkType::Ropsten;
	pub const ConfirmPeriod: BlockNumber = 200;
	pub const ApproveThreshold: Perbill = Perbill::from_percent(60);
	pub const RejectThreshold: Perbill = Perbill::from_percent(1);
}
impl darwinia_ethereum_relay::Trait for Runtime {
	type ModuleId = EthereumRelayModuleId;
	type Event = Event;
	type EthereumNetwork = EthereumNetwork;
	type Call = Call;
	type Currency = Ring;
	type RelayerGame = EthereumRelayerGame;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = EnsureRootOrHalfTechnicalComittee;
	type ConfirmPeriod = ConfirmPeriod;
	type TechnicalMembership = TechnicalMembership;
	type ApproveThreshold = ApproveThreshold;
	type RejectThreshold = RejectThreshold;
	type WeightInfo = ();
}

type EthereumRelayerGameInstance = darwinia_relayer_game::Instance0;
parameter_types! {
	pub const EthereumRelayerGameLockId: LockIdentifier = *b"ethrgame";
}
impl darwinia_relayer_game::Trait<EthereumRelayerGameInstance> for Runtime {
	type RingCurrency = Ring;
	type LockId = EthereumRelayerGameLockId;
	type RingSlash = Treasury;
	type RelayerGameAdjustor = relay::EthereumRelayerGameAdjustor;
	type RelayableChain = EthereumRelay;
	type WeightInfo = ();
}

parameter_types! {
	pub const TronBackingModuleId: ModuleId = ModuleId(*b"da/trobk");
}
impl darwinia_tron_backing::Trait for Runtime {
	type ModuleId = TronBackingModuleId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type WeightInfo = ();
}

impl darwinia_header_mmr::Trait for Runtime {}

/// Fixed gas price of `1`.
pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		// Gas price is always one token per gas.
		0.into()
	}
}
parameter_types! {
	pub const ChainId: u64 = 43;
}
impl darwinia_evm::Trait for Runtime {
	type FeeCalculator = FixedGasPrice;
	type GasToWeight = ();
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = ConcatAddressMapping;
	type Currency = Balances;
	type Event = Event;
	type Precompiles = (
		darwinia_evm::precompiles::ECRecover,
		darwinia_evm::precompiles::Sha256,
		darwinia_evm::precompiles::Ripemd160,
		darwinia_evm::precompiles::Identity,
		NativeTransfer<Self>,
	);
	type ChainId = ChainId;
	type AccountBasicMapping = DVMAccountBasicMapping<Self>;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
}

type EthereumRelayAuthoritiesInstance = darwinia_relay_authorities::Instance0;
parameter_types! {
	pub const EthereumRelayAuthoritiesLockId: LockIdentifier = *b"ethrauth";
	pub const EthereumRelayAuthoritiesTermDuration: BlockNumber = 30 * DAYS;
	pub const MaxCandidates: usize = 7;
	pub const OpCodes: (OpCode, OpCode) = (
		[71, 159, 189, 249],
		[180, 188, 244, 151]
	);
	pub const SignThreshold: Perbill = Perbill::from_percent(60);
	pub const SubmitDuration: BlockNumber = 100;
}
impl darwinia_relay_authorities::Trait<EthereumRelayAuthoritiesInstance> for Runtime {
	type Event = Event;
	type RingCurrency = Ring;
	type LockId = EthereumRelayAuthoritiesLockId;
	type TermDuration = EthereumRelayAuthoritiesTermDuration;
	type MaxCandidates = MaxCandidates;
	type AddOrigin = ApproveOrigin;
	type RemoveOrigin = ApproveOrigin;
	type ResetOrigin = ApproveOrigin;
	type DarwiniaMMR = HeaderMMR;
	type Sign = EthereumBacking;
	type OpCodes = OpCodes;
	type SignThreshold = SignThreshold;
	type SubmitDuration = SubmitDuration;
	type WeightInfo = ();
}

pub struct EthereumFindAuthor<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Babe::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.0.to_raw_vec()[4..24]));
		}
		None
	}
}
impl dvm_ethereum::Trait for Runtime {
	type Event = Event;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type AddressMapping = ConcatAddressMapping;
	type RingCurrency = Balances;
}

construct_runtime!(
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = OpaqueBlock,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Basic stuff; balances is uncallable initially.
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},

		// Must be before session.
		Babe: pallet_babe::{Module, Call, Storage, Config, Inherent, ValidateUnsigned},

		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Balances: darwinia_balances::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance1>::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},

		// Consensus support.
		Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
		Staking: darwinia_staking::{Module, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
		Offences: pallet_offences::{Module, Call, Storage, Event},
		Historical: pallet_session_historical::{Module},
		Session: pallet_session::{Module, Call, Storage, Config<T>, Event},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event, ValidateUnsigned},
		ImOnline: pallet_im_online::{Module, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
		AuthorityDiscovery: pallet_authority_discovery::{Module, Call, Config},

		// Governance stuff; uncallable initially.
		Democracy: darwinia_democracy::{Module, Call, Storage, Config, Event<T>},
		Council: pallet_collective::<Instance0>::{Module, Call, Storage, Origin<T>, Config<T>, Event<T>},
		TechnicalCommittee: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Config<T>, Event<T>},
		ElectionsPhragmen: darwinia_elections_phragmen::{Module, Call, Storage, Config<T>, Event<T>},
		TechnicalMembership: pallet_membership::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},
		Treasury: darwinia_treasury::{Module, Call, Storage, Event<T>},

		// Claims. Usable initially.
		Claims: darwinia_claims::{Module, Call, Storage, Config, Event<T>, ValidateUnsigned},

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: darwinia_vesting::{Module, Call, Storage, Event<T>, Config<T>},

		// System scheduler.
		Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},

		// Proxy module. Late addition.
		Proxy: pallet_proxy::{Module, Call, Storage, Event<T>} = 30,

		Sudo: pallet_sudo::{Module, Call, Storage, Config<T>, Event<T>},

		CrabIssuing: darwinia_crab_issuing::{Module, Call, Storage, Config, Event<T>},
		CrabBacking: darwinia_crab_backing::{Module, Storage, Config<T>},

		EthereumRelayerGame: darwinia_relayer_game::<Instance0>::{Module, Storage},
		EthereumRelay: darwinia_ethereum_relay::{Module, Call, Storage, Config<T>, Event<T>},
		EthereumBacking: darwinia_ethereum_backing::{Module, Call, Storage, Config<T>, Event<T>},

		TronBacking: darwinia_tron_backing::{Module, Storage, Config<T>},

		HeaderMMR: darwinia_header_mmr::{Module, Call, Storage},

		EVM: darwinia_evm::{Module, Config, Call, Storage, Event<T>},
		Ethereum: dvm_ethereum::{Module, Call, Storage, Event, Config, ValidateUnsigned},

		EthereumRelayAuthorities: darwinia_relay_authorities::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},
	}
);

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as sp_runtime::traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		Call,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				debug::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (account, signature, extra)))
	}
}
impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}
impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(
			data: sp_inherents::InherentData
		) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::SlotNumber {
			Babe::current_epoch_start()
		}

		fn generate_key_ownership_proof(
			_slot_number: sp_consensus_babe::SlotNumber,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> TransactionPaymentRuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	}

	impl darwinia_balances_rpc_runtime_api::BalancesApi<Block, AccountId, Balance> for Runtime {
		fn usable_balance(instance: u8, account: AccountId) -> BalancesRuntimeDispatchInfo<Balance> {
			match instance {
				0 => Ring::usable_balance_rpc(account),
				1 => Kton::usable_balance_rpc(account),
				_ => Default::default()
			}
		}
	}

	impl darwinia_header_mmr_rpc_runtime_api::HeaderMMRApi<Block, Hash> for Runtime {
		fn gen_proof(
			block_number_of_member_leaf: u64,
			block_number_of_last_leaf: u64
		) -> HeaderMMRRuntimeDispatchInfo<Hash> {
			HeaderMMR::gen_proof_rpc(block_number_of_member_leaf, block_number_of_last_leaf )
		}
	}

	impl darwinia_staking_rpc_runtime_api::StakingApi<Block, AccountId, Power> for Runtime {
		fn power_of(account: AccountId) -> StakingRuntimeDispatchInfo<Power> {
			Staking::power_of_rpc(account)
		}
	}

	impl dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as darwinia_evm::Trait>::ChainId::get()
		}

		fn gas_price() -> U256 {
			<Runtime as darwinia_evm::Trait>::FeeCalculator::min_gas_price()
		}

		fn account_basic(address: H160) -> EVMAccount {
			<Runtime as darwinia_evm::Trait>::AccountBasicMapping::account_basic(&address)
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			darwinia_evm::Module::<Runtime>::account_codes(address)
		}

		fn author() -> H160 {
			<dvm_ethereum::Module<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			darwinia_evm::Module::<Runtime>::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<darwinia_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Trait>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as darwinia_evm::Trait>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.low_u32(),
				gas_price,
				nonce,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Trait>::config()),
			).map_err(|err| err.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<darwinia_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Trait>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as darwinia_evm::Trait>::Runner::create(
				from,
				data,
				value,
				gas_limit.low_u32(),
				gas_price,
				nonce,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Trait>::config()),
			).map_err(|err| err.into())
		}


		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<dvm_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<dvm_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<dvm_ethereum::Block>,
			Option<Vec<dvm_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}
	}
}

pub struct TransactionConverter;
impl dvm_rpc_runtime_api::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			<dvm_ethereum::Call<Runtime>>::transact(transaction).into(),
		)
	}
}
impl dvm_rpc_runtime_api::ConvertTransaction<OpaqueExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> OpaqueExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			<dvm_ethereum::Call<Runtime>>::transact(transaction).into(),
		);
		let encoded = extrinsic.encode();

		OpaqueExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

// pub struct CustomOnRuntimeUpgrade;
// impl frame_support::traits::OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
// 	fn on_runtime_upgrade() -> frame_support::weights::Weight {
// 		// --- substrate ---
// 		use frame_support::migration::*;

// 		<Runtime as frame_system::Trait>::MaximumBlockWeight::get()
// 	}
// }
