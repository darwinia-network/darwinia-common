#[cfg(feature = "try-runtime")]
pub mod try_runtime {
	pub fn pre_migrate() -> Result<(), &'static str> {
		Ok(())
	}
}
#[cfg(feature = "try-runtime")]
pub use try_runtime::*;

// --- darwinia-network ---
#[allow(unused)]
use crate::*;

#[allow(unused)]
pub fn migrate<T>()
where
	T: Config,
{
}
