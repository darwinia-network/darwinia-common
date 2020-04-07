//! The Darwinia Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub pallet_aura: Aura,
			pub pallet_grandpa: Grandpa,
		}
	}
}

pub mod support_kton_in_the_future {
	use sp_runtime::traits::Convert;

	use crate::*;

	/// Struct that handles the conversion of Balance -> `u64`. This is used for staking's election
	/// calculation.
	pub struct CurrencyToVoteHandler;

	impl CurrencyToVoteHandler {
		fn factor() -> Balance {
			(Ring::total_issuance() / u64::max_value() as Balance).max(1)
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
}

mod impls;

// --- substrate ---
// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types, traits::Randomness, weights::Weight, StorageValue,
};
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{traits::OpaqueKeys, Perbill, Percent, Permill};
// --- darwinia ---
pub use darwinia_staking::StakerStatus;

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::debug;
use frame_system::offchain::TransactionSubmitter;
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	u32_trait::{_2, _3, _4},
	OpaqueMetadata,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, ConvertInto, IdentifyAccount, IdentityLookup,
		SaturatedConversion, Verify,
	},
	transaction_validity::TransactionValidity,
	ApplyExtrinsicResult, MultiSignature, RuntimeDebug,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia ---
use impls::*;

/// An index to a block.
pub type BlockNumber = u32;

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
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Power of an account.
pub type Power = u32;

/// Alias Balances Module as Ring Module.
pub type Ring = Balances;

/// Alias Balances1 Module as Kton Module.
pub type Kton = Balances1;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-template"),
	impl_name: create_runtime_str!("node-template"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
};

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const BLOCKS_PER_SESSION: BlockNumber = 10 * MINUTES;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(BLOCKS_PER_SESSION as f64 * SLOT_FILL_RATE) as u64
};
pub const SESSION_DURATION: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
pub const SESSIONS_PER_ERA: sp_staking::SessionIndex = 6;

pub const CAP: Balance = 1_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1_000_000_000;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
}
impl frame_system::Trait for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<Self::AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type ModuleToIndex = ModuleToIndex;
	/// What to do if a new account is created.
	type MigrateAccount = ();
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the frame_system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = AccountData<Balance>;
}

impl pallet_aura::Trait for Runtime {
	type AuthorityId = AuraId;
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}
impl pallet_timestamp::Trait for Runtime {
	/// A pallet_timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub aura: Aura,
		pub im_online: ImOnline,
	}
}
parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
	pub const PERIOD: BlockNumber = BLOCKS_PER_SESSION;
	pub const OFFSET: BlockNumber = BLOCKS_PER_SESSION;
}
impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = darwinia_staking::StashOf<Self>;
	type ShouldEndSession = pallet_session::PeriodicSessions<PERIOD, OFFSET>;
	type SessionManager = Staking;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
}
type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
}

/// A runtime transaction submitter.
type SubmitTransaction = TransactionSubmitter<ImOnlineId, Runtime, UncheckedExtrinsic>;
parameter_types! {
	pub const SessionDuration: BlockNumber = SESSION_DURATION;
}
impl pallet_im_online::Trait for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type Call = Call;
	type SubmitTransaction = SubmitTransaction;
	type SessionDuration = SessionDuration;
	type ReportUnresponsiveness = Offences;
}

impl pallet_offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification = darwinia_staking::Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = darwinia_staking::ExposureOf<Runtime>;
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * COIN;
}
type RingInstance = darwinia_balances::Instance0;
impl darwinia_balances::Trait<RingInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type BalanceInfo = AccountData<Balance>;
	type DustCollector = (Kton,);
}
type KtonInstance = darwinia_balances::Instance1;
impl darwinia_balances::Trait<KtonInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type BalanceInfo = AccountData<Balance>;
	type DustCollector = (Ring,);
}

parameter_types! {
	pub const Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}
impl darwinia_claims::Trait for Runtime {
	type Event = Event;
	type Prefix = Prefix;
	type RingCurrency = Ring;
}

parameter_types! {
	pub const CandidacyBond: Balance = 10 * COIN;
	pub const VotingBond: Balance = 1 * COIN;
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
}
impl darwinia_elections_phragmen::Trait for Runtime {
	type Event = Event;
	type Currency = Ring;
	type ChangeMembers = Council;
	type CurrencyToVote = support_kton_in_the_future::CurrencyToVoteHandler;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type LoserCandidate = ();
	type BadReport = ();
	type KickedMember = ();
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
}

parameter_types! {
	pub const EthNetwork: darwinia_eth_relay::EthNetworkType = darwinia_eth_relay::EthNetworkType::Ropsten;
}
impl darwinia_eth_relay::Trait for Runtime {
	type Event = Event;
	type EthNetwork = EthNetwork;
}

parameter_types! {
	pub const SubKeyPrefix: u8 = 42;
}
impl darwinia_eth_backing::Trait for Runtime {
	type Event = Event;
	type Time = Timestamp;
	type DetermineAccountId = darwinia_eth_backing::AccountIdDeterminator<Runtime>;
	type EthRelay = EthRelay;
	type OnDepositRedeem = Staking;
	type Ring = Ring;
	type RingReward = ();
	type Kton = Kton;
	type KtonReward = ();
	type SubKeyPrefix = SubKeyPrefix;
}

impl frame_system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;

	fn create_transaction<
		TSigner: frame_system::offchain::Signer<Self::Public, Self::Signature>,
	>(
		call: Call,
		public: Self::Public,
		account: AccountId,
		index: Index,
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
		let extra: SignedExtra = (
			frame_system::CheckVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(index),
			frame_system::CheckWeight::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				debug::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = TSigner::sign(public, &raw_payload)?;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (account, signature, extra)))
	}
}
type SubmitPFTransaction =
	TransactionSubmitter<darwinia_eth_offchain::crypto::Public, Runtime, UncheckedExtrinsic>;
parameter_types! {
	pub const FetchInterval: BlockNumber = 3;
	// TODO: pass this from command line
	// this a poc versiona, build with following command to launch the poc binary
	// `ETHER_SCAN_API_KEY=XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX cargo build`
	pub const EtherScanAPIKey: Option<Vec<u8>> = match option_env!("ETHER_SCAN_API_KEY"){
		Some(s) => Some(s.as_bytes().to_vec()),
		None => None,
	};
}
impl darwinia_eth_offchain::Trait for Runtime {
	type Event = Event;
	type Time = Timestamp;
	type Call = Call;
	type SubmitSignedTransaction = SubmitPFTransaction;
	type FetchInterval = FetchInterval;
	type EtherScanAPIKey = EtherScanAPIKey;
}

impl darwinia_header_mmr::Trait for Runtime {}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDurationInEra: darwinia_staking::EraIndex = 14 * 24 * (HOURS / (SESSIONS_PER_ERA * BLOCKS_PER_SESSION));
	pub const BondingDurationInBlockNumber: BlockNumber = 14 * DAYS;
	pub const SlashDeferDuration: darwinia_staking::EraIndex = 7 * 24; // 1/4 the bonding duration.
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	// --- custom ---
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl darwinia_staking::Trait for Runtime {
	type Time = Timestamp;
	type Event = Event;
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin =
		pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
	type SessionInterface = Self;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
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

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const RingProposalBondMinimum: Balance = 1 * COIN;
	pub const KtonProposalBondMinimum: Balance = 1 * COIN;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * COIN;
	pub const TipReportDepositPerByte: Balance = 1 * MILLI;
}
impl darwinia_treasury::Trait for Runtime {
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type ApproveOrigin = pallet_collective::EnsureMembers<_4, AccountId, CouncilCollective>;
	type RejectOrigin = pallet_collective::EnsureMembers<_2, AccountId, CouncilCollective>;
	type Tippers = Elections;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = Event;
	type RingProposalRejection = ();
	type KtonProposalRejection = ();
	type ProposalBond = ProposalBond;
	type RingProposalBondMinimum = RingProposalBondMinimum;
	type KtonProposalBondMinimum = KtonProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * COIN;
}
impl darwinia_vesting::Trait for Runtime {
	type Event = Event;
	type Currency = Ring;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
}

construct_runtime!(
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Aura: pallet_aura::{Module, Config<T>, Inherent(Timestamp)},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
		Session: pallet_session::{Module, Call, Storage, Config<T>, Event},
		Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Config<T>, Event<T>},
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
		ImOnline: pallet_im_online::{Module, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
		Offences: pallet_offences::{Module, Call, Storage, Event},
		// Custom Module
		Balances: darwinia_balances::<Instance0>::{Module, Call, Storage, Config<T>, Event<T>},
		Balances1: darwinia_balances::<Instance1>::{Module, Call, Storage, Config<T>, Event<T>},
		Claims: darwinia_claims::{Module, Call, Storage, Config, Event<T>, ValidateUnsigned},
		Elections: darwinia_elections_phragmen::{Module, Call, Storage, Event<T>},
		EthBacking: darwinia_eth_backing::{Module, Call, Storage, Config<T>, Event<T>},
		EthRelay: darwinia_eth_relay::{Module, Call, Storage, Config<T>, Event<T>},
		EthOffchain: darwinia_eth_offchain::{Module, Call, Storage, Event<T>},
		HeaderMMR: darwinia_header_mmr::{Module, Call, Storage},
		Staking: darwinia_staking::{Module, Call, Storage, Config<T>, Event<T>},
		Treasury: darwinia_treasury::{Module, Call, Storage, Event<T>},
		Vesting: darwinia_vesting::{Module, Call, Storage, Config<T>, Event<T>},
	}
);

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
	frame_system::CheckVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Index, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
>;

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

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
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
		fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			Executive::validate_transaction(tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}
	}
}
