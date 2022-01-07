pub use mmr::MMR;

// --- crates.io ---
use codec::{Decode, Encode};
// --- github.com ---
use mmr::{Error, MMRStore, Merge, MerkleProof, Result as MMRResult};
// --- paritytech ---
use frame_support::log;
use sp_core::offchain::StorageKind;
use sp_io::{offchain, offchain_index};
use sp_runtime::traits::Hash;
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use crate::*;

pub struct Hasher<T>(PhantomData<T>);
impl<T: Config> Merge for Hasher<T> {
	type Item = T::Hash;

	fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Self::Item {
		T::Hashing::hash_of(&(lhs, rhs))
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
	fn get_elem(&self, position: NodeIndex) -> MMRResult<Option<T::Hash>> {
		let key = <Pallet<T>>::offchain_key(position);

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
	fn get_elem(&self, position: NodeIndex) -> MMRResult<Option<T::Hash>> {
		Ok(<Pallet<T>>::peak_of(position))
	}

	// ? Actually we can move this pruning logic outside
	// ? and perform the pruning every X blocks
	// ?
	// ? commented by Xavier
	fn append(&mut self, position: NodeIndex, elems: Vec<T::Hash>) -> MMRResult<()> {
		let mmr_size = <MmrSize<T>>::get();

		if position != mmr_size {
			Err(Error::InconsistentStore)?;
		}

		let diff = |a: &[NodeIndex], b: &[NodeIndex]| -> Vec<NodeIndex> {
			b.iter().filter(|x| !a.contains(x)).cloned().collect()
		};
		let peaks_before = if mmr_size == 0 {
			vec![]
		} else {
			mmr::helper::get_peaks(mmr_size)
		};
		let elems = elems
			.into_iter()
			.enumerate()
			.map(|(i, elem)| (mmr_size + i as NodeIndex, elem))
			.collect::<Vec<_>>();
		let mmr_size = mmr_size + elems.len() as NodeIndex;

		<MmrSize<T>>::put(mmr_size);

		for (position, elem) in elems.iter() {
			elem.using_encoded(|elem| {
				offchain_index::set(&<Pallet<T>>::offchain_key(*position), elem)
			});
		}

		let peaks_after = mmr::helper::get_peaks(mmr_size);
		let nodes_to_prune = diff(&peaks_after, &peaks_before);
		let peaks_to_store = diff(&peaks_before, &peaks_after);

		{
			log::trace!("elems: {:?}\n", elems);
			log::trace!("peaks_before: {:?}", peaks_before);
			log::trace!("peaks_after: {:?}", peaks_after);
			log::trace!("nodes_to_prune: {:?}", nodes_to_prune);
			log::trace!("peaks_to_store: {:?}\n", peaks_to_store);
		}

		for position in nodes_to_prune {
			<Peaks<T>>::remove(position);
		}
		for position in peaks_to_store {
			if let Some(i) = elems
				.iter()
				.position(|(position_, _)| *position_ == position)
			{
				if let Some((_, elem)) = elems.get(i) {
					<Peaks<T>>::insert(position, elem);

					log::trace!("position: {}, elem: {:?}", position, elem);
				} else {
					log::error!("The different must existed in `elems`; qed");
				}
			} else {
				log::error!("The different must existed in `elems`; qed");
			}
		}

		Ok(())
	}
}

pub struct Mmr<StorageType, T>
where
	Storage<StorageType, T>: MMRStore<T::Hash>,
	T: Config,
{
	pub mmr: MMR<T::Hash, Hasher<T>, Storage<StorageType, T>>,
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

	pub fn get_root(&self) -> MMRResult<T::Hash> {
		self.mmr.get_root()
	}

	// TODO
	pub fn verify(&self) -> MMRResult<bool> {
		todo!()
	}
}
impl<T> Mmr<OffchainStorage, T>
where
	T: Config,
{
	pub fn gen_proof(&self, index: NodeIndex) -> MMRResult<MerkleProof<T::Hash, Hasher<T>>> {
		self.mmr.gen_proof(vec![mmr::leaf_index_to_pos(index)])
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
