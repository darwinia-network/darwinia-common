//! # Darwinia-eth-linear-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::{H128, H512};
// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system as system;
use sp_runtime::DispatchError;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::relay::{RawHeaderThing, Relayable, TcHeaderId};
use eth_primitives::{header::EthHeader, EthBlockNumber, H256};

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
	type TcBlockNumber = EthBlockNumber;
	type TcHeaderHash = H256;

	fn highest_confirmed_at() -> Self::TcBlockNumber {
		unimplemented!()
	}

	fn verify_raw_header_thing<R: AsRef<RawHeaderThing>>(
		raw_header_thing: R,
	) -> Result<TcHeaderId<Self::TcBlockNumber, Self::TcHeaderHash>, DispatchError> {
		unimplemented!()
	}

	fn header_existed(block_number: Self::TcBlockNumber) -> bool {
		unimplemented!()
	}
}

#[derive(Encode, Decode)]
pub struct EthHeaderThing {
	header: EthHeader,
	ethash_proof: Vec<DoubleNodeWithMerkleProof>,
	// mmr: ?,
}

#[derive(Encode, Decode)]
pub struct DoubleNodeWithMerkleProof {
	dag_nodes: [H512; 2],
	proof: Vec<H128>,
}
