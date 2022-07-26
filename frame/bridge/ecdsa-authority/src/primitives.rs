pub(crate) use sp_core::ecdsa::Signature;

// --- paritytech ---
use sp_core::{H160, H256};
use sp_io::{crypto, hashing};

pub(crate) type Address = H160;
pub(crate) type Hash = H256;
pub(crate) type Message = [u8; 32];

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
	fn hash(data: &[u8]) -> [u8; 32] {
		hashing::keccak_256(data).into()
	}

	pub(crate) fn eth_signable_message(data: &[u8]) -> Message {
		// "\x19\x01" + keccak256("EcdsaAuthority()")
		let mut v = vec![
			25, 1, 101, 44, 46, 220, 101, 125, 125, 234, 202, 24, 100, 124, 39, 60, 190, 127, 35,
			130, 10, 168, 215, 250, 243, 136, 57, 63, 133, 96, 239, 199, 15, 135,
		];
		let message = Self::hash(data);

		v.extend_from_slice(&message);

		Self::hash(&v)
	}

	pub(crate) fn verify_signature(
		signature: &Signature,
		message: &Message,
		address: &Address,
	) -> bool {
		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature.as_ref(), &message) {
			Self::hash(&public_key)[12..] == address[..]
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
