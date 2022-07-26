// --- crates.io ---
use sp_io::{crypto, hashing};
use sp_std::prelude::*;

pub(crate) type Address = [u8; 20];
pub(crate) type Message = [u8; 32];
pub(crate) type Signature = [u8; 65];

pub(crate) const AUTHORITY_SENTINEL: [u8; 20] =
	[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

pub(crate) enum Sign {}
impl Sign {
	pub(crate) fn hash(data: &[u8]) -> Message {
		hashing::keccak_256(data)
	}

	pub(crate) fn verify_signature(
		signature: &Signature,
		message: &Message,
		address: &Address,
	) -> bool {
		fn eth_signable_message(message: &Message) -> Vec<u8> {
			let mut l = message.len();
			let mut rev = Vec::new();

			while l > 0 {
				rev.push(b'0' + (l % 10) as u8);
				l /= 10;
			}

			let mut v = b"\x19\x01".to_vec();

			v.extend(rev.into_iter().rev());
			v.extend_from_slice(message);

			v
		}

		let message = hashing::keccak_256(&eth_signable_message(message));

		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature, &message) {
			hashing::keccak_256(&public_key)[12..] == address[..]
		} else {
			false
		}
	}
}

pub(crate) enum Method {
	AddMember { new: Address },
	RemoveMember { pre: Address, old: Address },
	SwapMembers { pre: Address, old: Address, new: Address },
}
impl Method {
	pub(crate) fn id(&self) -> [u8; 4] {
		match self {
			Method::AddMember { .. } => [178, 143, 99, 28],
			Method::RemoveMember { .. } => [134, 33, 209, 250],
			Method::SwapMembers { .. } => [203, 118, 8, 91],
		}
	}
}
