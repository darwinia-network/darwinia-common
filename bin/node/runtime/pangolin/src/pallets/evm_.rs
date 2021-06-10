pub use darwinia_evm_precompile_dispatch::Dispatch;
pub use darwinia_evm_precompile_issuing::Issuing;
pub use darwinia_evm_precompile_kton::Kton as KtonPrecompile;
pub use darwinia_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
pub use darwinia_evm_precompile_util::Util;
pub use darwinia_evm_precompile_withdraw::WithDraw;

// --- crates.io ---
use evm::{Context, ExitError, ExitSucceed};
// --- substrate ---
use codec::{Decode, Encode};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use sp_core::{H160, U256};
use sp_std::{marker::PhantomData, vec::Vec};
// --- darwinia ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, ConcatAddressMapping, Config, EnsureAddressTruncated};
use dp_evm::{Precompile, PrecompileSet};
use dvm_ethereum::account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance};

pub struct PangolinPrecompiles<R>(PhantomData<R>);
impl<R> PrecompileSet for PangolinPrecompiles<R>
where
	R: darwinia_s2s_issuing::Config + darwinia_evm::Config,
	R::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode + Decode,
	<R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
	R::Call: From<darwinia_s2s_issuing::Call<R>>,
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
				Some(<WithDraw<R>>::execute(input, target_gas, context))
			}
			_ if address == to_address(22) => {
				Some(<KtonPrecompile<R>>::execute(input, target_gas, context))
			}
			_ if address == to_address(23) => {
				Some(<Issuing<R>>::execute(input, target_gas, context))
			}
			_ if address == to_address(24) => Some(<Util<R>>::execute(input, target_gas, context)),
			_ if address == to_address(25) => {
				Some(<Dispatch<R>>::execute(input, target_gas, context))
			}
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
	type CallOrigin = EnsureAddressTruncated;
	type AddressMapping = ConcatAddressMapping;
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
