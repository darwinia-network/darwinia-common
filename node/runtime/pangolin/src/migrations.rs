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
	use frame_support::StorageHasher;

	let from = AccountId::from([
		100, 118, 109, 58, 0, 0, 0, 0, 0, 0, 0, 136, 9, 249, 179, 172, 239, 29, 163, 9, 244, 155,
		90, 185, 122, 76, 15, 170, 100, 230, 174, 73,
	]);
	let to = AccountId::from([
		100, 118, 109, 58, 0, 0, 0, 0, 0, 0, 0, 227, 249, 15, 111, 231, 199, 11, 31, 140, 190, 188,
		52, 119, 4, 141, 79, 50, 230, 31, 7, 204,
	]);

	let _ = Kton::transfer_all(Origin::signed(from.clone()), Address::from(to.clone()), false);
	if let Some(v) = migration::take_storage_value::<Balance>(
		b"Ethereum",
		b"RemainingKtonBalance",
		&frame_support::Blake2_128Concat::hash(from.as_ref()),
	) {
		migration::put_storage_value(
			b"Ethereum",
			b"RemainingKtonBalance",
			&frame_support::Blake2_128Concat::hash(to.as_ref()),
			v,
		);
	}

	// 0
	// <Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	RuntimeBlockWeights::get().max_block
}
