pub use mmr::MMR;

// --- crates.io ---
use codec::{Decode, Encode};
// --- github.com ---
use mmr::{Error, MMRStore, Merge, Result as MMRResult};
// --- paritytech ---
use sp_core::offchain::StorageKind;
use sp_io::{offchain, offchain_index};
use sp_runtime::traits::Hash;
use sp_std::marker::PhantomData;
// --- darwinia ---
use crate::*;

pub struct Hasher<T>(PhantomData<T>);
impl<T: Config> Merge for Hasher<T> {
	type Item = <T as frame_system::Config>::Hash;

	fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Self::Item {
		let encodable = (lhs, rhs);
		<T as frame_system::Config>::Hashing::hash_of(&encodable)
	}
}

pub struct OffchainStorage;
pub struct RuntimeStorage;
pub struct Storage<StorageType, T>(PhantomData<(StorageType, T)>);
impl<StorageType, T> Default for Storage<StorageType, T> {
	fn default() -> Self {
		Self(Default::default())
	}
}
impl<T> MMRStore<T::Hash> for Storage<OffchainStorage, T>
where
	T: Config,
{
	fn get_elem(&self, pos: NodeIndex) -> MMRResult<Option<T::Hash>> {
		let key = <Pallet<T>>::offchain_key(pos);

		// TODO: search runtime DB while pruning
		Ok(offchain::local_storage_get(StorageKind::PERSISTENT, &key)
			.and_then(|v| Decode::decode(&mut &*v).ok()))
	}

	fn append(&mut self, _: NodeIndex, _: Vec<T::Hash>) -> MMRResult<()> {
		log::error!("Not allow to append elem(s) in the off-chain context!");

		Err(Error::InconsistentStore)
	}
}
impl<T> MMRStore<T::Hash> for Storage<RuntimeStorage, T>
where
	T: Config,
{
	fn get_elem(&self, pos: NodeIndex) -> MMRResult<Option<T::Hash>> {
		Ok(<Pallet<T>>::mmr_node_list(pos))
	}

	fn append(&mut self, pos: NodeIndex, elems: Vec<T::Hash>) -> MMRResult<()> {
		let mut mmr_size = <MmrSize<T>>::get();

		if pos != mmr_size {
			Err(Error::InconsistentStore)?;
		}

		for elem in elems.into_iter() {
			let key = <Pallet<T>>::offchain_key(mmr_size);

			// TODO prune to peaks on chain
			<MMRNodeList<T>>::insert(mmr_size, elem);
			elem.using_encoded(|elem| offchain_index::set(&key, elem));

			mmr_size += 1;
		}

		<MmrSize<T>>::put(mmr_size);

		Ok(())
	}
}

pub struct Mmr<StorageType, T>
where
	T: Config,
	Storage<StorageType, T>: MMRStore<T::Hash>,
{
	mmr: MMR<T::Hash, Hasher<T>, Storage<StorageType, T>>,
}
impl<StorageType, T> Mmr<StorageType, T>
where
	T: Config,
	Storage<StorageType, T>: MMRStore<T::Hash>,
{
	pub fn new() -> Self {
		Self {
			mmr: MMR::new(<MmrSize<T>>::get(), Default::default()),
		}
	}

	pub fn with_size(size: NodeIndex) -> Self {
		Self {
			mmr: MMR::new(size, Default::default()),
		}
	}

	pub fn get_root(self) -> MMRResult<T::Hash> {
		self.mmr.get_root()
	}
}
impl<T> Mmr<RuntimeStorage, T>
where
	T: Config,
{
	pub fn push(&mut self, leaf: T::Hash) -> Option<NodeIndex> {
		self.mmr.push(leaf).ok()
	}

	pub fn finalize(self) -> MMRResult<T::Hash> {
		let root = self.mmr.get_root()?;

		self.mmr.commit()?;

		Ok(root)
	}
}
