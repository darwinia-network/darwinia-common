//! The Darwinia Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod impls {
	//! Some configurable implementations as associated type for the substrate runtime.

	pub mod bridge {
		// --- darwinia ---
		use crate::{impls::*, *};
		use darwinia_support::relay::*;

		pub struct EthRelayerGameAdjustor;
		impl AdjustableRelayerGame for EthRelayerGameAdjustor {
			type Moment = BlockNumber;
			type Balance = Balance;
			type TcBlockNumber = <EthRelay as darwinia_support::relay::Relayable>::TcBlockNumber;

			fn challenge_time(round: Round) -> Self::Moment {
				match round {
					// 3 mins
					0 => 30,
					// 1 mins
					_ => 10,
				}
			}

			fn round_from_chain_len(chain_len: u64) -> Round {
				chain_len - 1
			}

			fn chain_len_from_round(round: Round) -> u64 {
				round + 1
			}

			fn update_samples(samples: &mut Vec<Vec<Self::TcBlockNumber>>) {
				samples.push(vec![samples.last().unwrap().last().unwrap() - 1]);
			}

			fn estimate_bond(round: Round, proposals_count: u64) -> Self::Balance {
				match round {
					0 => match proposals_count {
						0 => 1000 * COIN,
						_ => 1500 * COIN,
					},
					_ => 100 * COIN,
				}
			}
		}
	}

	// --- substrate ---
	use frame_support::traits::{Currency, Get, Imbalance, OnUnbalanced};
	use sp_runtime::{traits::Convert, Fixed128, FixedPointNumber, Perquintill};
	// --- darwinia ---
	use crate::{primitives::*, *};

	darwinia_support::impl_account_data! {
		struct AccountData<Balance>
		for
			RingInstance,
			KtonInstance
		where
			Balance = u128
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

	/// Struct that handles the conversion of Balance -> `u64`. This is used for staking's election
	/// calculation.
	pub struct CurrencyToVoteHandler;
	impl CurrencyToVoteHandler {
		fn factor() -> Balance {
			(Balances::total_issuance() / u64::max_value() as Balance).max(1)
		}
	}
	impl Convert<Balance, u64> for CurrencyToVoteHandler {
		fn convert(x: Balance) -> u64 {
			(x / Self::factor()) as u64
		}
	}
	impl Convert<u128, Balance> for CurrencyToVoteHandler {
		fn convert(x: u128) -> Balance {
			x * Self::factor()
		}
	}

	/// Update the given multiplier based on the following formula
	///
	///   diff = (previous_block_weight - target_weight)/max_weight
	///   v = 0.00004
	///   next_weight = weight * (1 + (v * diff) + (v * diff)^2 / 2)
	///
	/// Where `target_weight` must be given as the `Get` implementation of the `T` generic type.
	/// https://research.web3.foundation/en/latest/polkadot/Token%20Economics/#relay-chain-transaction-fees
	pub struct TargetedFeeAdjustment<T>(sp_std::marker::PhantomData<T>);

	impl<T: Get<Perquintill>> Convert<Fixed128, Fixed128> for TargetedFeeAdjustment<T> {
		fn convert(multiplier: Fixed128) -> Fixed128 {
			let max_weight = MaximumBlockWeight::get();
			let block_weight = System::block_weight().total().min(max_weight);
			let target_weight = (T::get() * max_weight) as u128;
			let block_weight = block_weight as u128;

			// determines if the first_term is positive
			let positive = block_weight >= target_weight;
			let diff_abs = block_weight.max(target_weight) - block_weight.min(target_weight);
			// safe, diff_abs cannot exceed u64.
			let diff = Fixed128::saturating_from_rational(diff_abs, max_weight.max(1));
			let diff_squared = diff.saturating_mul(diff);

			// 0.00004 = 4/100_000 = 40_000/10^9
			let v = Fixed128::saturating_from_rational(4, 100_000);
			// 0.00004^2 = 16/10^10 Taking the future /2 into account... 8/10^10
			let v_squared_2 = Fixed128::saturating_from_rational(8, 10_000_000_000u64);

			let first_term = v.saturating_mul(diff);
			let second_term = v_squared_2.saturating_mul(diff_squared);

			if positive {
				// Note: this is merely bounded by how big the multiplier and the inner value can go,
				// not by any economical reasoning.
				let excess = first_term.saturating_add(second_term);
				multiplier.saturating_add(excess)
			} else {
				// Defensive-only: first_term > second_term. Safe subtraction.
				let negative = first_term.saturating_sub(second_term);
				multiplier
					.saturating_sub(negative)
					// despite the fact that apply_to saturates weight (final fee cannot go below 0)
					// it is crucially important to stop here and don't further reduce the weight fee
					// multiplier. While at -1, it means that the network is so un-congested that all
					// transactions have no weight fee. We stop here and only increase if the network
					// became more busy.
					.max(Fixed128::saturating_from_integer(-1))
			}
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
}

pub mod opaque {
	//! Opaque types. These are used by the CLI to instantiate machinery that don't need to know
	//! the specifics of the runtime. They can then be made to be agnostic over specific formats
	//! of data like extrinsics, allowing for them to continue syncing the network through upgrades
	//! to even the core data structures.

	// --- substrate ---
	pub use sp_runtime::{generic, traits::BlakeTwo256, OpaqueExtrinsic as UncheckedExtrinsic};
	// --- darwinia ---
	use crate::primitives::*;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

pub mod primitives {
	/// App-specific crypto used for reporting equivocation/misbehavior in BABE and
	/// GRANDPA. Any rewards for misbehavior reporting will be paid out to this
	/// account.
	pub mod report {
		// --- substrate ---
		use frame_system::offchain::AppCrypto;
		use sp_core::crypto::{key_types, KeyTypeId};
		// --- crates ---
		use crate::primitives::{Signature, Verify};

		/// Key type for the reporting module. Used for reporting BABE and GRANDPA
		/// equivocations.
		pub const KEY_TYPE: KeyTypeId = key_types::REPORTING;

		mod app {
			use sp_application_crypto::{app_crypto, sr25519};
			app_crypto!(sr25519, super::KEY_TYPE);
		}

		/// Identity of the equivocation/misbehavior reporter.
		pub type ReporterId = app::Public;

		/// An `AppCrypto` type to allow submitting signed transactions using the reporting
		/// application key as signer.
		pub struct ReporterAppCrypto;

		impl AppCrypto<<Signature as Verify>::Signer, Signature> for ReporterAppCrypto {
			type RuntimeAppPublic = ReporterId;
			type GenericPublic = sp_core::sr25519::Public;
			type GenericSignature = sp_core::sr25519::Signature;
		}
	}

	// --- substrate ---
	use frame_support::traits::Currency;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, IdentifyAccount, Verify},
		MultiSignature,
	};
	// --- darwinia ---
	use crate::*;

	/// An index to a block.
	pub type BlockNumber = u32;

	/// An instant or duration in time.
	pub type Moment = u64;

	/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
	pub type Signature = MultiSignature;

	/// Some way of identifying an account on the chain. We intentionally make it equivalent
	/// to the public key of our transaction signing scheme.
	pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

	/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
	/// never know...
	pub type AccountIndex = u32;

	/// Balance of an account.
	pub type Balance = u128;

	/// Index of a transaction in the chain.
	pub type Nonce = u32;

	/// A hash of some data used by the chain.
	pub type Hash = sp_core::H256;

	/// Digest item type.
	pub type DigestItem = generic::DigestItem<Hash>;

	/// Power of an account.
	pub type Power = u32;

	/// Alias Balances Module as Ring Module.
	pub type Ring = Balances;

	pub type NegativeImbalance = <Ring as Currency<AccountId>>::NegativeImbalance;

	/// The address format for describing accounts.
	pub type Address = AccountId;

	/// Block header type as expected by this runtime.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

	/// Block type as expected by this runtime.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	/// A Block signed with a Justification
	pub type SignedBlock = generic::SignedBlock<Block>;

	/// BlockId type as expected by this runtime.
	pub type BlockId = generic::BlockId<Block>;

	/// The SignedExtension to the basic transaction logic.
	pub type SignedExtra = (
		frame_system::CheckSpecVersion<Runtime>,
		frame_system::CheckTxVersion<Runtime>,
		frame_system::CheckGenesis<Runtime>,
		frame_system::CheckEra<Runtime>,
		frame_system::CheckNonce<Runtime>,
		frame_system::CheckWeight<Runtime>,
		pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
		pallet_grandpa::ValidateEquivocationReport<Runtime>,
		darwinia_ethereum_linear_relay::CheckEthRelayHeaderHash<Runtime>,
	);

	/// Unchecked extrinsic type as expected by this runtime.
	pub type UncheckedExtrinsic =
		generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

	/// The payload being signed in transactions.
	pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

	/// Extrinsic type that has already been checked.
	pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Nonce, Call>;

	/// Executive: handles dispatch to the various modules.
	pub type Executive = frame_executive::Executive<
		Runtime,
		Block,
		frame_system::ChainContext<Runtime>,
		Runtime,
		AllModules,
	>;
}

// --- crates ---
use codec::{Decode, Encode};
use static_assertions::const_assert;
// --- substrate ---
use frame_support::{
	construct_runtime, debug, parameter_types,
	traits::{KeyOwnerProofSystem, LockIdentifier, Randomness},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		IdentityFee, Weight,
	},
};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo as TransactionPaymentRuntimeDispatchInfo;
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _5},
	OpaqueMetadata,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, IdentityLookup, NumberFor, OpaqueKeys, SaturatedConversion,
		Saturating,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ModuleId, PerThing, Perbill, Percent, Permill, Perquintill, RuntimeDebug,
};
use sp_staking::SessionIndex;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia ---
use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo as BalancesRuntimeDispatchInfo;
use darwinia_ethereum_linear_relay::EthNetworkType;
use darwinia_ethereum_offchain::crypto::AuthorityId as EthOffchainId;
use darwinia_header_mmr_rpc_runtime_api::RuntimeDispatchInfo as HeaderMMRRuntimeDispatchInfo;
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo as StakingRuntimeDispatchInfo;
use impls::*;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-template"),
	impl_name: create_runtime_str!("node-template"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const MILLISECS_PER_BLOCK: u64 = 3000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const BLOCKS_PER_SESSION: BlockNumber = MINUTES / 2;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(BLOCKS_PER_SESSION as f64 * SLOT_FILL_RATE) as u64
};
pub const SESSION_DURATION: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
pub const SESSIONS_PER_ERA: SessionIndex = 3;

pub const CAP: Balance = 10_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

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
const_assert!(AvailableBlockRatio::get().deconstruct() >= AVERAGE_ON_INITIALIZE_WEIGHT.deconstruct());
impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Call = Call;
	type Index = Nonce;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
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
	type ModuleToIndex = ModuleToIndex;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}
impl pallet_babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	// session module is the trigger
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}
impl pallet_timestamp::Trait for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MICRO;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
}
// for a sane configuration, this should always be less than `AvailableBlockRatio`.
const_assert!(
	TargetBlockFullness::get().deconstruct() <
	(AvailableBlockRatio::get().deconstruct() as <Perquintill as PerThing>::Inner)
		* (<Perquintill as PerThing>::ACCURACY / <Perbill as PerThing>::ACCURACY as <Perquintill as PerThing>::Inner)
);
impl pallet_transaction_payment::Trait for Runtime {
	type Currency = Ring;
	type OnTransactionPayment = DealWithFees;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<TargetBlockFullness>;
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
}

parameter_types! {
	pub const WindowSize: BlockNumber = pallet_finality_tracker::DEFAULT_WINDOW_SIZE.into();
	pub const ReportLatency: BlockNumber = pallet_finality_tracker::DEFAULT_REPORT_LATENCY.into();
}
impl pallet_finality_tracker::Trait for Runtime {
	type OnFinalizationStalled = ();
	type WindowSize = WindowSize;
	type ReportLatency = ReportLatency;
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
	type HandleEquivocation = pallet_grandpa::EquivocationHandler<
		Self::KeyOwnerIdentification,
		primitives::report::ReporterAppCrypto,
		Runtime,
		Offences,
	>;
}

parameter_types! {
	pub const SessionDuration: BlockNumber = SESSION_DURATION;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}
impl pallet_im_online::Trait for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type SessionDuration = SessionDuration;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
}

impl pallet_authority_discovery::Trait for Runtime {}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
}
type CouncilCollective = pallet_collective::Instance0;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
}
type TechnicalCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<TechnicalCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
}

impl pallet_membership::Trait<pallet_membership::Instance0> for Runtime {
	type Event = Event;
	type AddOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type RemoveOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type SwapOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type ResetOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type PrimeOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

type RingInstance = darwinia_balances::Instance0;
parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * COIN;
}
impl darwinia_balances::Trait<RingInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Kton,);
}
type KtonInstance = darwinia_balances::Instance1;
impl darwinia_balances::Trait<KtonInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Ring,);
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDurationInEra: darwinia_staking::EraIndex = 14 * 24 * (HOURS / (SESSIONS_PER_ERA * BLOCKS_PER_SESSION));
	pub const BondingDurationInBlockNumber: BlockNumber = 14 * DAYS;
	pub const SlashDeferDuration: darwinia_staking::EraIndex = 0;
	pub const ElectionLookahead: BlockNumber = BLOCKS_PER_SESSION / 4;
	pub const MaxIterations: u32 = 5;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	/// We prioritize im-online heartbeats over phragmen solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl darwinia_staking::Trait for Runtime {
	type Event = Event;
	type UnixTime = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority;
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
}

// Make sure that there are no more than `MAX_MEMBERS` members elected via phragmen.
const_assert!(DESIRED_MEMBERS <= pallet_collective::MAX_MEMBERS);
parameter_types! {
	pub const ElectionsPhragmenModuleId: LockIdentifier = *b"da/phrel";
	pub const CandidacyBond: Balance = 1 * COIN;
	pub const VotingBond: Balance = 5 * MILLI;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
	/// Daily council elections.
	pub const TermDuration: BlockNumber = 24 * HOURS;
}
// Make sure that there are no more than `MAX_MEMBERS` members elected via phragmen.
const_assert!(DesiredMembers::get() <= pallet_collective::MAX_MEMBERS);
impl darwinia_elections_phragmen::Trait for Runtime {
	type Event = Event;
	type ModuleId = ElectionsPhragmenModuleId;
	type Currency = Ring;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = CurrencyToVoteHandler;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type LoserCandidate = Treasury;
	type BadReport = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
}

parameter_types! {
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"da/trsry");
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * COIN;
	pub const TipReportDepositPerByte: Balance = 1 * MILLI;
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 20 * COIN;
	pub const KtonProposalBondMinimum: Balance = 20 * COIN;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
}
impl darwinia_treasury::Trait for Runtime {
	type ModuleId = TreasuryModuleId;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin =
		pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>;
	type RejectOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type Tippers = ElectionsPhragmen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = Event;
	type RingProposalRejection = Treasury;
	type KtonProposalRejection = Treasury;
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
}

parameter_types! {
	pub const ClaimsModuleId: ModuleId = ModuleId(*b"da/claim");
	pub const Prefix: &'static [u8] = b"Pay RINGs to the template account:";
}
impl darwinia_claims::Trait for Runtime {
	type Event = Event;
	type ModuleId = ClaimsModuleId;
	type Prefix = Prefix;
	type RingCurrency = Ring;
}

parameter_types! {
	pub const EthBackingModuleId: ModuleId = ModuleId(*b"da/backi");
	pub const SubKeyPrefix: u8 = 42;
}
impl darwinia_ethereum_backing::Trait for Runtime {
	type ModuleId = EthBackingModuleId;
	type Event = Event;
	type DetermineAccountId = darwinia_ethereum_backing::AccountIdDeterminator<Runtime>;
	type EthRelay = EthLinearRelay;
	type OnDepositRedeem = Staking;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type SubKeyPrefix = SubKeyPrefix;
}

parameter_types! {
	pub const EthereumLinearRelayModuleId: ModuleId = ModuleId(*b"da/ethli");
	pub const EthNetwork: EthNetworkType = EthNetworkType::Mainnet;
}
impl darwinia_ethereum_linear_relay::Trait for Runtime {
	type ModuleId = EthereumLinearRelayModuleId;
	type Event = Event;
	type EthNetwork = EthNetwork;
	type Call = Call;
	type Currency = Ring;
}

parameter_types! {
	pub const EthereumRelayModuleId: ModuleId = ModuleId(*b"da/ethrl");
}

impl darwinia_ethereum_relay::Trait for Runtime {
	type ModuleId = EthereumRelayModuleId;
	type Event = Event;
	type Currency = Ring;
}

parameter_types! {
	pub const FetchInterval: BlockNumber = 3;
}
impl darwinia_ethereum_offchain::Trait for Runtime {
	type AuthorityId = EthOffchainId;
	type FetchInterval = FetchInterval;
}

impl darwinia_header_mmr::Trait for Runtime {}

type EthereumRelayerGameInstance = darwinia_relayer_game::Instance0;
parameter_types! {
	pub const ConfirmPeriod: BlockNumber = 200;
}
impl darwinia_relayer_game::Trait<EthereumRelayerGameInstance> for Runtime {
	type Event = Event;
	type RingCurrency = Ring;
	type RingSlash = Treasury;
	type RelayerGameAdjustor = bridge::EthRelayerGameAdjustor;
	type TargetChain = EthRelay;
	type ConfirmPeriod = ConfirmPeriod;
	type ApproveOrigin =
		pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>;
	type RejectOrigin =
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
}

construct_runtime!(
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// --- substrate ---
		// Basic stuff; balances is uncallable initially.
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},

		// Must be before session.
		Babe: pallet_babe::{Module, Call, Storage, Config, Inherent(Timestamp)},

		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},

		// Consensus support.
		Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
		Offences: pallet_offences::{Module, Call, Storage, Event},
		Historical: pallet_session_historical::{Module},
		Session: pallet_session::{Module, Call, Storage, Config<T>, Event},
		FinalityTracker: pallet_finality_tracker::{Module, Call, Storage, Inherent},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
		ImOnline: pallet_im_online::{Module, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
		AuthorityDiscovery: pallet_authority_discovery::{Module, Call, Config},

		// Governance stuff; uncallable initially.
		// Democracy: pallet_democracy::{Module, Call, Storage, Config, Event<T>},
		Council: pallet_collective::<Instance0>::{Module, Call, Storage, Origin<T>, Config<T>, Event<T>},
		TechnicalCommittee: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Config<T>, Event<T>},
		TechnicalMembership: pallet_membership::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},

		Sudo: pallet_sudo::{Module, Call, Storage, Config<T>, Event<T>},

		// --- darwinia ---
		// Basic stuff; balances is uncallable initially.
		Balances: darwinia_balances::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance1>::{Module, Call, Storage, Config<T>, Event<T>},

		// Consensus support.
		Staking: darwinia_staking::{Module, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},

		// Governance stuff; uncallable initially.
		ElectionsPhragmen: darwinia_elections_phragmen::{Module, Call, Storage, Config<T>, Event<T>},

		// Claims. Usable initially.
		Claims: darwinia_claims::{Module, Call, Storage, Config, Event<T>, ValidateUnsigned},

		EthBacking: darwinia_ethereum_backing::{Module, Call, Storage, Config<T>, Event<T>},
		EthLinearRelay: darwinia_ethereum_linear_relay::{Module, Call, Storage, Config<T>, Event<T>},
		EthOffchain: darwinia_ethereum_offchain::{Module, Call},
		EthRelay: darwinia_ethereum_relay::{Module, Call, Storage, Event<T>},

		HeaderMMR: darwinia_header_mmr::{Module, Call, Storage},

		RelayerGame: darwinia_relayer_game::<Instance0>::{Module, Call, Storage, Event<T>},

		// Governance stuff; uncallable initially.
		Treasury: darwinia_treasury::{Module, Call, Storage, Event<T>},
	}
);

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
			pallet_grandpa::ValidateEquivocationReport::<Runtime>::new(),
			Default::default(),
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

		fn submit_report_equivocation_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_report_equivocation_extrinsic(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

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
		UncheckedExtrinsic,
	> for Runtime {
		fn query_info(uxt: UncheckedExtrinsic, len: u32) -> TransactionPaymentRuntimeDispatchInfo<Balance> {
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

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			pallet: Vec<u8>,
			benchmark: Vec<u8>,
			lowest_range_values: Vec<u32>,
			highest_range_values: Vec<u32>,
			steps: Vec<u32>,
			repeat: u32,
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark};
			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency issues.
			// To get around that, we separated the Session benchmarks into its own crate, which is why
			// we need these two lines below.
			// TODO: benchmark
			// use darwinia_session_benchmarking::Module as SessionBench;
			// impl darwinia_session_benchmarking::Trait for Runtime {}
			//
			// let mut batches = Vec::<BenchmarkBatch>::new();
			// let params = (&pallet, &benchmark, &lowest_range_values, &highest_range_values, &steps, repeat);
			// add_benchmark!(params, batches, b"balances", Balances);
			// add_benchmark!(params, batches, b"im-online", ImOnline);
			// add_benchmark!(params, batches, b"identity", Identity);
			// add_benchmark!(params, batches, b"session", SessionBench::<Runtime>);
			// add_benchmark!(params, batches, b"staking", Staking);
			// add_benchmark!(params, batches, b"timestamp", Timestamp);
			// add_benchmark!(params, batches, b"treasury", Treasury);
			// add_benchmark!(params, batches, b"vesting", Vesting);
			// add_benchmark!(params, batches, b"democracy", Democracy);
			// add_benchmark!(params, batches, b"collective", Council);
			// if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			// Ok(batches)

			unimplemented!()
		}
	}
}
