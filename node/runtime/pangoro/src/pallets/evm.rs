// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use codec::{Decode, Encode};
use fp_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::FindAuthor,
	ConsensusEngineId,
};
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::{crypto::Public, H160, U256};
// --- darwinia-network ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, Config, EVMCurrencyAdapter, EnsureAddressTruncated};
use darwinia_evm_precompile_transfer::Transfer;
use darwinia_support::evm::ConcatConverter;
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	EthereumBlockHashMapping,
};

pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		F::find_author(digests).map(|author_index| {
			let authority_id = Babe::authorities()[author_index as usize].clone();

			H160::from_slice(&authority_id.0.to_raw_vec()[4..24])
		})
	}
}

pub struct PangoroPrecompiles<R>(PhantomData<R>);
impl<R> PangoroPrecompiles<R>
where
	R: darwinia_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1, 2, 3, 4, 21]
			.into_iter()
			.map(|x| addr(x))
			.collect()
	}
}

impl<R> PrecompileSet for PangoroPrecompiles<R>
where
	R: darwinia_evm::Config + darwinia_bridge_bsc::Config,
	R::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode + Decode,
	<R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
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
			// Ethereum precompiles
			_ if address == addr(1) => {
				Some(ECRecover::execute(input, target_gas, context, is_static))
			}
			_ if address == addr(2) => Some(Sha256::execute(input, target_gas, context, is_static)),
			_ if address == addr(3) => {
				Some(Ripemd160::execute(input, target_gas, context, is_static))
			}
			_ if address == addr(4) => {
				Some(Identity::execute(input, target_gas, context, is_static))
			}
			// Darwinia precompiles
			_ if address == addr(21) => Some(<Transfer<R>>::execute(input, target_gas, context)),
			_ if address == addr(26) => Some(<BscBridge<R>>::execute(input, target_gas, context)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		U256::from(1 * COIN)
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 45;
	pub BlockGasLimit: U256 = u32::MAX.into();
	pub PrecompilesValue: PangoroPrecompiles<Runtime> = PangoroPrecompiles::<_>::new();
}

impl Config for Runtime {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = ConcatConverter<Self::AccountId>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Event = Event;
	type PrecompilesType = PangoroPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
	type OnChargeTransaction = EVMCurrencyAdapter;
}

fn addr(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}
