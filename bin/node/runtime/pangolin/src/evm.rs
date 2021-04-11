// --- substrate ---
use sp_core::U256;
// --- darwinia ---
use crate::*;
use darwinia_evm::{
	runner::stack::Runner, ConcatAddressMapping, Config, EnsureAddressTruncated, FeeCalculator,
};
use darwinia_evm_precompile::DarwiniaPrecompiles;
use dvm_ethereum::account_basic::DvmAccountBasic;
use dvm_ethereum::{KtonRemainBalance, RingRemainBalance};

/// Fixed gas price.
pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		// Gas price is always one token per gas.
		1_000_000_000.into()
	}
}
frame_support::parameter_types! {
	pub const ChainId: u64 = 43;
}
impl Config for Runtime {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = ConcatAddressMapping;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = Event;
	type Precompiles = DarwiniaPrecompiles<Self>;
	type ChainId = ChainId;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
}
