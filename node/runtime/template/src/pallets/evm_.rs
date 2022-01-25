// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use frame_support::{traits::FindAuthor, ConsensusEngineId};
use sp_core::crypto::Public;
// --- darwinia-network ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, Config, EnsureAddressTruncated};
use darwinia_support::evm::ConcatConverter;
use dvm_ethereum::account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance};

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
	R: pallet_evm::Config,
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

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		U256::from(1)
	}
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
}
