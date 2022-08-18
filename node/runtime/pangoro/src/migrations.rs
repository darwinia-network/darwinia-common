// --- paritytech ---
#[allow(unused)]
use frame_support::{migration, traits::OnRuntimeUpgrade, weights::Weight};
// --- darwinia-network ---
#[allow(unused)]
use crate::*;

pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		migrate()
	}
}

fn migrate() -> Weight {
	let module: &[u8] = b"PangolinFeeMarket";
	let item: &[u8] = b"Orders";

	for ((lane_id, nonce), order) in storage_key_iter::<
		(LaneId, MessageNonce),
		Order<AccountId, BlockNumber, Balance>,
		Blake2_128Concat,
	>(module, item)
	.drain()
	{
		if lane_id != [0, 0, 0, 0] || lane_id != [0, 0, 0, 1] {
			Orders::<Runtime, WithPangolinFeeMarket>::insert((lane_id, nonce), order);
		}
	}

	// 0
	RuntimeBlockWeights::get().max_block
}
