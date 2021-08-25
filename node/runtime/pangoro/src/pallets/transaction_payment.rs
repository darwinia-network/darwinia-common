// --- substrate ---
use frame_support::weights::IdentityFee;
use pallet_transaction_payment::{Config, CurrencyAdapter};
// --- darwinia ---
use crate::*;

frame_support::parameter_types! {
	pub const TransactionByteFee: Balance = 1;
}

impl Config for Runtime {
	type OnChargeTransaction = CurrencyAdapter<Ring, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}
