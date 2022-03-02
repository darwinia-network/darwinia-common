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

// --- std ---
use std::str::FromStr;
// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use ethereum::{TransactionAction, TransactionSignature};
use rlp::RlpStream;
use scale_info::TypeInfo;
use sha3::{Digest, Keccak256};
// --- paritytech ---
use frame_support::{
	traits::{Everything, GenesisBuild},
	PalletId,
};
use frame_system::mocking::*;
use pallet_evm::FeeCalculator;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{
	*, {self as s2s_issuing},
};
use darwinia_evm::{EVMCurrencyAdapter, EnsureAddressTruncated, SubstrateBlockHashMapping};
use darwinia_support::{
	evm::IntoAccountId,
	s2s::{LatestMessageNoncer, RelayMessageSender},
};
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot, RawOrigin, Transaction,
};

type Block = MockBlock<Test>;
type SignedExtra = (frame_system::CheckSpecVersion<Test>,);
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test, (), SignedExtra>;
type Balance = u64;

/*
* Test Contract
pragma solidity ^0.6.0;

contract MockS2SMappingTokenFactory {

	address public constant SYSTEM_ACCOUNT = 0x6D6F646C6461722f64766D700000000000000000;
	address public constant MOCKED_ADDRESS = 0x0000000000000000000000000000000000000001;

	mapping(bytes32 => address) public salt2MappingToken;

	event IssuingERC20Created(address backing_address, address original_token, address mapping_token);
	event MappingTokenIssued(address mapping_token, address recipient, uint256 amount);

	receive() external payable {
	}

	/**
	 * @dev Throws if called by any account other than the system account defined by SYSTEM_ACCOUNT address.
	 */
	modifier onlySystem() {
		require(SYSTEM_ACCOUNT == msg.sender, "System: caller is not the system account");
		_;
	}

	function mappingToken(address backing_address, address original_token) public view returns (address) {
		bytes32 salt = keccak256(abi.encodePacked(backing_address, original_token));
		return salt2MappingToken[salt];
	}

	function newErc20Contract(
		uint32,
		string memory,
		string memory,
		uint8,
		address backing_address,
		address original_token
	) public virtual onlySystem returns (address mapping_token) {
		bytes32 salt = keccak256(abi.encodePacked(backing_address, original_token));
		salt2MappingToken[salt] = MOCKED_ADDRESS;
		emit IssuingERC20Created(backing_address, original_token, MOCKED_ADDRESS);
		return MOCKED_ADDRESS;
	}

	function issueMappingToken(address mapping_token, address recipient, uint256 amount) public virtual onlySystem {
		require(mapping_token == MOCKED_ADDRESS, "invalid mapping token address");
		emit MappingTokenIssued(mapping_token, recipient, amount);
	}
}
*/
pub const TEST_CONTRACT_BYTECODE: &str = "608060405234801561001057600080fd5b506105ad806100206000396000f3fe6080604052600436106100595760003560e01c8063148a79fd14610065578063739d40d9146100ab578063b28bf620146100c0578063c8ff0854146100d5578063ecd22a191461011a578063ef13ef4d1461015557610060565b3661006057005b600080fd5b34801561007157600080fd5b5061008f6004803603602081101561008857600080fd5b50356102b6565b604080516001600160a01b039092168252519081900360200190f35b3480156100b757600080fd5b5061008f6102d1565b3480156100cc57600080fd5b5061008f6102e4565b3480156100e157600080fd5b50610118600480360360608110156100f857600080fd5b506001600160a01b038135811691602081013590911690604001356102e9565b005b34801561012657600080fd5b5061008f6004803603604081101561013d57600080fd5b506001600160a01b03813581169160200135166103e3565b34801561016157600080fd5b5061008f600480360360c081101561017857600080fd5b63ffffffff82351691908101906040810160208201356401000000008111156101a057600080fd5b8201836020820111156101b257600080fd5b803590602001918460018302840111640100000000831117156101d457600080fd5b91908080601f016020809104026020016040519081016040528093929190818152602001838380828437600092019190915250929594936020810193503591505064010000000081111561022757600080fd5b82018360208201111561023957600080fd5b8035906020019184600183028401116401000000008311171561025b57600080fd5b91908080601f0160208091040260200160405190810160405280939291908181526020018383808284376000920191909152509295505060ff8335169350506001600160a01b03602083013581169260400135169050610442565b6000602081905290815260409020546001600160a01b031681565b6b06d6f646c6461722f64766d760441b81565b600181565b6b06d6f646c6461722f64766d760441b33146103365760405162461bcd60e51b81526004018080602001828103825260288152602001806105506028913960400191505060405180910390fd5b6001600160a01b038316600114610394576040805162461bcd60e51b815260206004820152601d60248201527f696e76616c6964206d617070696e6720746f6b656e2061646472657373000000604482015290519081900360640190fd5b604080516001600160a01b0380861682528416602082015280820183905290517f4c965b0027d1a0b20e874218493f3717f065d312001e29e75c42d135c7ab96259181900360600190a1505050565b604080516bffffffffffffffffffffffff19606094851b81166020808401919091529390941b9093166034840152805160288185030181526048909301815282519282019290922060009081529081905220546001600160a01b031690565b60006b06d6f646c6461722f64766d760441b33146104915760405162461bcd60e51b81526004018080602001828103825260288152602001806105506028913960400191505060405180910390fd5b604080516bffffffffffffffffffffffff19606086811b82166020808501919091529086901b909116603483015282516028818403018152604883018085528151918301919091206000818152928390529184902080546001600160a01b03191660019081179091556001600160a01b038089169092529086166068840152608883015291517fc9c337e478378d4317643765b21b7d2da0d66f86675b2e3b6e1aff67ce572daf9181900360a80190a150600197965050505050505056fe53797374656d3a2063616c6c6572206973206e6f74207468652073797374656d206163636f756e74a26469706673582212202576f15b3a6363c8f6949d2605f736ff2b3b65e179f1788b5db7d244efebbc0d64736f6c63430006090033";

darwinia_support::impl_test_account_data! {}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl darwinia_balances::Config<RingInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type OtherCurrencies = ();
	type Balance = Balance;
	type Event = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type BalanceInfo = AccountData<Balance>;
	type WeightInfo = ();
}
impl darwinia_balances::Config<KtonInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type OtherCurrencies = ();
	type Balance = Balance;
	type Event = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type BalanceInfo = AccountData<Balance>;
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const MinimumPeriod: u64 = 6000 / 2;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
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
	pub const DvmPalletId: PalletId = PalletId(*b"dar/dvmp");
}

impl dvm_ethereum::Config for Test {
	type Event = ();
	type PalletId = DvmPalletId;
	type StateRoot = IntermediateStateRoot;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		1.into()
	}
}

pub struct HashedConverter;
impl IntoAccountId<AccountId32> for HashedConverter {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);
		AccountId32::from(Into::<[u8; 32]>::into(data))
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 42;
	pub const BlockGasLimit: U256 = U256::MAX;
}
impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = HashedConverter;
	type Event = ();
	type PrecompilesType = ();
	type PrecompilesValue = ();
	type FindAuthor = ();
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter;
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const PangoroChainId: bp_runtime::ChainId = *b"pagr";
	pub RootAccountForPayments: Option<AccountId32> = Some([1;32].into());
	pub PangoroName: Vec<u8> = (b"Pangoro").to_vec();
	pub MessageLaneId: [u8; 4] = *b"ltor";
}

pub struct AccountIdConverter;
impl Convert<H256, AccountId32> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}
pub struct ToPangoroMessageRelayCaller;
impl RelayMessageSender for ToPangoroMessageRelayCaller {
	fn encode_send_message(
		_message_pallet_index: u32,
		_lane_id: [u8; 4],
		_payload: Vec<u8>,
		_fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		Ok(Vec::new())
	}
}
pub struct MockLatestMessageNoncer;
impl LatestMessageNoncer for MockLatestMessageNoncer {
	fn outbound_latest_generated_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}
	fn inbound_latest_received_nonce(_lane_id: [u8; 4]) -> u64 {
		0
	}
}

pub struct TruncateToEthAddress;
impl ToEthAddress<AccountId32> for TruncateToEthAddress {
	fn into_ethereum_id(address: &AccountId32) -> H160 {
		let account20: &[u8] = &address.as_ref();
		H160::from_slice(&account20[..20])
	}
}

impl Config for Test {
	type Event = ();
	type PalletId = S2sRelayPalletId;
	type WeightInfo = ();

	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayloadCreator = ();
	type InternalTransactHandler = Ethereum;
	type BackingChainName = PangoroName;
	type MessageLaneId = MessageLaneId;
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
		S2sIssuing: s2s_issuing::{Pallet, Call, Storage, Config, Event<T>},
		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>},
		Ethereum: dvm_ethereum::{Pallet, Call, Storage, Config, Origin},
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
			Call::Ethereum(call) => call.validate_self_contained(info),
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
			call @ Call::Ethereum(dvm_ethereum::Call::transact { .. }) => {
				Some(call.dispatch(Origin::from(RawOrigin::EthereumTransaction(info))))
			}
			_ => None,
		}
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
		let hash = self.signing_hash();
		let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
		let s = libsecp256k1::sign(
			&msg,
			&libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap(),
		);
		let sig = s.0.serialize();

		let sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + ChainId::get() * 2 + 35,
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

pub fn new_test_ext(accounts_len: usize) -> (Vec<AccountInfo>, sp_io::TestExternalities) {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	let mapping_factory_address =
		H160::from_str("0000000000000000000000000000000000000002").unwrap();

	<s2s_issuing::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
		&s2s_issuing::GenesisConfig {
			mapping_factory_address,
		},
		&mut t,
	)
	.unwrap();

	let pairs = (0..accounts_len)
		.map(|i| address_build(i as u8))
		.collect::<Vec<_>>();

	let balances: Vec<_> = (0..accounts_len)
		.map(|i| (pairs[i].account_id.clone(), 100_000_000_000))
		.collect();

	darwinia_balances::GenesisConfig::<Test, RingInstance> { balances }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	(pairs, ext.into())
	//t.into()
}
