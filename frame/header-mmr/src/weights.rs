// --- paritytech ---
use frame_support::weights::Weight;

pub trait WeightInfo {
	fn on_initialize() -> Weight;
}
impl WeightInfo for () {
	fn on_initialize() -> Weight {
		0
	}
}
