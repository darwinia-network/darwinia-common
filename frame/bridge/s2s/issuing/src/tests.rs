#![cfg(test)]

use crate::*;
use crate::{self as s2s_issuing};
use darwinia_evm::{
	AddressMapping, EnsureAddressTruncated, FeeCalculator, Precompile, PrecompileSet,
};
use darwinia_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use darwinia_evm_precompile_transfer::Transfer;
use darwinia_support::s2s::TruncateToEthAddress;
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot,
};
use evm::{Context, ExitError, ExitSucceed};
use frame_support::assert_ok;
use frame_support::weights::PostDispatchInfo;
use frame_system::mocking::*;
use sha3::{Digest, Keccak256};
use sp_runtime::DispatchErrorWithPostInfo;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

use std::str::FromStr;

type Balance = u64;

use frame_support::traits::FindAuthor;
use frame_support::{traits::GenesisBuild, ConsensusEngineId, PalletId};

use codec::{Decode, Encode};
darwinia_support::impl_test_account_data! {}

impl darwinia_balances::Config<RingInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
	type Balance = Balance;
	type Event = ();
	type BalanceInfo = AccountData<Balance>;
}

impl darwinia_balances::Config<KtonInstance> for Test {
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
	type Balance = Balance;
	type Event = ();
	type BalanceInfo = AccountData<Balance>;
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
	type BaseCallFilter = ();
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

pub struct AccountInfo {
	pub address: H160,
	pub account_id: AccountId32,
	pub private_key: H256,
}

fn address_build(seed: u8) -> AccountInfo {
	let private_key = H256::from_slice(&[(seed + 1) as u8; 32]); //H256::from_low + 1) as u64);
	let secret_key = secp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
	let public_key = &secp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
	let address = H160::from(H256::from_slice(&Keccak256::digest(public_key)[..]));

	let mut data = [0u8; 32];
	data[0..20].copy_from_slice(&address[..]);

	AccountInfo {
		private_key,
		account_id: AccountId32::from(Into::<[u8; 32]>::into(data)),
		address,
	}
}

pub struct EthereumFindAuthor;
impl FindAuthor<H160> for EthereumFindAuthor {
	fn find_author<'a, I>(_digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		Some(address_build(0).address)
	}
}

impl dvm_ethereum::Config for Test {
	type Event = ();
	type FindAuthor = EthereumFindAuthor;
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

pub struct HashedAddressMapping;
frame_support::parameter_types! {
	pub const TransactionByteFee: u64 = 1;
	pub const ChainId: u64 = 42;
	pub const EVMPalletId: PalletId = PalletId(*b"py/evmpa");
	pub const BlockGasLimit: U256 = U256::MAX;
}

impl AddressMapping<AccountId32> for HashedAddressMapping {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);
		AccountId32::from(Into::<[u8; 32]>::into(data))
	}
}

pub struct MockPrecompiles<R>(PhantomData<R>);
impl<R> PrecompileSet for MockPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> Option<Result<(ExitSucceed, Vec<u8>, u64), ExitError>> {
		let to_address = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == to_address(1) => Some(ECRecover::execute(input, target_gas, context)),
			_ if address == to_address(2) => Some(Sha256::execute(input, target_gas, context)),
			_ if address == to_address(3) => Some(Ripemd160::execute(input, target_gas, context)),
			_ if address == to_address(4) => Some(Identity::execute(input, target_gas, context)),
			// Darwinia precompiles
			_ if address == to_address(21) => {
				Some(<Transfer<R>>::execute(input, target_gas, context))
			}
			_ => None,
		}
	}
}

impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type AddressMapping = HashedAddressMapping;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = ();
	type Precompiles = MockPrecompiles<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = darwinia_evm::runner::stack::Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type IssuingHandler = ();
}

frame_support::parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const MillauChainId: bp_runtime::ChainId = *b"mcid";
	pub RootAccountForPayments: Option<AccountId32> = Some([1;32].into());
}

pub struct AccountIdConverter;
impl Convert<H256, AccountId32> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId32 {
		hash.to_fixed_bytes().into()
	}
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
pub struct MockMessagePayload {
	spec_version: u32,
	weight: u64,
	call: Vec<u8>,
}

impl Size for MockMessagePayload {
	fn size_hint(&self) -> u32 {
		self.call.len() as _
	}
}

pub struct MillauCallEncoder;
impl EncodeCall<AccountId32, MockMessagePayload> for MillauCallEncoder {
	fn encode_remote_unlock(
		spec_version: u32,
		weight: u64,
		_token: Token,
		_recipient: RecipientAccount<AccountId32>,
	) -> Result<MockMessagePayload, ()> {
		return Ok(MockMessagePayload {
			spec_version,
			weight,
			call: vec![],
		});
	}
}

pub struct ToMillauMessageRelayCaller;
impl RelayMessageCaller<MockMessagePayload, Balance> for ToMillauMessageRelayCaller {
	fn send_message(
		_payload: MockMessagePayload,
		_fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		Ok(PostDispatchInfo {
			actual_weight: None,
			pays_fee: Pays::No,
		})
	}
}

impl Config for Test {
	type Event = ();
	type PalletId = S2sRelayPalletId;
	//type Call = Call;
	type WeightInfo = ();
	type ReceiverAccountId = AccountId32;

	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = MillauChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayload = MockMessagePayload;
	type CallEncoder = MillauCallEncoder;
	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToMillauMessageRelayCaller;
}

frame_support::construct_runtime! {
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		S2sIssuing: s2s_issuing::{Pallet, Call, Storage, Config, Event<T>},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
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
	t.into()
}

#[test]
fn burn_and_remote_unlock_success() {
	new_test_ext().execute_with(|| {
		let burn_info = TokenBurnInfo {
			spec_version: 0,
			weight: 100,
			token_type: 1,
			backing: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			sender: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			source: H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			recipient: [1; 32].to_vec(),
			amount: U256::from(1),
			fee: U256::from(1),
		};
		assert_ok!(S2sIssuing::burn_and_remote_unlock(0, burn_info,));
	});
}

#[test]
fn check_digest() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			S2sIssuing::digest(),
			array_bytes::hex2bytes_unchecked("0xd184c5bd").as_slice()
		);
	});
}
