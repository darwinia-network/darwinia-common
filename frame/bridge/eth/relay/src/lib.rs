//! # Darwinia-eth-linear-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system as system;
use sp_runtime::DispatchError;
// --- darwinia ---
use darwinia_support::relay::Relayable;
use eth_primitives::{EthBlockNumber, H256};

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		<T as frame_system::Trait>::AccountId,
	{
		/// TODO
		TODO(AccountId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaEthRelay {
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;
	}
}

impl<T: Trait<I>, I: Instance> Relayable for Module<T, I> {
	type BlockNumber = EthBlockNumber;
	type HeaderHash = H256;

	fn highest_confirmed_tc_header_id() -> (Self::BlockNumber, Self::HeaderHash) {
		unimplemented!()
	}

	fn verify<S: AsRef<[u8]>>(
		header_thing: S,
	) -> Result<(Self::BlockNumber, Self::HeaderHash), DispatchError> {
		unimplemented!()
	}
}
