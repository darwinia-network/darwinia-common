// --- paritytech ---
use frame_support::weights::IdentityFee;
use pallet_transaction_payment::{Config, CurrencyAdapter};
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl Config for Runtime {
	type OnChargeTransaction = CurrencyAdapter<Ring, ()>;
	type TransactionByteFee = TransactionByteFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}
