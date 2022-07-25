// --- crates.io ---
use codec::Encode;
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
	pub(crate) fn hash(raw_message: impl AsRef<[u8]>) -> Message {
		hashing::keccak_256(raw_message.as_ref())
	}

	pub(crate) fn verify_signature(
		signature: &Signature,
		message: &Message,
		address: &Address,
	) -> bool {
		fn eth_signable_message(message: &[u8]) -> Vec<u8> {
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

#[derive(Encode)]
pub(crate) struct RelayMessage<_1, _2, _3, _4, _5, _6>
where
	_1: Encode,
	_2: Encode,
	_3: Encode,
	_4: Encode,
	_5: Encode,
	_6: Encode,
{
	pub(crate) _1: _1,
	pub(crate) _2: _2,
	pub(crate) _3: _3,
	pub(crate) _4: _4,
	pub(crate) _5: _5,
	pub(crate) _6: _6,
}