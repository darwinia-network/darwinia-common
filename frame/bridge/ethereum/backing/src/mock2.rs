//! Mock file for ethereum-backing.

// --- std ---
use std::cell::RefCell;
// --- substrate ---
use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use frame_system::EnsureRoot;
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
	ModuleId, {KeyTypeId, MultiSignature, Perbill},
};
// --- darwinia ---
use array_bytes::fixed_hex_bytes_unchecked;
use darwinia_staking::{EraIndex, Exposure, ExposureOf};
use darwinia_relay_primitives::*;
use darwinia_ethereum_relay::{EthereumHeaderThingWithProof, EthereumHeaderThing};

use crate::*;

type Balance = u128;
type BlockNumber = u64;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
type Signature = MultiSignature;

type Extrinsic = TestXt<Call, ()>;

pub type RingInstance = darwinia_balances::Instance0;
type _RingError = darwinia_balances::Error<Test, RingInstance>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

pub type KtonInstance = darwinia_balances::Instance1;
type _KtonError = darwinia_balances::Error<Test, KtonInstance>;
pub type Kton = darwinia_balances::Module<Test, KtonInstance>;

type Session = pallet_session::Module<Test>;
type System = frame_system::Module<Test>;
type Timestamp = pallet_timestamp::Module<Test>;
pub type EthereumRelay = darwinia_ethereum_relay::Module<Test>;
pub type Staking = darwinia_staking::Module<Test>;
pub type EthereumBacking = Module<Test>;

thread_local! {
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
	static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
}

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		darwinia_ethereum_relay::EthereumRelay,
		darwinia_staking::Staking,
	}
}

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

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId, Ks)],
		_queued_validators: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_validator_index: usize) {}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const EthBackingModuleId: ModuleId = ModuleId(*b"da/backi");
}
impl Trait for Test {
	type ModuleId = EthBackingModuleId;
	type Event = ();
	type RedeemAccountId = AccountId;
	type EthereumRelay = EthereumRelay;
	type OnDepositRedeem = Staking;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl pallet_timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const Period: BlockNumber = 1;
	pub const Offset: BlockNumber = 0;
}
impl pallet_session::Trait for Test {
	type Event = ();
	type ValidatorId = AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = TestSessionHandler;
	type Keys = UintAuthorityId;
	type DisabledValidatorsThreshold = ();
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

parameter_types! {
	pub const EthereumRelayModuleId: ModuleId = ModuleId(*b"da/ethrl");
}
impl darwinia_ethereum_relay::Trait for Test {
	type ModuleId = EthereumRelayModuleId;
	type Event = ();
	type RelayerGame = UnusedRelayerGame;
	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EnsureRoot<AccountId>;
	type Call = Call;
	type Currency = Ring;
	type WeightInfo = ();
}

impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = ();
	type WeightInfo = ();
}
impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = ();
	type WeightInfo = ();
}

impl darwinia_staking::Trait for Test {
	type Event = ();
	type UnixTime = Timestamp;
	type SessionsPerEra = ();
	type BondingDurationInEra = ();
	type BondingDurationInBlockNumber = ();
	type SlashDeferDuration = ();
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type ElectionLookahead = ();
	type Call = Call;
	type MaxIterations = ();
	type MinSolutionScoreBump = ();
	type MaxNominatorRewardedPerValidator = ();
	type UnsignedPriority = ();
	type RingCurrency = Ring;
	type RingRewardRemainder = ();
	type RingSlash = ();
	type RingReward = ();
	type KtonCurrency = Kton;
	type KtonSlash = ();
	type KtonReward = ();
	type Cap = ();
	type TotalPower = ();
	type WeightInfo = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
	fn default() -> Self {
		Self
	}
}
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		GenesisConfig::<Test> {
			token_redeem_address: fixed_hex_bytes_unchecked!(
				"0x49262B932E439271d05634c32978294C7Ea15d0C",
				20
			)
			.into(),
			deposit_redeem_address: fixed_hex_bytes_unchecked!(
				"0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e",
				20
			)
			.into(),
			ring_token_address: fixed_hex_bytes_unchecked!(
				"0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
				20
			).into(),
			kton_token_address: fixed_hex_bytes_unchecked!(
				"0x1994100c58753793D52c6f457f189aa3ce9cEe94",
				20
			).into(),
			ring_locked: 20000000000000,
			kton_locked: 5000000000000,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

pub struct UnusedRelayerGame;
impl RelayerGameProtocol for UnusedRelayerGame {
	type Relayer = AccountId;
	type Balance = Balance;
	type HeaderThingWithProof = EthereumHeaderThingWithProof;
	type HeaderThing = EthereumHeaderThing;

	fn proposals_of_game(
		_: <Self::HeaderThing as HeaderThing>::Number,
	) -> Vec<
		RelayProposal<
			Self::Relayer,
			Self::Balance,
			Self::HeaderThing,
			<Self::HeaderThing as HeaderThing>::Hash,
		>,
	> {
		unimplemented!()
	}

	fn submit_proposal(_: Self::Relayer, _: Vec<Self::HeaderThingWithProof>) -> DispatchResult {
		unimplemented!()
	}
	fn approve_pending_header(_: <Self::HeaderThing as HeaderThing>::Number) -> DispatchResult {
		unimplemented!()
	}
	fn reject_pending_header(_: <Self::HeaderThing as HeaderThing>::Number) -> DispatchResult {
		unimplemented!()
	}
}

