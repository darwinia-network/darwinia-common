pub(crate) use sp_core::ecdsa::Signature;

// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use sp_core::{H160, H256};
use sp_io::{crypto, hashing};
use sp_runtime::RuntimeDebug;

pub(crate) type Address = H160;
pub(crate) type Hash = H256;
pub(crate) type Message = [u8; 32];

// address(0x1)
pub(crate) const AUTHORITY_SENTINEL: H160 =
	H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
// keccak256("ChangeRelayer(bytes4 sig,bytes params,uint256 nonce)");
// 0x30a82982a8d5050d1c83bbea574aea301a4d317840a8c4734a308ffaa6a63bc8
pub(crate) const RELAY_TYPE_HASH: H256 = H256([
	3, 36, 202, 12, 164, 213, 41, 224, 238, 252, 198, 209, 35, 189, 23, 236, 152, 36, 152, 207, 46,
	115, 33, 96, 204, 71, 210, 80, 72, 37, 228, 178,
]);
// keccak256("Commitment(uint32 block_number, bytes32 message_root, uint256 nonce)");
// 0x1927575a20e860281e614acf70aa85920a1187ed2fb847ee50d71702e80e2b8f
pub(crate) const COMMIT_TYPE_HASH: H256 = H256([
	9, 64, 53, 206, 220, 62, 70, 239, 84, 120, 16, 153, 130, 131, 113, 234, 48, 235, 223, 241, 173,
	144, 226, 255, 196, 208, 61, 76, 80, 87, 251, 230,
]);

pub(crate) enum Sign {}
impl Sign {
	fn hash(data: &[u8]) -> [u8; 32] {
		hashing::keccak_256(data).into()
	}

	pub(crate) fn eth_signable_message(chain_id: &[u8], spec_name: &[u8], data: &[u8]) -> Message {
		// \x19\01 + keccack256(ChainIDSpecName::ecdsa-authority) + struct_hash
		Self::hash(
			&[
				b"\x19\01".as_slice(),
				&Self::hash(&[chain_id, spec_name, b"::ecdsa-authority"].concat()),
				data,
			]
			.concat(),
		)
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

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Operation {
	AddMember { new: Address },
	RemoveMember { pre: Address, old: Address },
	SwapMembers { pre: Address, old: Address, new: Address },
}
impl Operation {
	pub(crate) fn id(&self) -> [u8; 4] {
		match self {
			// bytes4(keccak256("add_relayer(address,uint256)"))
			// 0xb7aafe32
			Self::AddMember { .. } => [183, 170, 254, 50],
			// bytes4(keccak256("remove_relayer(address,address,uint256)"))
			// 0x8621d1fa
			Self::RemoveMember { .. } => [134, 33, 209, 250],
			// bytes4(keccak256("swap_relayer(address,address,address)"))
			// 0xcb76085b
			Self::SwapMembers { .. } => [203, 118, 8, 91],
		}
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Commitment {
	pub(crate) block_number: u32,
	pub(crate) message_root: Hash,
	pub(crate) nonce: u32,
}

#[test]
fn eth_signable_message() {
	assert_eq!(
		array_bytes::bytes2hex("0x", &Sign::eth_signable_message(b"46", b"Darwinia", &[0; 32])),
		"0x8c2f82fe9a2be0813e57092c9dd86742130362f7d552992b9a17c96d64945cb1"
	);
	assert_eq!(
		array_bytes::bytes2hex("0x", &Sign::hash(b"46Darwinia::ecdsa-authority")),
		"0xf8a76f5ceeff36d74ff99c4efc0077bcc334721f17d1d5f17cfca78455967e1e"
	);
}
