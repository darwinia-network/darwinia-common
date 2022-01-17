// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use bp_messages::LaneId;
use codec::{Decode, Encode};
use fp_evm::{Context, ExitError, Precompile, PrecompileOutput, PrecompileSet};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{FindAuthor, PalletInfoAccess},
	ConsensusEngineId,
};
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::{crypto::Public, H160, U256};
// --- darwinia-network ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, Config, EnsureAddressTruncated, FeeCalculator};
use darwinia_evm_precompile_bridge_bsc::BscBridge;
use darwinia_evm_precompile_bridge_ethereum::EthereumBridge;
use darwinia_evm_precompile_bridge_s2s::Sub2SubBridge;
use darwinia_evm_precompile_dispatch::Dispatch;
use darwinia_evm_precompile_transfer::Transfer;
use darwinia_support::{
	evm::ConcatConverter,
	s2s::{LatestMessageNoncer, RelayMessageSender},
};
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

// TODO: rename
pub struct PangolinPrecompiles<R>(PhantomData<R>);
impl<R> PrecompileSet for PangolinPrecompiles<R>
where
	R: from_substrate_issuing::Config + from_ethereum_issuing::Config,
	R: darwinia_evm::Config,
	R: darwinia_bridge_bsc::Config,
	R::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode + Decode,
	<R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
	R::Call: From<from_ethereum_issuing::Call<R>> + From<from_substrate_issuing::Call<R>>,
{
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> Option<Result<PrecompileOutput, ExitError>> {
		let addr = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == addr(1) => Some(ECRecover::execute(input, target_gas, context)),
			_ if address == addr(2) => Some(Sha256::execute(input, target_gas, context)),
			_ if address == addr(3) => Some(Ripemd160::execute(input, target_gas, context)),
			_ if address == addr(4) => Some(Identity::execute(input, target_gas, context)),
			// Darwinia precompiles
			_ if address == addr(21) => Some(<Transfer<R>>::execute(input, target_gas, context)),

		}
	}
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		U256::from(1 * COIN)
	}
}

// TODO: Update chain id
frame_support::parameter_types! {
	pub const ChainId: u64 = 43;
	pub BlockGasLimit: U256 = u32::MAX.into();
}

impl Config for Runtime {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = ConcatConverter<Self::AccountId>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Event = Event;
	type Precompiles = PangolinPrecompiles<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
}
