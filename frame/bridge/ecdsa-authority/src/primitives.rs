// --- paritytech ---
use sp_core::{H160, H256};
use sp_io::{crypto, hashing};
use sp_std::prelude::*;

pub(crate) type Address = H160;
pub(crate) type Hash = H256;
pub(crate) type Message = [u8; 32];
pub(crate) type Signature = [u8; 65];

// address(0x1)
pub(crate) const AUTHORITY_SENTINEL: H160 =
	H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
// keccak256("ChangeRelayer(bytes32 network,bytes4 sig,bytes params,uint256 nonce)");
pub(crate) const RELAY_TYPE_HASH: H256 = H256([
	3, 36, 202, 12, 164, 213, 41, 224, 238, 252, 198, 209, 35, 189, 23, 236, 152, 36, 152, 207, 46,
	115, 33, 96, 204, 71, 210, 80, 72, 37, 228, 178,
]);
// keccak256("SignCommitment(bytes32 network,bytes32 commitment,uint256 nonce)");
pub(crate) const COMMIT_TYPE_HASH: H256 = H256([
	9, 64, 53, 206, 220, 62, 70, 239, 84, 120, 16, 153, 130, 131, 113, 234, 48, 235, 223, 241, 173,
	144, 226, 255, 196, 208, 61, 76, 80, 87, 251, 230,
]);

pub(crate) enum Sign {}
impl Sign {
	pub(crate) fn hash(data: &[u8]) -> Message {
		hashing::keccak_256(data).into()
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

			// "\x19\x01" + keccak256("EcdsaAuthority()")
			let mut v = vec![
				25, 1, 101, 44, 46, 220, 101, 125, 125, 234, 202, 24, 100, 124, 39, 60, 190, 127,
				35, 130, 10, 168, 215, 250, 243, 136, 57, 63, 133, 96, 239, 199, 15, 135,
			];

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
			// bytes4(keccak256("add_relayer_with_threshold(address,uint256)"))
			Method::AddMember { .. } => [178, 143, 99, 28],
			// bytes4(keccak256("remove_relayer(address,address,uint256)"))
			Method::RemoveMember { .. } => [134, 33, 209, 250],
			// bytes4(keccak256("swap_relayer(address,address,address)"))
			Method::SwapMembers { .. } => [203, 118, 8, 91],
		}
	}
}
