#[cfg(feature = "try-runtime")]
pub mod try_runtime {
	pub fn pre_migrate() -> Result<(), &'static str> {
		Ok(())
	}
}
#[cfg(feature = "try-runtime")]
pub use try_runtime::*;

// --- darwinia-network ---
use crate::*;

pub fn migrate<T, I>()
where
	T: Config<I>,
	I: 'static,
{
	// --- darwinia-network ---
	use darwinia_support::balance::*;
	// --- paritytech ---
	use frame_support::WeakBoundedVec;
	use sp_std::prelude::*;

	<Locks<T, I>>::translate::<
		WeakBoundedVec<OldBalanceLock<T::Balance, T::BlockNumber>, T::MaxLocks>,
		_,
	>(|_, locks| {
		Some(WeakBoundedVec::force_from(
			locks
				.into_inner()
				.into_iter()
				.map(|OldBalanceLock { id, lock_for, reasons }| BalanceLock::<T::Balance> {
					id,
					amount: match lock_for {
						LockFor::Common { amount } => amount,
						LockFor::Staking(staking_lock) =>
							staking_lock.staking_amount + staking_lock.total_unbond(),
					},
					reasons,
				})
				.collect::<Vec<_>>(),
			None,
		))
	});
}
