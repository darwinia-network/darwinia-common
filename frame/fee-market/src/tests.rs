// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- substrate ---
use bp_messages::{
	source_chain::{
		LaneMessageVerifier, MessageDeliveryAndDispatchPayment, Sender, TargetHeaderChain,
	},
	target_chain::{
		DispatchMessage, MessageDispatch, ProvedLaneMessages, ProvedMessages, SourceHeaderChain,
	},
	DeliveredMessages, InboundLaneData, LaneId, Message, MessageNonce, OutboundLaneData,
	Parameter as MessagesParameter, UnrewardedRelayer, UnrewardedRelayersState,
};
use bp_runtime::{messages::MessageDispatchResult, Size};
use frame_support::{
	assert_err, assert_ok,
	traits::{GenesisBuild, LockIdentifier},
	weights::{RuntimeDbWeight, Weight},
	PalletId,
};
use frame_system::mocking::*;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	FixedU128, Permill, RuntimeDebug,
};
// --- std ---
use bitvec::prelude::*;
use std::{collections::VecDeque, ops::RangeInclusive};
// --- darwinia-network ---
use crate::payment::{slash_order_assigned_relayers, RewardsBook};
use crate::{self as darwinia_fee_market, *};

pub type Block = MockBlock<Test>;
pub type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
pub type Balance = u64;
pub type AccountId = u64;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight { read: 1, write: 2 };
}
impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = DbWeight;
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

// >>> Start mock pallet-bridges-message config data

pub type TestMessageFee = u64;
pub type TestRelayer = u64;
/// Lane that we're using in tests.
pub const TEST_LANE_ID: LaneId = [0, 0, 0, 1];
/// Error that is returned by all test implementations.
pub const TEST_ERROR: &str = "Test error";
/// Account id of test relayer.
pub const TEST_RELAYER_A: AccountId = 100;
/// Account id of additional test relayer - B.
pub const TEST_RELAYER_B: AccountId = 101;
/// Payload that is rejected by `TestTargetHeaderChain`.
pub const PAYLOAD_REJECTED_BY_TARGET_CHAIN: TestPayload = message_payload(1, 50);
/// Regular message payload.
pub const REGULAR_PAYLOAD: TestPayload = message_payload(0, 50);
/// Vec of proved messages, grouped by lane.
pub type MessagesByLaneVec = Vec<(LaneId, ProvedLaneMessages<Message<TestMessageFee>>)>;

#[derive(Decode, Encode, Clone, Debug, PartialEq, Eq)]
pub struct TestPayload {
	/// Field that may be used to identify messages.
	pub id: u64,
	/// Dispatch weight that is declared by the message sender.
	pub declared_weight: Weight,
	/// Message dispatch result.
	///
	/// Note: in correct code `dispatch_result.unspent_weight` will always be <= `declared_weight`,
	/// but for test purposes we'll be making it larger than `declared_weight` sometimes.
	pub dispatch_result: MessageDispatchResult,
	/// Extra bytes that affect payload size.
	pub extra: Vec<u8>,
}
impl Size for TestPayload {
	fn size_hint(&self) -> u32 {
		16 + self.extra.len() as u32
	}
}
/// Constructs message payload using given arguments and zero unspent weight.
pub const fn message_payload(id: u64, declared_weight: Weight) -> TestPayload {
	TestPayload {
		id,
		declared_weight,
		dispatch_result: dispatch_result(0),
		extra: Vec::new(),
	}
}

/// Test messages proof.
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
pub struct TestMessagesProof {
	pub result: Result<MessagesByLaneVec, ()>,
}
impl Size for TestMessagesProof {
	fn size_hint(&self) -> u32 {
		0
	}
}

/// Messages delivery proof used in tests.
#[derive(Debug, Encode, Decode, Eq, Clone, PartialEq)]
pub struct TestMessagesDeliveryProof(pub Result<(LaneId, InboundLaneData<TestRelayer>), ()>);
impl Size for TestMessagesDeliveryProof {
	fn size_hint(&self) -> u32 {
		0
	}
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum TestMessagesParameter {
	TokenConversionRate(FixedU128),
}
impl MessagesParameter for TestMessagesParameter {
	fn save(&self) {
		match *self {
			TestMessagesParameter::TokenConversionRate(conversion_rate) => {
				TokenConversionRate::set(&conversion_rate)
			}
		}
	}
}

/// Target header chain that is used in tests.
#[derive(Debug, Default)]
pub struct TestTargetHeaderChain;
impl TargetHeaderChain<TestPayload, TestRelayer> for TestTargetHeaderChain {
	type Error = &'static str;

	type MessagesDeliveryProof = TestMessagesDeliveryProof;

	fn verify_message(payload: &TestPayload) -> Result<(), Self::Error> {
		if *payload == PAYLOAD_REJECTED_BY_TARGET_CHAIN {
			Err(TEST_ERROR)
		} else {
			Ok(())
		}
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<TestRelayer>), Self::Error> {
		proof.0.map_err(|_| TEST_ERROR)
	}
}

/// Lane message verifier that is used in tests.
#[derive(Debug, Default)]
pub struct TestLaneMessageVerifier;
impl LaneMessageVerifier<AccountId, TestPayload, TestMessageFee> for TestLaneMessageVerifier {
	type Error = &'static str;

	fn verify_message(
		_submitter: &Sender<AccountId>,
		delivery_and_dispatch_fee: &TestMessageFee,
		_lane: &LaneId,
		_lane_outbound_data: &OutboundLaneData,
		_payload: &TestPayload,
	) -> Result<(), Self::Error> {
		if *delivery_and_dispatch_fee != 0 {
			Ok(())
		} else {
			Err(TEST_ERROR)
		}
	}
}

/// Message fee payment system that is used in tests.
#[derive(Debug, Default)]
pub struct TestMessageDeliveryAndDispatchPayment;
impl TestMessageDeliveryAndDispatchPayment {
	/// Reject all payments.
	pub fn reject_payments() {
		frame_support::storage::unhashed::put(b":reject-message-fee:", &true);
	}

	/// Returns true if given fee has been paid by given submitter.
	pub fn is_fee_paid(submitter: AccountId, fee: TestMessageFee) -> bool {
		frame_support::storage::unhashed::get(b":message-fee:")
			== Some((Sender::Signed(submitter), fee))
	}

	/// Returns true if given relayer has been rewarded with given balance. The reward-paid flag is
	/// cleared after the call.
	pub fn is_reward_paid(relayer: AccountId, fee: TestMessageFee) -> bool {
		let key = (b":relayer-reward:", relayer, fee).encode();
		frame_support::storage::unhashed::take::<bool>(&key).is_some()
	}
}
impl MessageDeliveryAndDispatchPayment<AccountId, TestMessageFee>
	for TestMessageDeliveryAndDispatchPayment
{
	type Error = &'static str;

	fn pay_delivery_and_dispatch_fee(
		submitter: &Sender<AccountId>,
		fee: &TestMessageFee,
		_relayer_fund_account: &AccountId,
	) -> Result<(), Self::Error> {
		if frame_support::storage::unhashed::get(b":reject-message-fee:") == Some(true) {
			return Err(TEST_ERROR);
		}

		frame_support::storage::unhashed::put(b":message-fee:", &(submitter, fee));
		Ok(())
	}

	fn pay_relayers_rewards(
		_lane_id: LaneId,
		message_relayers: VecDeque<UnrewardedRelayer<AccountId>>,
		confirmation_relayer: &AccountId,
		_received_range: &RangeInclusive<MessageNonce>,
		relayer_fund_account: &AccountId,
	) {
		let RewardsBook {
			messages_relayers_rewards,
			confirmation_relayer_rewards,
			assigned_relayers_rewards,
			treasury_total_rewards,
		} = crate::payment::cal_rewards::<Test, ()>(message_relayers, relayer_fund_account);
		let confimation_key = (
			b":relayer-reward:",
			confirmation_relayer,
			confirmation_relayer_rewards,
		)
			.encode();
		frame_support::storage::unhashed::put(&confimation_key, &true);

		for (relayer, reward) in &messages_relayers_rewards {
			let key = (b":relayer-reward:", relayer, reward).encode();
			frame_support::storage::unhashed::put(&key, &true);
		}

		for (relayer, reward) in &assigned_relayers_rewards {
			let key = (b":relayer-reward:", relayer, reward).encode();
			frame_support::storage::unhashed::put(&key, &true);
		}

		let treasury_account: AccountId = <Test as Config>::TreasuryPalletId::get().into_account();
		let treasury_key = (
			b":relayer-reward:",
			&treasury_account,
			treasury_total_rewards,
		)
			.encode();
		frame_support::storage::unhashed::put(&treasury_key, &true);
	}
}
/// Source header chain that is used in tests.
#[derive(Debug)]
pub struct TestSourceHeaderChain;
impl SourceHeaderChain<TestMessageFee> for TestSourceHeaderChain {
	type Error = &'static str;

	type MessagesProof = TestMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		_messages_count: u32,
	) -> Result<ProvedMessages<Message<TestMessageFee>>, Self::Error> {
		proof
			.result
			.map(|proof| proof.into_iter().collect())
			.map_err(|_| TEST_ERROR)
	}
}

/// Source header chain that is used in tests.
#[derive(Debug)]
pub struct TestMessageDispatch;
impl MessageDispatch<AccountId, TestMessageFee> for TestMessageDispatch {
	type DispatchPayload = TestPayload;

	fn dispatch_weight(message: &DispatchMessage<TestPayload, TestMessageFee>) -> Weight {
		match message.data.payload.as_ref() {
			Ok(payload) => payload.declared_weight,
			Err(_) => 0,
		}
	}

	fn dispatch(
		_relayer_account: &AccountId,
		message: DispatchMessage<TestPayload, TestMessageFee>,
	) -> MessageDispatchResult {
		match message.data.payload.as_ref() {
			Ok(payload) => payload.dispatch_result.clone(),
			Err(_) => dispatch_result(0),
		}
	}
}

pub struct AccountIdConverter;
impl sp_runtime::traits::Convert<H256, AccountId> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId {
		hash.to_low_u64_ne()
	}
}

// >>> End mock pallet-bridges-message config data

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: u64 = 10;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: u64 = 16;
	pub const MaxUnconfirmedMessagesAtInboundLane: u64 = 32;
	pub storage TokenConversionRate: FixedU128 = 1.into();
	pub const TestBridgedChainId: bp_runtime::ChainId = *b"test";
}

impl pallet_bridge_messages::Config for Test {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = TestMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = TestPayload;
	type OutboundMessageFee = TestMessageFee;

	type InboundPayload = TestPayload;
	type InboundMessageFee = TestMessageFee;
	type InboundRelayer = TestRelayer;

	type AccountIdConverter = AccountIdConverter;

	type TargetHeaderChain = TestTargetHeaderChain;
	type LaneMessageVerifier = TestLaneMessageVerifier;
	type MessageDeliveryAndDispatchPayment = TestMessageDeliveryAndDispatchPayment;
	type OnMessageAccepted = MessageAcceptedHandler<Self>;
	type OnDeliveryConfirmed = MessageConfirmedHandler<Self>;

	type SourceHeaderChain = TestSourceHeaderChain;
	type MessageDispatch = TestMessageDispatch;
	type BridgedChainId = TestBridgedChainId;
}

frame_support::parameter_types! {
	pub const FeeMarketPalletId: PalletId = PalletId(*b"da/feemk");
	pub const TreasuryPalletId: PalletId = PalletId(*b"da/trsry");
	pub const MiniumLockCollateral: Balance = 100;
	pub const MinimumRelayFee: Balance = 30;
	pub const FeeMarketLockId: LockIdentifier = *b"da/feelf";
	pub const SlotTimes: (u64, u64, u64) = (50, 50, 50);

	pub const ForAssignedRelayers: Permill = Permill::from_percent(60);
	pub const ForMessageRelayer: Permill = Permill::from_percent(80);
	pub const ForConfirmRelayer: Permill = Permill::from_percent(20);
	pub const TreasuryPalletAccount: u64 = 666;
}
impl Config for Test {
	type PalletId = FeeMarketPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type MiniumLockCollateral = MiniumLockCollateral;
	type MinimumRelayFee = MinimumRelayFee;
	type LockId = FeeMarketLockId;
	type SlotTimes = SlotTimes;

	type ForAssignedRelayers = ForAssignedRelayers;
	type ForMessageRelayer = ForMessageRelayer;
	type ForConfirmRelayer = ForConfirmRelayer;
	type AssignedRelayersAbsentSlash = ();

	type RingCurrency = Ring;
	type Event = Event;
	type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		FeeMarket: darwinia_fee_market::{Pallet, Call, Storage, Event<T>},
		Messages: pallet_bridge_messages::{Pallet, Call, Event<T>},
	}
}
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	darwinia_balances::GenesisConfig::<Test, RingInstance> {
		balances: vec![(1, 150), (2, 200), (3, 350), (4, 220), (5, 350), (12, 400)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Returns message dispatch result with given unspent weight.
pub const fn dispatch_result(unspent_weight: Weight) -> MessageDispatchResult {
	MessageDispatchResult {
		dispatch_result: true,
		unspent_weight,
		dispatch_fee_paid_during_dispatch: true,
	}
}

/// Constructs unrewarded relayer entry from nonces range and relayer id.
pub fn unrewarded_relayer(
	begin: MessageNonce,
	end: MessageNonce,
	relayer: TestRelayer,
) -> UnrewardedRelayer<TestRelayer> {
	UnrewardedRelayer {
		relayer,
		messages: DeliveredMessages {
			begin,
			end,
			dispatch_results: if end >= begin {
				bitvec![Msb0, u8; 1; (end - begin + 1) as _]
			} else {
				Default::default()
			},
		},
	}
}

#[test]
fn test_single_relayer_registration_workflow_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Ring::free_balance(1), 150);
		assert_err!(
			FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 1, None),
			<Error<Test>>::LockCollateralTooLow
		);
		assert_err!(
			FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 200, None),
			<Error<Test>>::InsufficientBalance
		);

		assert_ok!(FeeMarket::enroll_and_lock_collateral(
			Origin::signed(1),
			100,
			None
		));
		assert!(FeeMarket::is_enrolled(&1));
		assert_eq!(FeeMarket::relayers().len(), 1);
		assert_eq!(Ring::usable_balance(&1), 50);
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 100);
		assert_eq!(FeeMarket::market_relayer_fee(), None);

		assert_err!(
			FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, None),
			<Error<Test>>::AlreadyEnrolled
		);
	});
}
#[test]
fn test_single_relayer_update_lock_collateral() {
	new_test_ext().execute_with(|| {
		assert_err!(
			FeeMarket::update_locked_collateral(Origin::signed(1), 120),
			<Error::<Test>>::NotEnrolled
		);
		assert_ok!(FeeMarket::enroll_and_lock_collateral(
			Origin::signed(1),
			100,
			None
		));
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 100);

		// Increase locked balance
		assert_ok!(FeeMarket::update_locked_collateral(Origin::signed(1), 120));
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 120);
		// Decrease locked balance
		assert_err!(
			FeeMarket::update_locked_collateral(Origin::signed(1), 100),
			<Error<Test>>::OnlyIncreaseLockedCollateralAllowed
		);
	});
}

#[test]
fn test_single_relayer_cancel_registration() {
	new_test_ext().execute_with(|| {
		assert_err!(
			FeeMarket::cancel_enrollment(Origin::signed(1)),
			<Error<Test>>::NotEnrolled
		);

		assert_ok!(FeeMarket::enroll_and_lock_collateral(
			Origin::signed(1),
			100,
			None
		));
		assert!(FeeMarket::is_enrolled(&1));
		assert_err!(
			FeeMarket::cancel_enrollment(Origin::signed(1)),
			<Error<Test>>::TooFewEnrolledRelayers
		);
		assert!(FeeMarket::is_enrolled(&1));
	});
}

#[test]
fn test_single_relayer_update_fee_works() {
	new_test_ext().execute_with(|| {
		assert_err!(
			FeeMarket::update_relay_fee(Origin::signed(1), 1),
			<Error<Test>>::NotEnrolled
		);
		assert_ok!(FeeMarket::enroll_and_lock_collateral(
			Origin::signed(1),
			100,
			None
		));
		assert_err!(
			FeeMarket::update_relay_fee(Origin::signed(1), 1),
			<Error<Test>>::RelayFeeTooLow
		);

		assert_eq!(FeeMarket::relayer_fee(&1), 30);
		assert_ok!(FeeMarket::update_relay_fee(Origin::signed(1), 40));
		assert_eq!(FeeMarket::relayer_fee(&1), 40);
	});
}

#[test]
fn test_multiple_relayers_registration_with_same_lock_value_and_default_fee() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 100, None);

		assert_eq!(FeeMarket::relayers(), vec![1, 2, 3, 4]);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 30));
	});
}

#[test]
fn test_multiple_relayers_cancel_registration() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(5), 100, None);
		assert_eq!(FeeMarket::relayers(), vec![1, 2, 3, 4, 5]);
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 100);
		assert_eq!(FeeMarket::relayer_locked_collateral(&2), 100);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 30));

		assert_ok!(FeeMarket::cancel_enrollment(Origin::signed(1)));
		assert_ok!(FeeMarket::cancel_enrollment(Origin::signed(5)));
		assert!(!FeeMarket::is_enrolled(&1));
		assert!(!FeeMarket::is_enrolled(&5));
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 0);
		assert_eq!(FeeMarket::relayer_locked_collateral(&5), 0);
		assert_eq!(FeeMarket::relayers(), vec![2, 3, 4]);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (4, 30));

		assert_err!(
			FeeMarket::cancel_enrollment(Origin::signed(2)),
			<Error<Test>>::TooFewEnrolledRelayers
		);
	});
}

#[test]
fn test_multiple_relayers_sort() {
	new_test_ext().execute_with(|| {
		let r1 = Relayer::<AccountId, Balance>::new(1, 100, 30);
		let r2 = Relayer::<AccountId, Balance>::new(2, 100, 40);
		assert!(r1 < r2);

		let r3 = Relayer::<AccountId, Balance>::new(3, 150, 30);
		let r4 = Relayer::<AccountId, Balance>::new(4, 100, 30);
		assert!(r3 < r4);
	});
}

#[test]
fn test_multiple_relayers_choose_assigned_relayers_with_same_default_fee() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 130, None);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(5), 140, None);

		assert_eq!(FeeMarket::relayers().len(), 5);
		assert_eq!(
			FeeMarket::assigned_relayers().unwrap(),
			(
				Relayer::<AccountId, Balance>::new(5, 140, 30),
				Relayer::<AccountId, Balance>::new(4, 130, 30),
				Relayer::<AccountId, Balance>::new(3, 120, 30),
			)
		);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 30));
	});
}

#[test]
fn test_multiple_relayers_choose_assigned_relayers_with_same_lock_balance() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 100, Some(40));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 100, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 100, Some(60));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(5), 100, Some(70));

		assert_eq!(FeeMarket::relayers().len(), 5);
		assert_eq!(
			FeeMarket::assigned_relayers().unwrap(),
			(
				Relayer::<AccountId, Balance>::new(1, 100, 30),
				Relayer::<AccountId, Balance>::new(2, 100, 40),
				Relayer::<AccountId, Balance>::new(3, 100, 50),
			)
		);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 50));
	});
}

#[test]
#[test]
fn test_multiple_relayers_choose_assigned_relayers_with_diff_lock_and_fee() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 100, Some(40));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 100, Some(50));

		assert_eq!(FeeMarket::relayers().len(), 4);
		assert_eq!(
			FeeMarket::assigned_relayers().unwrap(),
			(
				Relayer::<AccountId, Balance>::new(1, 100, 30),
				Relayer::<AccountId, Balance>::new(2, 100, 40),
				Relayer::<AccountId, Balance>::new(3, 120, 50),
			)
		);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 50));
	});
}

fn send_regular_message(fee: Balance) -> (LaneId, u64) {
	let message_nonce = Messages::outbound_latest_generated_nonce(TEST_LANE_ID) + 1;
	assert_ok!(Messages::send_message(
		Origin::signed(1),
		TEST_LANE_ID,
		REGULAR_PAYLOAD,
		fee
	));

	(TEST_LANE_ID, message_nonce)
}

fn receive_messages_delivery_proof() {
	assert_ok!(Messages::receive_messages_delivery_proof(
		Origin::signed(1),
		TestMessagesDeliveryProof(Ok((
			TEST_LANE_ID,
			InboundLaneData {
				last_confirmed_nonce: 1,
				relayers: vec![UnrewardedRelayer {
					relayer: 0,
					messages: DeliveredMessages::new(1, true),
				}]
				.into_iter()
				.collect(),
			},
		))),
		UnrewardedRelayersState {
			unrewarded_relayer_entries: 1,
			total_messages: 1,
			..Default::default()
		},
	));
}

#[test]
fn test_order_creation_when_bridged_pallet_accept_message() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(100));
		System::set_block_number(2);

		let assigned_relayers = FeeMarket::assigned_relayers().unwrap();
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (lane, message_nonce) = send_regular_message(market_fee);
		let order = FeeMarket::order(&lane, &message_nonce).unwrap();
		let (relayer1, relayer2, relayer3) = order.assigned_relayers().unwrap();
		assert_eq!(relayer1.id, assigned_relayers.0.id);
		assert_eq!(relayer2.id, assigned_relayers.1.id);
		assert_eq!(relayer3.id, assigned_relayers.2.id);
		assert_eq!(order.sent_time, 2);
	});
}

#[test]
#[should_panic]
fn test_no_order_created_after_send_message_when_fee_market_not_ready() {
	new_test_ext().execute_with(|| {
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		System::set_block_number(2);

		assert!(FeeMarket::assigned_relayers().is_none());
		let (lane, message_nonce) = send_regular_message(80);
		assert!(FeeMarket::order(&lane, &message_nonce).is_none());
	});
}

#[test]
fn test_order_confirm_time_set_when_bridged_pallet_confirmed_message() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(100));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (lane, message_nonce) = send_regular_message(market_fee);
		let order = FeeMarket::order(&lane, &message_nonce).unwrap();
		assert_eq!(order.confirm_time, None);

		System::set_block_number(4);
		receive_messages_delivery_proof();
		let order = FeeMarket::order(&lane, &message_nonce).unwrap();
		assert_eq!(order.confirm_time, Some(4));
		assert_eq!(<ConfirmedMessagesThisBlock<Test>>::get().len(), 1);
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayer_finish_delivery_single_message() {
	new_test_ext().execute_with(|| {
		// Send message
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(100));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, _) = send_regular_message(market_fee);

		// Receive delivery message proof
		System::set_block_number(4);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));

		// Analysis:
		// 1. assigned_relayers [(1, 30, 2-52),(2, 50, 52-102),(3, 100, 102-152)] -> id: 1, reward = 60% * 30 = 18
		// 2. message relayer -> id: 100, reward = 40% * 30 * 80% = 9.6 ~ 10
		// 3. confirmation relayer -> id: 5, reward = 40% * 30 * 20% = 2.4 ~ 2
		// 4. treasury reward -> reward: 100 - 30 = 70
		let t: AccountId = <Test as Config>::TreasuryPalletId::get().into_account();
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(t, 70));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(1, 18));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 2));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			10
		));
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayer_finish_delivery_with_multiple_messages() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(300));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(500));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(1000));

		// Send message
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, message_nonce1) = send_regular_message(market_fee);
		let (_, message_nonce2) = send_regular_message(market_fee);
		assert_eq!(message_nonce1 + 1, message_nonce2);

		// Receive delivery message proof
		System::set_block_number(4);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![
						unrewarded_relayer(1, 1, TEST_RELAYER_A),
						unrewarded_relayer(2, 2, TEST_RELAYER_B)
					]
					.into_iter()
					.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 2,
				total_messages: 2,
				..Default::default()
			},
		));

		let t: AccountId = <Test as Config>::TreasuryPalletId::get().into_account();
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			t, 1400
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			1, 360
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 48));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			96
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_B,
			96
		));
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayer_single_message_with_multiple_duplicated_delivery_proof(
) {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(100));

		// Send message
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, _) = send_regular_message(market_fee);

		// The first time receive delivery message proof
		System::set_block_number(4);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));
		// The second time receive delivery message proof
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(6),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));

		assert_eq!(ConfirmedMessagesThisBlock::<Test>::get().len(), 1);
		let t: AccountId = <Test as Config>::TreasuryPalletId::get().into_account();
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(t, 70));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(1, 18));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 2));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			10
		));
	});
}

#[test]
fn test_assigned_relayers_absent_slash_calculation_below_min_lock_value() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(70));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (lane, message_nonce) = send_regular_message(market_fee);
		let order = FeeMarket::order(&lane, &message_nonce).unwrap();

		assert_eq!(
			slash_order_assigned_relayers::<Test>(0, order.assigned_relayers.clone(), &0),
			210
		);
		assert_eq!(
			slash_order_assigned_relayers::<Test>(5, order.assigned_relayers, &0),
			240
		);
	});
}

#[test]
fn test_assigned_relayers_absent_slash_calculation_exceed_min_lock_value() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(70));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (lane, message_nonce) = send_regular_message(market_fee);
		let order = FeeMarket::order(&lane, &message_nonce).unwrap();

		assert_eq!(
			slash_order_assigned_relayers::<Test>(14, order.assigned_relayers.clone(), &0),
			98 * 3
		);
		assert_eq!(
			slash_order_assigned_relayers::<Test>(15, order.assigned_relayers.clone(), &0),
			300
		);
		assert_eq!(
			slash_order_assigned_relayers::<Test>(50, order.assigned_relayers, &0),
			300
		);
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayers_absent_with_single_message() {
	new_test_ext().execute_with(|| {
		// Send message
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(100));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, _) = send_regular_message(market_fee);

		// Receive delivery message proof
		System::set_block_number(200);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 60));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			240
		));
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayers_absent_with_multiple_message() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(300));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(500));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(1000));

		// Send message
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, message_nonce1) = send_regular_message(market_fee);
		let (_, message_nonce2) = send_regular_message(market_fee);
		assert_eq!(message_nonce1 + 1, message_nonce2);

		// Receive delivery message proof
		System::set_block_number(200);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![
						unrewarded_relayer(1, 1, TEST_RELAYER_A),
						unrewarded_relayer(2, 2, TEST_RELAYER_B)
					]
					.into_iter()
					.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 2,
				total_messages: 2,
				..Default::default()
			},
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			5, 120
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			240
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_B,
			240
		));
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayers_absent_update_lock_balance() {
	new_test_ext().execute_with(|| {
		// Send message
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 150, Some(50));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 200, Some(100));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, _) = send_regular_message(market_fee);
		// The original (account-balance) map in genesis: [(1, 150), (2, 200), (3, 350)]
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 100);
		assert_eq!(FeeMarket::relayer_locked_collateral(&2), 150);
		assert_eq!(FeeMarket::relayer_locked_collateral(&3), 200);
		assert_eq!(Ring::usable_balance(&1), 50);
		assert_eq!(Ring::usable_balance(&2), 50);
		assert_eq!(Ring::usable_balance(&3), 150);
		assert_eq!(FeeMarket::relayers().len(), 3);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 100));

		// Receive delivery message proof
		System::set_block_number(200);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 60));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			240
		));
		// The p1, p2, p3 slash value is 100
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 50);
		assert_eq!(FeeMarket::relayer_locked_collateral(&2), 100);
		assert_eq!(FeeMarket::relayer_locked_collateral(&3), 200);
		assert_eq!(Ring::usable_balance(&1), 0);
		assert_eq!(Ring::usable_balance(&2), 0);
		assert_eq!(Ring::usable_balance(&3), 50);
		assert_eq!(FeeMarket::relayers().len(), 3);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (3, 100));
	});
}

#[test]
fn test_payment_reward_calculation_assigned_relayers_absent_update_assigned_relayers_list() {
	new_test_ext().execute_with(|| {
		// Send message
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 150, Some(30));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 130, Some(100));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(4), 150, Some(100));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(5), 100, Some(200));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(12), 150, Some(200));
		assert_eq!(FeeMarket::relayers().len(), 6);
		assert_eq!(
			FeeMarket::assigned_relayers().unwrap(),
			(
				Relayer::<AccountId, Balance>::new(2, 150, 30),
				Relayer::<AccountId, Balance>::new(1, 100, 30),
				Relayer::<AccountId, Balance>::new(4, 150, 100),
			)
		);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (4, 100));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (_, _) = send_regular_message(market_fee);
		// The original (account-balance) map in genesis: [(1, 150), (2, 200), (3, 350), (4, 300), (5, 350), (12, 400)]
		assert_eq!(FeeMarket::relayer_locked_collateral(&2), 150);
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 100);
		assert_eq!(FeeMarket::relayer_locked_collateral(&4), 150);
		assert_eq!(Ring::usable_balance(&2), 50);
		assert_eq!(Ring::usable_balance(&1), 50);
		assert_eq!(Ring::usable_balance(&4), 70);

		// Receive delivery message proof
		System::set_block_number(200);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![unrewarded_relayer(1, 1, TEST_RELAYER_A)]
						.into_iter()
						.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 1,
				total_messages: 1,
				..Default::default()
			},
		));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(5, 60));
		assert!(TestMessageDeliveryAndDispatchPayment::is_reward_paid(
			TEST_RELAYER_A,
			240
		));

		// The p1, p2, p3 slash value is 100
		assert_eq!(FeeMarket::relayer_locked_collateral(&2), 100);
		assert_eq!(FeeMarket::relayer_locked_collateral(&1), 0);
		assert_eq!(FeeMarket::relayer_locked_collateral(&4), 120);
		assert_eq!(Ring::usable_balance(&2), 0);
		assert_eq!(Ring::usable_balance(&1), 50);
		assert_eq!(Ring::usable_balance(&4), 0);
		assert_eq!(FeeMarket::relayers().len(), 5);
		assert_eq!(
			FeeMarket::assigned_relayers().unwrap(),
			(
				Relayer::<AccountId, Balance>::new(2, 100, 30),
				Relayer::<AccountId, Balance>::new(3, 130, 100),
				Relayer::<AccountId, Balance>::new(4, 120, 100),
			)
		);
		assert_eq!(FeeMarket::market_relayer_fee().unwrap(), (4, 100));
		assert!(!FeeMarket::is_enrolled(&1));
	});
}

#[test]
fn test_clean_order_state_at_the_end_of_block() {
	new_test_ext().execute_with(|| {
		System::set_block_number(2);
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(1), 100, Some(300));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(2), 110, Some(500));
		let _ = FeeMarket::enroll_and_lock_collateral(Origin::signed(3), 120, Some(1000));
		let market_fee = FeeMarket::market_relayer_fee().unwrap().1;
		let (lane1, nonce1) = send_regular_message(market_fee);
		let (lane2, nonce2) = send_regular_message(market_fee);
		System::set_block_number(3);
		let (lane3, nonce3) = send_regular_message(market_fee);
		let (lane4, nonce4) = send_regular_message(market_fee);

		System::set_block_number(10);
		assert_ok!(Messages::receive_messages_delivery_proof(
			Origin::signed(5),
			TestMessagesDeliveryProof(Ok((
				TEST_LANE_ID,
				InboundLaneData {
					relayers: vec![
						unrewarded_relayer(1, 2, TEST_RELAYER_A),
						unrewarded_relayer(3, 4, TEST_RELAYER_B)
					]
					.into_iter()
					.collect(),
					..Default::default()
				}
			))),
			UnrewardedRelayersState {
				unrewarded_relayer_entries: 2,
				total_messages: 4,
				..Default::default()
			},
		));
		assert_eq!(ConfirmedMessagesThisBlock::<Test>::get().len(), 4);
		assert!(FeeMarket::order(&lane1, &nonce1).is_some());
		assert!(FeeMarket::order(&lane2, &nonce2).is_some());
		assert!(FeeMarket::order(&lane3, &nonce3).is_some());
		assert!(FeeMarket::order(&lane4, &nonce4).is_some());

		// Check in next block
		FeeMarket::on_finalize(10);
		System::set_block_number(1);
		assert_eq!(ConfirmedMessagesThisBlock::<Test>::get().len(), 0);
		assert!(FeeMarket::order(&lane1, &nonce1).is_none());
		assert!(FeeMarket::order(&lane2, &nonce2).is_none());
		assert!(FeeMarket::order(&lane3, &nonce3).is_none());
		assert!(FeeMarket::order(&lane4, &nonce4).is_none());
	});
}
