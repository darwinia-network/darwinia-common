//! # Darwinia-ethereum-linear-relay Module

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethereum_types::{H128, H512};
// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system as system;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::{convert::From, prelude::*};
// --- darwinia ---
use darwinia_support::relay::*;
use ethereum_primitives::{
	header::EthHeader,
	merkle::DoubleNodeWithMerkleProof,
	pow::{EthashPartial, EthashSeal},
	EthBlockNumber, H256,
};

// TODO: MMR type
type EthereumMMR = ();

pub trait Trait<I: Instance = DefaultInstance>:
	frame_system::Trait + darwinia_ethereum_linear_relay::Trait
{
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
		TargetHeaderAE,
		HeaderInvalid,
		NotComplyWithConfirmebBlocks
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaEthereumRelay {
		/// Ethereum last confrimed header info including ethereum block number, hash, and mmr
		LastConfirmedHeaderInfo get(fn last_confirm_header_info): Option<(EthBlockNumber, H256, EthereumMMR)>;

		/// The Ethereum headers confrimed by relayer game
		/// The actural storage needs to be defined
		ConfirmedHeaders get(fn confirmed_headers): map hasher(identity) EthBlockNumber => EthHeader;

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

impl<T: Trait<I>, I: Instance> Module<T, I> {
	/// validate block with the hash, difficulty of confirmed headers
	fn verify_block_with_confrim_blocks(header: &EthHeader) -> bool {
		let eth_partial = EthashPartial::production();
		if ConfirmedHeaders::<I>::contains_key(header.number - 1) {
			let previous_header = ConfirmedHeaders::<I>::get(header.number - 1);
			if header.parent_hash != previous_header.hash.unwrap_or_default()
				|| *header.difficulty()
					!= eth_partial.calculate_difficulty(header, &previous_header)
			{
				return false;
			}
		}

		if ConfirmedHeaders::<I>::contains_key(header.number + 1) {
			let subsequent_header = ConfirmedHeaders::<I>::get(header.number + 1);
			if header.hash.unwrap_or_default() != subsequent_header.parent_hash
				|| *subsequent_header.difficulty()
					!= eth_partial.calculate_difficulty(&subsequent_header, header)
			{
				return false;
			}
		}
		true
	}

	fn verify_block_seal(header: &EthHeader, ethash_proof: &[DoubleNodeWithMerkleProof]) -> bool {
		if header.hash() != header.re_compute_hash() {
			return false;
		}

		let eth_partial = EthashPartial::production();

		if eth_partial.verify_block_basic(header).is_err() {
			return false;
		}

		if eth_partial.verify_block_basic(header).is_err() {
			return false;
		}

		let merkle_root = <darwinia_ethereum_linear_relay::Module<T>>::dag_merkle_root(
			(header.number as usize / 30000) as u64,
		);
		if eth_partial
			.verify_seal_with_proof(&header, &ethash_proof, &merkle_root)
			.is_err()
		{
			return false;
		};

		return true;
	}
}

impl<T: Trait<I>, I: Instance> Relayable for Module<T, I> {
	type TcBlockNumber = EthBlockNumber;
	type TcHeaderHash = H256;
	type TcHeaderMMR = EthereumMMR;

	fn last_confirmed() -> Self::TcBlockNumber {
		return if let Some(i) = LastConfirmedHeaderInfo::<I>::get() {
			i.0
		} else {
			0u64.into()
		};
	}

	fn header_existed(block_number: Self::TcBlockNumber) -> bool {
		ConfirmedHeaders::<I>::contains_key(block_number)
	}

	fn verify_raw_header_thing(
		raw_header_thing: RawHeaderThing,
	) -> Result<
		TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
		DispatchError,
	> {
		let EthHeaderThing {
			header,
			ethash_proof,
			mmr: _mmr,
		} = raw_header_thing.into();

		if ConfirmedHeaders::<I>::contains_key(header.number) {
			return Err(<Error<T, I>>::TargetHeaderAE)?;
		}
		if !Self::verify_block_seal(&header, &ethash_proof) {
			return Err(<Error<T, I>>::HeaderInvalid)?;
		};

		Ok(vec![TcHeaderThing::BlockNumber(header.number)])
	}

	/// verify ethereum headers with seal, hash, and difficulty
	fn verify_raw_header_thing_chain(
		raw_header_thing_chain: Vec<RawHeaderThing>,
	) -> Result<
		Vec<TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>>,
		DispatchError,
	> {
		let output = vec![];
		for raw_header_thing in raw_header_thing_chain {
			let EthHeaderThing {
				header,
				ethash_proof,
				mmr: _mmr,
			} = raw_header_thing.into();

			if !Self::verify_block_seal(&header, &ethash_proof) {
				return Err(<Error<T, I>>::HeaderInvalid)?;
			};

			if !Self::verify_block_with_confrim_blocks(&header) {
				return Err(<Error<T, I>>::NotComplyWithConfirmebBlocks)?;
			}
		}
		Ok(output)
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

	fn store_header(raw_header_thing: RawHeaderThing) -> DispatchResult {
		let last_comfirmed_block_number = if let Some(i) = LastConfirmedHeaderInfo::<I>::get() {
			i.0
		} else {
			0
		};
		let EthHeaderThing {
			header,
			ethash_proof: _,
			mmr,
		} = raw_header_thing.into();

		if header.number > last_comfirmed_block_number {
			LastConfirmedHeaderInfo::<I>::set(Some((
				header.number,
				header.hash.unwrap_or_default(),
				mmr,
			)))
		};

		ConfirmedHeaders::<I>::insert(header.number, header);

		Ok(())
	}
}

#[derive(Encode, Decode, Default)]
pub struct EthHeaderThing {
	header: EthHeader,
	ethash_proof: Vec<DoubleNodeWithMerkleProof>,
	mmr: EthereumMMR,
}

impl From<RawHeaderThing> for EthHeaderThing {
	fn from(raw_header_thing: RawHeaderThing) -> Self {
		EthHeaderThing::decode(&mut &*raw_header_thing).unwrap_or_default()
	}
}
