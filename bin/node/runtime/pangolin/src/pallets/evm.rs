// --- substrate ---
use sp_core::U256;
// --- darwinia ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, ConcatAddressMapping, Config, EnsureAddressTruncated};
use darwinia_evm_precompile::DarwiniaPrecompiles;
use dvm_ethereum::account_basic::DvmAccountBasic;
use dvm_ethereum::account_basic::{KtonRemainBalance, RingRemainBalance};

frame_support::parameter_types! {
	pub const ChainId: u64 = 43;
	pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl Config for Runtime {
	type FeeCalculator = dvm_dynamic_fee::Pallet<Self>;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated;
	type AddressMapping = ConcatAddressMapping;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type Event = Event;
	type Precompiles = DarwiniaPrecompiles<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
	type IssuingHandler = EthereumIssuing;
}
