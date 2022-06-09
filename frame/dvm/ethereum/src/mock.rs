// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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

//! Test utilities

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use ethereum::{TransactionAction, TransactionSignature};
use rlp::RlpStream;
use scale_info::TypeInfo;
use sha3::{Digest, Keccak256};
// --- paritytech ---
use fp_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	traits::{Everything, FindAuthor, GenesisBuild},
	weights::GetDispatchInfo,
	ConsensusEngineId, PalletId,
};
use frame_system::mocking::*;
use pallet_evm::FeeCalculator;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::{self as darwinia_ethereum, account_basic::*, *};
use darwinia_evm::{runner::stack::Runner, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_support::evm::IntoAccountId;

type Block = MockBlock<Test>;
pub type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u64;

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId32;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

frame_support::parameter_types! {
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 10;
	pub const ExistentialDeposit: u64 = 500;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 6000 / 2;
}
impl pallet_timestamp::Config for Test {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}
pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(address_build(0).address)
	}
}
pub struct HashedConverter;
impl IntoAccountId<AccountId32> for HashedConverter {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut raw_account = [0u8; 32];
		raw_account[0..20].copy_from_slice(&address[..]);
		raw_account.into()
	}
}

frame_support::parameter_types! {
	pub const TransactionByteFee: u64 = 1;
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
	pub PrecompilesValue: MockPrecompiles<Test> = MockPrecompiles::<_>::new();
}

pub struct MockPrecompiles<R>(PhantomData<R>);
impl<R> MockPrecompiles<R>
where
	R: darwinia_ethereum::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1, 2, 3, 4].into_iter().map(|x| H160::from_low_u64_be(x)).collect()
	}
}

impl<R> PrecompileSet for MockPrecompiles<R>
where
	R: darwinia_ethereum::Config,
{
	fn execute(
		&self,
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> Option<PrecompileResult> {
		let to_address = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == to_address(1) =>
				Some(ECRecover::execute(input, target_gas, context, is_static)),
			_ if address == to_address(2) =>
				Some(Sha256::execute(input, target_gas, context, is_static)),
			_ if address == to_address(3) =>
				Some(Ripemd160::execute(input, target_gas, context, is_static)),
			_ if address == to_address(4) =>
				Some(Identity::execute(input, target_gas, context, is_static)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}

impl darwinia_evm::Config for Test {
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type ChainId = ChainId;
	type Event = Event;
	type FeeCalculator = FixedGasPrice;
	type FindAuthor = FindAuthorTruncated;
	type GasWeightMapping = ();
	type IntoAccountId = HashedConverter;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter<()>;
	type PrecompilesType = MockPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type Runner = Runner<Self>;
}

// --- pallet-bridge-messages config start ---
use bitvec::prelude::*;
use bp_messages::{
	source_chain::{
		LaneMessageVerifier, MessageDeliveryAndDispatchPayment, OnDeliveryConfirmed,
		OnMessageAccepted, Sender, TargetHeaderChain,
	},
	target_chain::{
		DispatchMessage, MessageDispatch, ProvedLaneMessages, ProvedMessages, SourceHeaderChain,
	},
	DeliveredMessages, InboundLaneData, LaneId, Message, MessageData, MessageKey, MessageNonce,
	OutboundLaneData, Parameter as MessagesParameter, UnrewardedRelayer,
};
use bp_runtime::{messages::MessageDispatchResult, Size};
use frame_support::{
	parameter_types,
	weights::{RuntimeDbWeight, Weight},
};
use sp_runtime::{
	testing::Header as SubstrateHeader,
	FixedU128,
};
use std::{
	collections::{BTreeMap, VecDeque},
	ops::RangeInclusive,
};
frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: u64 = 10;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: u64 = 16;
	pub const MaxUnconfirmedMessagesAtInboundLane: u64 = 32;
	pub storage TokenConversionRate: FixedU128 = 1.into();
  pub const TestBridgedChainId: bp_runtime::ChainId = *b"test";
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
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

pub type TestMessageFee = u64;
pub type TestRelayer = AccountId32;
pub const TEST_ERROR: &str = "Test error";
pub const PAYLOAD_REJECTED_BY_TARGET_CHAIN: TestPayload = message_payload(1, 50);

/// Constructs message payload using given arguments and zero unspent weight.
pub const fn message_payload(id: u64, declared_weight: Weight) -> TestPayload {
	TestPayload { id, declared_weight, dispatch_result: dispatch_result(0), extra: Vec::new() }
}

pub struct AccountIdConverter;

impl sp_runtime::traits::Convert<H256, AccountId32> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum TestMessagesParameter {
	TokenConversionRate(FixedU128),
}

impl MessagesParameter for TestMessagesParameter {
	fn save(&self) {
		match *self {
			TestMessagesParameter::TokenConversionRate(conversion_rate) =>
				TokenConversionRate::set(&conversion_rate),
		}
	}
}

/// Target header chain that is used in tests.
#[derive(Debug, Default)]
pub struct TestTargetHeaderChain;

impl TargetHeaderChain<TestPayload, TestRelayer> for TestTargetHeaderChain {
	type Error = &'static str;

	type MessagesDeliveryProof = ();

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
		Err(TEST_ERROR)
	}
}

/// Lane message verifier that is used in tests.
#[derive(Debug, Default)]
pub struct TestLaneMessageVerifier;

impl LaneMessageVerifier<AccountId32, TestPayload, TestMessageFee> for TestLaneMessageVerifier {
	type Error = &'static str;

	fn verify_message(
		_submitter: &Sender<AccountId32>,
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
	pub fn is_fee_paid(submitter: AccountId32, fee: TestMessageFee) -> bool {
		frame_support::storage::unhashed::get(b":message-fee:") ==
			Some((Sender::Signed(submitter), fee))
	}

	/// Returns true if given relayer has been rewarded with given balance. The reward-paid flag is
	/// cleared after the call.
	pub fn is_reward_paid(relayer: AccountId32, fee: TestMessageFee) -> bool {
		let key = (b":relayer-reward:", relayer, fee).encode();
		frame_support::storage::unhashed::take::<bool>(&key).is_some()
	}
}

impl MessageDeliveryAndDispatchPayment<AccountId32, TestMessageFee>
for TestMessageDeliveryAndDispatchPayment
{
	type Error = &'static str;

	fn pay_delivery_and_dispatch_fee(
		submitter: &Sender<AccountId32>,
		fee: &TestMessageFee,
		_relayer_fund_account: &AccountId32,
	) -> Result<(), Self::Error> {
		if frame_support::storage::unhashed::get(b":reject-message-fee:") == Some(true) {
			return Err(TEST_ERROR)
		}

		frame_support::storage::unhashed::put(b":message-fee:", &(submitter, fee));
		Ok(())
	}

	fn pay_relayers_rewards(
		lane_id: LaneId,
		message_relayers: VecDeque<UnrewardedRelayer<AccountId32>>,
		_confirmation_relayer: &AccountId32,
		received_range: &RangeInclusive<MessageNonce>,
		_relayer_fund_account: &AccountId32,
	) {
	}
}

/// Source header chain that is used in tests.
#[derive(Debug)]
pub struct TestSourceHeaderChain;

impl SourceHeaderChain<TestMessageFee> for TestSourceHeaderChain {
	type Error = &'static str;

	type MessagesProof = ();

	fn verify_messages_proof(
		_proof: Self::MessagesProof,
		_messages_count: u32,
	) -> Result<ProvedMessages<Message<TestMessageFee>>, Self::Error> {
		Err("Test error")
	}
}

/// Source header chain that is used in tests.
#[derive(Debug)]
pub struct TestMessageDispatch;

impl MessageDispatch<AccountId32, TestMessageFee> for TestMessageDispatch {
	type DispatchPayload = TestPayload;

	fn dispatch_weight(message: &DispatchMessage<TestPayload, TestMessageFee>) -> Weight {
		match message.data.payload.as_ref() {
			Ok(payload) => payload.declared_weight,
			Err(_) => 0,
		}
	}

	fn dispatch(
		_relayer_account: &AccountId32,
		message: DispatchMessage<TestPayload, TestMessageFee>,
	) -> MessageDispatchResult {
		match message.data.payload.as_ref() {
			Ok(payload) => payload.dispatch_result.clone(),
			Err(_) => dispatch_result(0),
		}
	}
}

/// Returns message dispatch result with given unspent weight.
pub const fn dispatch_result(unspent_weight: Weight) -> MessageDispatchResult {
	MessageDispatchResult {
		dispatch_result: true,
		unspent_weight,
		dispatch_fee_paid_during_dispatch: true,
	}
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
	type OnMessageAccepted = ();
	type OnDeliveryConfirmed = ();

	type SourceHeaderChain = TestSourceHeaderChain;
	type MessageDispatch = TestMessageDispatch;
	type BridgedChainId = TestBridgedChainId;
}
// --- pallet-bridge-messages config end ---

frame_support::parameter_types! {
	pub const MockPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl darwinia_ethereum::Config for Test {
	type Event = Event;
	type KtonCurrency = Kton;
	type PalletId = MockPalletId;
	type RingCurrency = Ring;
	type StateRoot = IntermediateStateRoot;
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
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: darwinia_ethereum::{Pallet, Call, Storage, Config, Event<T>, Origin},
	}
}

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			Call::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(&self, info: &Self::SignedInfo) -> Option<TransactionValidity> {
		match self {
			Call::Ethereum(ref call) => Some(validate_self_contained_inner(&self, &call, info)),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.pre_dispatch_self_contained(info),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<sp_runtime::traits::PostDispatchInfoOf<Self>>> {
		use sp_runtime::traits::Dispatchable as _;
		match self {
			call @ Call::Ethereum(crate::Call::transact { .. }) =>
				Some(call.dispatch(Origin::from(crate::RawOrigin::EthereumTransaction(info)))),
			_ => None,
		}
	}
}

fn validate_self_contained_inner(
	call: &Call,
	eth_call: &darwinia_ethereum::Call<Test>,
	signed_info: &<Call as fp_self_contained::SelfContainedCall>::SignedInfo,
) -> TransactionValidity {
	if let darwinia_ethereum::Call::transact { ref transaction } = eth_call {
		// Previously, ethereum transactions were contained in an unsigned
		// extrinsic, we now use a new form of dedicated extrinsic defined by
		// frontier, but to keep the same behavior as before, we must perform
		// the controls that were performed on the unsigned extrinsic.
		use sp_runtime::traits::SignedExtension as _;
		let input_len = match transaction {
			darwinia_ethereum::Transaction::Legacy(t) => t.input.len(),
			darwinia_ethereum::Transaction::EIP2930(t) => t.input.len(),
			darwinia_ethereum::Transaction::EIP1559(t) => t.input.len(),
		};
		let extra_validation =
			SignedExtra::validate_unsigned(call, &call.get_dispatch_info(), input_len)?;
		// Then, do the controls defined by the ethereum pallet.
		let self_contained_validation = eth_call
			.validate_self_contained(signed_info)
			.ok_or(TransactionValidityError::Invalid(InvalidTransaction::BadProof))??;

		Ok(extra_validation.combine_with(self_contained_validation))
	} else {
		Err(TransactionValidityError::Unknown(
			sp_runtime::transaction_validity::UnknownTransaction::CannotLookup,
		))
	}
}

pub struct AccountInfo {
	pub address: H160,
	pub account_id: AccountId32,
	pub private_key: H256,
}

pub struct LegacyUnsignedTransaction {
	pub nonce: U256,
	pub gas_price: U256,
	pub gas_limit: U256,
	pub action: TransactionAction,
	pub value: U256,
	pub input: Vec<u8>,
}
impl LegacyUnsignedTransaction {
	fn signing_rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(9);
		s.append(&self.nonce);
		s.append(&self.gas_price);
		s.append(&self.gas_limit);
		s.append(&self.action);
		s.append(&self.value);
		s.append(&self.input);
		s.append(&ChainId::get());
		s.append(&0u8);
		s.append(&0u8);
	}

	fn signing_hash(&self) -> H256 {
		let mut stream = RlpStream::new();
		self.signing_rlp_append(&mut stream);
		H256::from_slice(&Keccak256::digest(&stream.out()).as_slice())
	}

	pub fn sign(&self, key: &H256) -> Transaction {
		self.sign_with_chain_id(key, ChainId::get())
	}

	pub fn sign_with_chain_id(&self, key: &H256, chain_id: u64) -> Transaction {
		let hash = self.signing_hash();
		let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
		let s = libsecp256k1::sign(&msg, &libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap());
		let sig = s.0.serialize();

		let sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + chain_id * 2 + 35,
			H256::from_slice(&sig[0..32]),
			H256::from_slice(&sig[32..64]),
		)
		.unwrap();

		Transaction::Legacy(ethereum::LegacyTransaction {
			nonce: self.nonce,
			gas_price: self.gas_price,
			gas_limit: self.gas_limit,
			action: self.action,
			value: self.value,
			input: self.input.clone(),
			signature: sig,
		})
	}
}

pub struct EIP2930UnsignedTransaction {
	pub nonce: U256,
	pub gas_price: U256,
	pub gas_limit: U256,
	pub action: TransactionAction,
	pub value: U256,
	pub input: Vec<u8>,
}

impl EIP2930UnsignedTransaction {
	pub fn sign(&self, secret: &H256, chain_id: Option<u64>) -> Transaction {
		let secret = {
			let mut sk: [u8; 32] = [0u8; 32];
			sk.copy_from_slice(&secret[0..]);
			libsecp256k1::SecretKey::parse(&sk).unwrap()
		};
		let chain_id = chain_id.unwrap_or(ChainId::get());
		let msg = ethereum::EIP2930TransactionMessage {
			chain_id,
			nonce: self.nonce,
			gas_price: self.gas_price,
			gas_limit: self.gas_limit,
			action: self.action,
			value: self.value,
			input: self.input.clone(),
			access_list: vec![],
		};
		let signing_message = libsecp256k1::Message::parse_slice(&msg.hash()[..]).unwrap();

		let (signature, recid) = libsecp256k1::sign(&signing_message, &secret);
		let rs = signature.serialize();
		let r = H256::from_slice(&rs[0..32]);
		let s = H256::from_slice(&rs[32..64]);
		Transaction::EIP2930(ethereum::EIP2930Transaction {
			chain_id: msg.chain_id,
			nonce: msg.nonce,
			gas_price: msg.gas_price,
			gas_limit: msg.gas_limit,
			action: msg.action,
			value: msg.value,
			input: msg.input.clone(),
			access_list: msg.access_list,
			odd_y_parity: recid.serialize() != 0,
			r,
			s,
		})
	}
}

pub struct EIP1559UnsignedTransaction {
	pub nonce: U256,
	pub max_priority_fee_per_gas: U256,
	pub max_fee_per_gas: U256,
	pub gas_limit: U256,
	pub action: TransactionAction,
	pub value: U256,
	pub input: Vec<u8>,
}

impl EIP1559UnsignedTransaction {
	pub fn sign(&self, secret: &H256, chain_id: Option<u64>) -> Transaction {
		let secret = {
			let mut sk: [u8; 32] = [0u8; 32];
			sk.copy_from_slice(&secret[0..]);
			libsecp256k1::SecretKey::parse(&sk).unwrap()
		};
		let chain_id = chain_id.unwrap_or(ChainId::get());
		let msg = ethereum::EIP1559TransactionMessage {
			chain_id,
			nonce: self.nonce,
			max_priority_fee_per_gas: self.max_priority_fee_per_gas,
			max_fee_per_gas: self.max_fee_per_gas,
			gas_limit: self.gas_limit,
			action: self.action,
			value: self.value,
			input: self.input.clone(),
			access_list: vec![],
		};
		let signing_message = libsecp256k1::Message::parse_slice(&msg.hash()[..]).unwrap();

		let (signature, recid) = libsecp256k1::sign(&signing_message, &secret);
		let rs = signature.serialize();
		let r = H256::from_slice(&rs[0..32]);
		let s = H256::from_slice(&rs[32..64]);
		Transaction::EIP1559(ethereum::EIP1559Transaction {
			chain_id: msg.chain_id,
			nonce: msg.nonce,
			max_priority_fee_per_gas: msg.max_priority_fee_per_gas,
			max_fee_per_gas: msg.max_fee_per_gas,
			gas_limit: msg.gas_limit,
			action: msg.action,
			value: msg.value,
			input: msg.input.clone(),
			access_list: msg.access_list,
			odd_y_parity: recid.serialize() != 0,
			r,
			s,
		})
	}
}

fn address_build(seed: u8) -> AccountInfo {
	let raw_private_key = [seed + 1; 32];
	let secret_key = libsecp256k1::SecretKey::parse_slice(&raw_private_key).unwrap();
	let raw_public_key = &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
	let raw_address = {
		let mut s = [0; 20];
		s.copy_from_slice(&Keccak256::digest(raw_public_key)[12..]);
		s
	};
	let raw_account = {
		let mut s = [0; 32];
		s[..20].copy_from_slice(&raw_address);
		s
	};

	AccountInfo {
		private_key: raw_private_key.into(),
		account_id: raw_account.into(),
		address: raw_address.into(),
	}
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
	// sc_cli::init_logger("");
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let pairs = (0..accounts_len).map(|i| address_build(i as u8)).collect::<Vec<_>>();

	let balances: Vec<_> =
		(0..accounts_len).map(|i| (pairs[i].account_id.clone(), 100_000_000_000)).collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));

	(pairs, ext.into())
}

pub fn contract_address(sender: H160, nonce: u64) -> H160 {
	let mut rlp = RlpStream::new_list(2);
	rlp.append(&sender);
	rlp.append(&nonce);

	H160::from_slice(&Keccak256::digest(&rlp.out())[12..])
}

pub fn storage_address(sender: H160, slot: H256) -> H256 {
	H256::from_slice(&Keccak256::digest([&H256::from(sender)[..], &slot[..]].concat().as_slice()))
}
