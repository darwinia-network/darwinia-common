//! # Darwinia-eth-linear-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::{H128, H512};
// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system as system;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::relay::*;
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
	// TODO: MMR type
	type TcHeaderMMR = ();

	fn last_confirmed() -> Self::TcBlockNumber {
		unimplemented!()
	}

	fn header_existed(block_number: Self::TcBlockNumber) -> bool {
		unimplemented!()
	}

	fn verify_raw_header_thing(
		raw_header_thing: RawHeaderThing,
	) -> Result<
		TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
		DispatchError,
	> {
		unimplemented!()
	}

	/// Eth additional `Other` fileds in `Vec<TcHeaderBrief>`:
	/// 	[
	///			...,
	/// 		Difficulty (shoule be in addition field `Other`, bytes style),
	/// 		Total Difficulty (shoule be in addition field `Other`, bytes style),
	/// 	]
	fn verify_raw_header_thing_chain(
		raw_header_thing_chain: Vec<RawHeaderThing>,
	) -> Result<
		Vec<TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>>,
		DispatchError,
	> {
		// TODO: also verify continuous here for eth
		unimplemented!()
	}

	fn on_chain_arbitrate(
		header_thing_brief_chain: Vec<
			darwinia_support::relay::TcHeaderBrief<
				Self::TcBlockNumber,
				Self::TcHeaderHash,
				Self::TcHeaderMMR,
			>,
		>,
	) -> DispatchResult {
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
