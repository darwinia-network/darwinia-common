//! Some configurable implementations as associated type for the substrate runtime.

// --- crates ---
use smallvec::smallvec;
// --- substrate ---
use frame_support::{
	traits::{Imbalance, OnUnbalanced},
	weights::{
		constants::ExtrinsicBaseWeight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
};
use sp_runtime::{Perbill, RuntimeDebug};
// --- darwinia ---
use crate::*;

darwinia_support::impl_account_data! {
	struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
		// other data
	}
}

// pub struct ToAuthor;
// impl OnUnbalanced<RingNegativeImbalance> for ToAuthor {
// 	fn on_nonzero_unbalanced(amount: RingNegativeImbalance) {
// 		let numeric_amount = amount.peek();
// 		let author = Authorship::author();
// 		Ring::resolve_creating(&Authorship::author(), amount);
// 		System::deposit_event(<darwinia_balances::Event<Runtime, RingInstance>>::Deposit(
// 			author,
// 			numeric_amount,
// 		));
// 	}
// }

pub struct DealWithFees;
impl OnUnbalanced<RingNegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = RingNegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 80% to treasury, 20% to author
			let mut split = fees.ration(80, 20);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				tips.merge_into(&mut split.1);
			}
			// Treasury::on_unbalanced(split.0);
			// ToAuthor::on_unbalanced(split.1);
		}
	}
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, MAXIMUM_BLOCK_WEIGHT]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Crab, extrinsic base weight (smallest non-zero weight) is mapped to 100 MILLI:
		let p = 100 * MILLI;
		let q = Balance::from(ExtrinsicBaseWeight::get());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}
