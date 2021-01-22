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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use sp_std::prelude::*;

#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Debug, PartialEq))]
pub struct Proof {
	pub nodes: Vec<Vec<u8>>,
}

impl Proof {
	pub fn to_rlp(&self) -> Vec<u8> {
		rlp::encode(self)
	}

	pub fn len(&self) -> usize {
		self.nodes.len()
	}
}

impl From<Vec<Vec<u8>>> for Proof {
	fn from(data: Vec<Vec<u8>>) -> Proof {
		Proof { nodes: data }
	}
}

impl Decodable for Proof {
	fn decode(r: &Rlp) -> Result<Self, DecoderError> {
		Ok(Proof {
			nodes: r.list_at(0)?,
		})
	}
}

impl Encodable for Proof {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(1);
		s.append_list::<Vec<u8>, Vec<u8>>(&self.nodes);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_encode_decode() {
		let nodes = vec![vec![0u8], vec![1], vec![2]];
		let expected = Proof { nodes };
		let rlp_proof = rlp::encode(&expected);
		let out_proof: Proof = rlp::decode(&rlp_proof).unwrap();
		println!("{:?}", out_proof);
		assert_eq!(expected, out_proof);
	}
}
