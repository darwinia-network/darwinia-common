#![cfg(test)]

use crate::*;
use crate::{self as s2s_issuing, *};
use darwinia_evm::{AddressMapping, EnsureAddressTruncated, FeeCalculator, IssuingHandler};
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	IntermediateStateRoot,
};
use frame_support::assert_ok;
use frame_system::mocking::*;
use sha3::{Digest, Keccak256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, RuntimeDebug,
};

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

use std::str::FromStr;

type Balance = u64;

use frame_support::{ensure, traits::FindAuthor};
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

impl darwinia_evm::Config for Test {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = ();
	type Precompiles = (
		darwinia_evm_precompile_simple::ECRecover,
		darwinia_evm_precompile_simple::Sha256,
		darwinia_evm_precompile_simple::Ripemd160,
		darwinia_evm_precompile_simple::Identity,
		darwinia_evm_precompile_withdraw::WithDraw<Self>,
	);
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

pub struct MockRelay;
impl Relay for MockRelay {
	type RelayProof = AccountId32;
	type RelayMessage = (u32, Token, RelayAccount<AccountId32>);
	type VerifiedResult = Result<EthereumAddress, DispatchError>;
	type RelayMessageResult = DispatchResult;
	//Origin::signed(0)
	fn verify(proof: &Self::RelayProof) -> Self::VerifiedResult {
		//ensure!(proof == [0;32], "verify failed");
		Ok(EthereumAddress::from_str("0000000000000000000000000000000000000000").unwrap())
	}
	fn relay_message(_message: &Self::RelayMessage) -> Self::RelayMessageResult {
		Ok(())
	}
	fn digest() -> RelayDigest {
        let mut digest: RelayDigest = Default::default();
        let pallet_digest = sha3::Keccak256::digest(S2sRelayPalletId::get().encode().as_slice());
        digest.copy_from_slice(&pallet_digest[..4]);
        digest
	}
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
}

impl Config for Test {
	type Event = ();
	type PalletId = S2sRelayPalletId;
	//type Call = Call;
	type WeightInfo = ();
	type ReceiverAccountId = AccountId32;
	type BackingRelay = MockRelay;
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

	s2s_issuing::GenesisConfig {
		mapping_factory_address,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

#[test]
fn burn_and_remote_unlock_success() {
	new_test_ext().execute_with(|| {
		assert_ok!(S2sIssuing::burn_and_remote_unlock(
			0,
			0,
			H160::from_str("1000000000000000000000000000000000000001").unwrap(),
			AccountId32::from(Into::<[u8; 32]>::into([0; 32])),
			U256::from(1),
		));
	});
}

#[test]
fn check_relay_digest() {
	new_test_ext().execute_with(|| {
        assert_eq!(
            S2sIssuing::relay_digest(),
            array_bytes::hex2bytes_unchecked("0xd184c5bd").as_slice()
        );
	});
}

#[test]
fn test_encode_token_creation() {
	new_test_ext().execute_with(|| {
        let encoded = S2sIssuing::abi_encode_token_creation(
            EthereumAddress::from_str("0000000000000000000000000000000000000001").unwrap(),
            EthereumAddress::from_str("0000000000000000000000000000000000000002").unwrap(),
            1,
            "ring token",
            "RING",
            9
        );
        assert_ok!(&encoded);
        let stream = encoded.unwrap();
        assert_eq!(stream,
                   array_bytes::hex2bytes_unchecked("0xe89a0b30d184c5bd00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000000000900000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000a72696e6720746f6b656e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000452494e4700000000000000000000000000000000000000000000000000000000").as_slice());
    });
}

#[test]
fn test_bytes_to_account_id() {
    new_test_ext().execute_with(|| {
        let accountid = AccountId32::from(Into::<[u8; 32]>::into([1; 32]));
        assert_eq!(
            S2sIssuing::account_id_try_from_bytes(&[1;32]).unwrap(),
            accountid
        );
	});
}

