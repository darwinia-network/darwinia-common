// --- substrate ---
use frame_support::{traits::FindAuthor, ConsensusEngineId};
use sp_core::{crypto::Public, H160, U256};
// --- darwinia ---
use crate::*;
use darwinia_evm::ConcatAddressMapping;
use dvm_ethereum::{Config, IntermediateStateRoot};

pub struct EthereumFindAuthor<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Babe::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.0.to_raw_vec()[4..24]));
		}
		None
	}
}
frame_support::parameter_types! {
	pub BlockGasLimit: U256 = U256::from(u32::max_value());
}
impl Config for Runtime {
	type Event = Event;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type StateRoot = IntermediateStateRoot;
	type BlockGasLimit = BlockGasLimit;
	type AddressMapping = ConcatAddressMapping;
	type RingCurrency = Ring;
}
