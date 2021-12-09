// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- core ---
#[cfg(any(feature = "full-rlp", test))]
use core::{cell::RefCell, mem};
use core::{
	cmp,
	convert::{From, Into, TryFrom},
};
// --- std ---
#[cfg(feature = "std")]
use std::collections::BTreeMap;
// --- alloc ---
#[cfg(all(not(feature = "std"), any(feature = "full-rlp", test)))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
// --- crates.io ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
use keccak_hash::KECCAK_EMPTY_LIST_RLP;
#[cfg(any(feature = "full-rlp", test))]
use rlp::Rlp;
use sp_debug_derive::RuntimeDebug;
// --- darwinia-network ---
#[cfg(any(feature = "full-rlp", test))]
use crate::{
	error::{BlockError, Error, Mismatch, OutOfBounds, ProofError, RlpError},
	ethashproof::EthashProof,
	H128,
};
use crate::{header::Header, BigEndianHash, BlockNumber, H256, H64, U256, U512};

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Default, PartialEq, Eq, Clone)]
pub struct EthashPartial {
	pub minimum_difficulty: U256,
	pub difficulty_bound_divisor: U256,
	pub difficulty_increment_divisor: u64,
	pub metropolis_difficulty_increment_divisor: u64,
	pub duration_limit: u64,
	pub homestead_transition: u64,
	pub difficulty_hardfork_transition: u64,
	pub difficulty_hardfork_bound_divisor: U256,
	pub bomb_defuse_transition: u64,
	pub eip100b_transition: u64,
	pub ecip1010_pause_transition: u64,
	pub ecip1010_continue_transition: u64,
	pub difficulty_bomb_delays: BTreeMap<BlockNumber, BlockNumber>,
	pub expip2_transition: u64,
	pub expip2_duration_limit: u64,
	pub progpow_transition: u64,
}
impl EthashPartial {
	pub fn set_difficulty_bomb_delays(&mut self, key: BlockNumber, value: BlockNumber) {
		self.difficulty_bomb_delays.insert(key, value);
	}

	pub fn expanse() -> Self {
		EthashPartial {
			minimum_difficulty: 0x20000.into(),
			difficulty_bound_divisor: 0x0800.into(),
			difficulty_increment_divisor: 0x3C,
			metropolis_difficulty_increment_divisor: 0x1E,
			duration_limit: 0x3C,
			homestead_transition: 0x30d40,
			difficulty_hardfork_transition: 0x59d9,
			difficulty_hardfork_bound_divisor: 0x0200.into(),
			bomb_defuse_transition: 0x30d40,
			eip100b_transition: 0xC3500,
			ecip1010_pause_transition: 0x2dc6c0,
			ecip1010_continue_transition: 0x4c4b40,
			difficulty_bomb_delays: BTreeMap::<BlockNumber, BlockNumber>::default(),
			expip2_transition: 0xc3500,
			expip2_duration_limit: 0x1e,
			progpow_transition: u64::MAX,
		}
	}

	pub fn production() -> Self {
		EthashPartial {
			minimum_difficulty: 0x20000.into(),
			difficulty_bound_divisor: 0x0800.into(),
			difficulty_increment_divisor: 10,
			metropolis_difficulty_increment_divisor: 9,
			duration_limit: 13,
			homestead_transition: 1150000,
			difficulty_hardfork_transition: u64::MAX,
			difficulty_hardfork_bound_divisor: 2048.into(),
			bomb_defuse_transition: u64::MAX,
			eip100b_transition: 4370000,
			ecip1010_pause_transition: u64::MAX,
			ecip1010_continue_transition: u64::MAX,
			difficulty_bomb_delays: BTreeMap::from([
				(4370000, 3000000),
				(7280000, 2000000),
				(0x8c6180, 0x3d0900),
				(0xc5d488, 0xaae60),
				(0xd228c8, 0xf4240),
			]),
			expip2_transition: u64::MAX,
			expip2_duration_limit: 30,
			progpow_transition: u64::MAX,
		}
	}

	pub fn ropsten_testnet() -> Self {
		EthashPartial {
			minimum_difficulty: 0x20000.into(),
			difficulty_bound_divisor: 0x0800.into(),
			difficulty_increment_divisor: 10,
			metropolis_difficulty_increment_divisor: 9,
			duration_limit: 0xd,
			homestead_transition: 0x0,
			difficulty_hardfork_transition: 0x59d9,
			difficulty_hardfork_bound_divisor: 0x0800.into(),
			bomb_defuse_transition: u64::MAX,
			eip100b_transition: 0x19f0a0,
			ecip1010_pause_transition: u64::MAX,
			ecip1010_continue_transition: u64::MAX,
			difficulty_bomb_delays: {
				let mut m = BTreeMap::new();
				m.insert(0x19f0a0, 0x2dc6c0);
				m.insert(0x408b70, 0x1e8480);
				m.insert(0x6c993d, 0x3d0900);
				m.insert(0xa03549, 0xaae60);
				m
			},
			expip2_transition: u64::MAX,
			expip2_duration_limit: 30,
			progpow_transition: u64::MAX,
		}
	}
}

impl EthashPartial {
	#[cfg(any(feature = "full-rlp", test))]
	pub fn verify_seal_with_proof(
		self,
		header: &Header,
		ethash_proof: &[EthashProof],
		merkle_root: &H128,
	) -> Result<(), Error> {
		let seal = Seal::parse_seal(header.seal())?;
		let (mix_hash, _result) = self.hashimoto_merkle(
			&header.bare_hash(),
			&seal.nonce,
			header.number,
			ethash_proof,
			merkle_root,
		)?;

		if mix_hash != seal.mix_hash {
			Err(BlockError::InvalidSeal)?;
		}

		Ok(())
	}

	#[cfg(any(feature = "full-rlp", test))]
	fn hashimoto_merkle(
		self,
		header_hash: &H256,
		nonce: &H64,
		block_number: u64,
		nodes: &[EthashProof],
		merkle_root: &H128,
	) -> Result<(H256, H256), ProofError> {
		// Boxed index since ethash::hashimoto gets Fn, but not FnMut
		let index = RefCell::new(0);
		let maybe_error = RefCell::new(None);

		let pair = ethash::hashimoto(
			*header_hash,
			*nonce,
			ethash::get_full_size(block_number as usize / 30000),
			|offset| {
				if maybe_error.borrow().is_some() {
					return Default::default();
				}

				let index = index.replace_with(|&mut old| old + 1);
				// Each two nodes are packed into single 128 bytes with Merkle proof
				let node = if let Some(node) = nodes.get(index / 2) {
					node
				} else {
					maybe_error.replace(Some(ProofError::MerkleProofOutOfRange(OutOfBounds {
						min: Some(0),
						max: Some(nodes.len()),
						found: index / 2,
					})));

					return Default::default();
				};

				let index_mod_2 = index % 2;

				if index_mod_2 == 0 {
					let expected_merkle_root = merkle_root.to_owned();
					// Divide by 2 to adjust offset for 64-byte words instead of 128-byte
					let calculated_merkle_root = node.apply_merkle_proof((offset / 2) as u64);

					if expected_merkle_root != calculated_merkle_root {
						maybe_error.replace(Some(ProofError::MerkleRootMismatch(Mismatch {
							expected: expected_merkle_root,
							found: calculated_merkle_root,
						})));

						return Default::default();
					}
				};

				// Reverse each 32 bytes for ETHASH compatibility
				// `index_mod_2` will never get out of range on `[H512; 2]`
				let mut data = node.dag_nodes[index_mod_2];
				data[..32].reverse();
				data[32..].reverse();

				data.into()
			},
		);

		if let Some(e) = maybe_error.into_inner() {
			Err(e)
		} else {
			Ok(pair)
		}
	}

	#[cfg(any(feature = "full-rlp", test))]
	pub fn verify_block_basic(&self, header: &Header) -> Result<(), Error> {
		// check the seal fields.
		let seal = Seal::parse_seal(header.seal())?;
		// TODO: consider removing these lines.
		let min_difficulty = self.minimum_difficulty;

		if header.difficulty() < &min_difficulty {
			Err(BlockError::DifficultyOutOfBounds(OutOfBounds {
				min: Some(min_difficulty),
				max: None,
				found: *header.difficulty(),
			}))?;
		}

		let difficulty = boundary_to_difficulty(&H256(quick_get_difficulty(
			&header.bare_hash().0,
			seal.nonce.to_low_u64_be(),
			&seal.mix_hash.0,
			header.number() >= self.progpow_transition,
		)));

		if &difficulty < header.difficulty() {
			Err(BlockError::InvalidProofOfWork(OutOfBounds {
				min: Some(*header.difficulty()),
				max: None,
				found: difficulty,
			}))?;
		}

		Ok(())
	}

	pub fn calculate_difficulty(&self, header: &Header, parent: &Header) -> U256 {
		const EXP_DIFF_PERIOD: u64 = 100_000;

		if header.number() == 0 {
			panic!("Can't calculate genesis block difficulty");
		}

		let parent_has_uncles = parent.uncles_hash() != &KECCAK_EMPTY_LIST_RLP;

		let min_difficulty = self.minimum_difficulty;

		let difficulty_hardfork = header.number() >= self.difficulty_hardfork_transition;
		let difficulty_bound_divisor = if difficulty_hardfork {
			self.difficulty_hardfork_bound_divisor
		} else {
			self.difficulty_bound_divisor
		};

		let expip2_hardfork = header.number() >= self.expip2_transition;
		let duration_limit = if expip2_hardfork {
			self.expip2_duration_limit
		} else {
			self.duration_limit
		};

		let frontier_limit = self.homestead_transition;

		let mut target = if header.number() < frontier_limit {
			if header.timestamp() >= parent.timestamp() + duration_limit {
				*parent.difficulty() - (*parent.difficulty() / difficulty_bound_divisor)
			} else {
				*parent.difficulty() + (*parent.difficulty() / difficulty_bound_divisor)
			}
		} else {
			//		trace!(target: "ethash", "Calculating difficulty parent.difficulty={}, header.timestamp={}, parent.timestamp={}", parent.difficulty(), header.timestamp(), parent.timestamp());
			//block_diff = parent_diff + parent_diff // 2048 * max(1 - (block_timestamp - parent_timestamp) // 10, -99)
			let (increment_divisor, threshold) = if header.number() < self.eip100b_transition {
				(self.difficulty_increment_divisor, 1)
			} else if parent_has_uncles {
				(self.metropolis_difficulty_increment_divisor, 2)
			} else {
				(self.metropolis_difficulty_increment_divisor, 1)
			};

			let diff_inc = (header.timestamp() - parent.timestamp()) / increment_divisor;
			if diff_inc <= threshold {
				*parent.difficulty()
					+ *parent.difficulty() / difficulty_bound_divisor
						* U256::from(threshold - diff_inc)
			} else {
				let multiplier: U256 = cmp::min(diff_inc - threshold, 99).into();
				parent
					.difficulty()
					.saturating_sub(*parent.difficulty() / difficulty_bound_divisor * multiplier)
			}
		};
		target = cmp::max(min_difficulty, target);
		if header.number() < self.bomb_defuse_transition {
			if header.number() < self.ecip1010_pause_transition {
				let mut number = header.number();
				let original_number = number;
				for (block, delay) in &self.difficulty_bomb_delays {
					if original_number >= *block {
						number = number.saturating_sub(*delay);
					}
				}
				let period = (number / EXP_DIFF_PERIOD) as usize;
				if period > 1 {
					target = cmp::max(min_difficulty, target + (U256::from(1) << (period - 2)));
				}
			} else if header.number() < self.ecip1010_continue_transition {
				let fixed_difficulty =
					((self.ecip1010_pause_transition / EXP_DIFF_PERIOD) - 2) as usize;
				target = cmp::max(min_difficulty, target + (U256::from(1) << fixed_difficulty));
			} else {
				let period = ((parent.number() + 1) / EXP_DIFF_PERIOD) as usize;
				let delay = ((self.ecip1010_continue_transition - self.ecip1010_pause_transition)
					/ EXP_DIFF_PERIOD) as usize;
				target = cmp::max(
					min_difficulty,
					target + (U256::from(1) << (period - delay - 2)),
				);
			}
		}
		target
	}
}

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct Seal {
	/// Ethash seal mix_hash
	pub mix_hash: H256,
	/// Ethash seal nonce
	pub nonce: H64,
}
impl Seal {
	/// Tries to parse rlp encoded bytes as an Ethash/Clique seal.
	#[cfg(any(feature = "full-rlp", test))]
	pub fn parse_seal<T: AsRef<[u8]>>(seal: &[T]) -> Result<Self, Error> {
		if seal.len() != 2 {
			Err(BlockError::InvalidSealArity(Mismatch {
				expected: 2,
				found: seal.len(),
			}))?;
		}

		let mix_hash = Rlp::new(seal[0].as_ref())
			.as_val::<H256>()
			.map_err(RlpError::from)?;
		let nonce = Rlp::new(seal[1].as_ref())
			.as_val::<H64>()
			.map_err(RlpError::from)?;
		let seal = Seal { mix_hash, nonce };

		Ok(seal)
	}
}

pub fn boundary_to_difficulty(boundary: &H256) -> U256 {
	difficulty_to_boundary_aux(&boundary.into_uint())
}

fn difficulty_to_boundary_aux<T: Into<U512>>(difficulty: T) -> U256 {
	let difficulty = difficulty.into();

	assert!(!difficulty.is_zero());

	if difficulty == U512::one() {
		U256::max_value()
	} else {
		const PROOF: &str = "difficulty > 1, so result never overflows 256 bits; qed";
		U256::try_from((U512::one() << 256) / difficulty).expect(PROOF)
	}
}

#[cfg(any(feature = "full-rlp", test))]
fn quick_get_difficulty(
	header_hash: &[u8; 32],
	nonce: u64,
	mix_hash: &[u8; 32],
	_progpow: bool,
) -> [u8; 32] {
	let mut first_buf = [0u8; 40];
	let mut buf = [0u8; 64 + 32];

	let hash_len = header_hash.len();
	first_buf[..hash_len].copy_from_slice(header_hash);
	first_buf[hash_len..hash_len + mem::size_of::<u64>()].copy_from_slice(&nonce.to_ne_bytes());

	keccak_hash::keccak_512(&first_buf, &mut buf);
	buf[64..].copy_from_slice(mix_hash);

	let mut hash = [0u8; 32];
	keccak_hash::keccak_256(&buf, &mut hash);

	hash

	//	let mut buf = [0u8; 64 + 32];
	//
	//	#[cfg(feature = "std")]
	//	unsafe {
	//		let hash_len = header_hash.len();
	//		buf[..hash_len].copy_from_slice(header_hash);
	//		buf[hash_len..hash_len + mem::size_of::<u64>()].copy_from_slice(&nonce.to_ne_bytes());
	//
	//		keccak_512::unchecked(buf.as_mut_ptr(), 64, buf.as_ptr(), 40);
	//		buf[64..].copy_from_slice(mix_hash);
	//
	//		let mut hash = [0u8; 32];
	//		keccak_256::unchecked(hash.as_mut_ptr(), hash.len(), buf.as_ptr(), buf.len());
	//
	//		hash
	//	}
}
