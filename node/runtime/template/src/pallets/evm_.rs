// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use fp_evm::{Context, PrecompileResult};
use frame_support::{traits::FindAuthor, ConsensusEngineId};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use sp_core::crypto::Public;

// --- darwinia-network ---
use crate::*;
use darwinia_evm::{
	runner::stack::Runner, Config, EVMCurrencyAdapter, EnsureAddressTruncated, Precompile,
	PrecompileSet,
};
use darwinia_support::evm::ConcatConverter;
use dvm_ethereum::account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance};

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1, 2, 3, 4, 5, 1024, 1025]
			.into_iter()
			.map(|x| hash(x))
			.collect()
	}
}
impl<R> PrecompileSet for FrontierPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	fn execute(
		&self,
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> Option<PrecompileResult> {
		match address {
			// Ethereum precompiles :
			a if a == hash(1) => Some(ECRecover::execute(input, target_gas, context, is_static)),
			a if a == hash(2) => Some(Sha256::execute(input, target_gas, context, is_static)),
			a if a == hash(3) => Some(Ripemd160::execute(input, target_gas, context, is_static)),
			a if a == hash(4) => Some(Identity::execute(input, target_gas, context, is_static)),
			a if a == hash(5) => Some(Modexp::execute(input, target_gas, context, is_static)),
			// Non-Frontier specific nor Ethereum precompiles :
			a if a == hash(1024) => {
				Some(Sha3FIPS256::execute(input, target_gas, context, is_static))
			}
			a if a == hash(1025) => Some(ECRecoverPublicKey::execute(
				input, target_gas, context, is_static,
			)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}

fn hash(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}
pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		F::find_author(digests).map(|author_index| {
			let authority_id = Aura::authorities()[author_index as usize].clone();

			H160::from_slice(&authority_id.to_raw_vec()[4..24])
		})
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 42;
	pub BlockGasLimit: U256 = U256::from(u32::max_value());
	pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
}

impl Config for Runtime {
	type FeeCalculator = BaseFee;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = ConcatConverter<Self::AccountId>;
	type FindAuthor = FindAuthorTruncated<Aura>;
	type BlockHashMapping = dvm_ethereum::EthereumBlockHashMapping<Self>;
	type Event = Event;
	type PrecompilesType = FrontierPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = Runner<Self>;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type OnChargeTransaction = EVMCurrencyAdapter;
}
