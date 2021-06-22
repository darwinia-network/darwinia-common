pub use darwinia_evm_precompile_issuing::Issuing;
pub use darwinia_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
pub use darwinia_evm_precompile_transfer::Transfer;

// --- crates.io ---
use evm::{Context, ExitError, ExitSucceed};
// --- substrate ---
use sp_core::{H160, U256};
use sp_std::{marker::PhantomData, vec::Vec};
// --- darwinia ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, ConcatAddressMapping, Config, EnsureAddressTruncated};
use dp_evm::{Precompile, PrecompileSet};
use dvm_ethereum::account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance};

pub struct PangolinPrecompiles<R>(PhantomData<R>);
impl<R: darwinia_evm::Config> PrecompileSet for PangolinPrecompiles<R> {
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> Option<Result<(ExitSucceed, Vec<u8>, u64), ExitError>> {
		let addr = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == addr(1) => Some(ECRecover::execute(input, target_gas, context)),
			_ if address == addr(2) => Some(Sha256::execute(input, target_gas, context)),
			_ if address == addr(3) => Some(Ripemd160::execute(input, target_gas, context)),
			_ if address == addr(4) => Some(Identity::execute(input, target_gas, context)),
			// Darwinia precompiles
			_ if address == addr(21) => Some(<Transfer<R>>::execute(input, target_gas, context)),
			_ if address == addr(23) => Some(<Issuing<R>>::execute(input, target_gas, context)),
			_ => None,
		}
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 43;
	pub BlockGasLimit: U256 = u32::max_value().into();
}

impl Config for Runtime {
	type FeeCalculator = dvm_dynamic_fee::Pallet<Self>;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type AddressMapping = ConcatAddressMapping<Self::AccountId>;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = Event;
	type Precompiles = PangolinPrecompiles<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
	type IssuingHandler = EthereumIssuing;
}
