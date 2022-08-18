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
	48, 168, 41, 130, 168, 213, 5, 13, 28, 131, 187, 234, 87, 74, 234, 48, 26, 77, 49, 120, 64,
	168, 196, 115, 74, 48, 143, 250, 166, 166, 59, 200,
]);
// keccak256("Commitment(uint32 block_number,bytes32 message_root,uint256 nonce)");
// 0xaca824a0c4edb3b2c17f33fea9cb21b33c7ee16c8e634c36b3bf851c9de7a223
pub(crate) const COMMIT_TYPE_HASH: H256 = H256([
	172, 168, 36, 160, 196, 237, 179, 178, 193, 127, 51, 254, 169, 203, 33, 179, 60, 126, 225, 108,
	142, 99, 76, 54, 179, 191, 133, 28, 157, 231, 162, 35,
]);

pub(crate) enum Sign {}
impl Sign {
	fn hash(data: &[u8]) -> [u8; 32] {
		hashing::keccak_256(data)
	}

	pub(crate) fn eth_signable_message(chain_id: &[u8], spec_name: &[u8], data: &[u8]) -> Message {
		// \x19\x01 + keccack256(ChainIDSpecName::ecdsa-authority) + struct_hash
		Self::hash(
			&[
				b"\x19\x01".as_slice(),
				&Self::hash(&[chain_id, spec_name, b"::ecdsa-authority"].concat()),
				&Self::hash(data),
			]
			.concat(),
		)
	}

	pub(crate) fn verify_signature(
		signature: &Signature,
		message: &Message,
		address: &Address,
	) -> bool {
		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature.as_ref(), message) {
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
		"0x2abcc6d89747121e6b6005700ccf065ec5c86359c7a2cae4d00b3c4f90f76e84"
	);
	assert_eq!(
		array_bytes::bytes2hex("0x", &Sign::hash(b"46Darwinia::ecdsa-authority")),
		"0xf8a76f5ceeff36d74ff99c4efc0077bcc334721f17d1d5f17cfca78455967e1e"
	);

	let data = array_bytes::hex2bytes_unchecked("0x30a82982a8d5050d1c83bbea574aea301a4d317840a8c4734a308ffaa6a63bc8cb76085b00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000100000000000000000000000068898db1012808808c903f390909c52d9f7067490000000000000000000000004cdc1dbbd754ea539f1ffaea91f1b6c4b8dd14bd");
	assert_eq!(
		array_bytes::bytes2hex("0x", &Sign::eth_signable_message(b"45", b"Pangoro", &data)),
		"0xc0cc97a3b7ce329120e03f03675fd4cc569f50bc9b792bbd40becd79c37badac"
	);

	let operation = Operation::SwapMembers {
		pre: AUTHORITY_SENTINEL,
		old: AUTHORITY_SENTINEL,
		new: AUTHORITY_SENTINEL,
	};
	let encoded = ethabi::encode(&[
		ethabi::Token::FixedBytes(RELAY_TYPE_HASH.as_ref().into()),
		ethabi::Token::FixedBytes(operation.id().into()),
		ethabi::Token::Bytes(ethabi::encode(&[
			ethabi::Token::Address(AUTHORITY_SENTINEL),
			ethabi::Token::Address(AUTHORITY_SENTINEL),
			ethabi::Token::Address(AUTHORITY_SENTINEL),
		])),
		ethabi::Token::Uint(0.into()),
	]);
	assert_eq!(
		array_bytes::bytes2hex("0x", &Sign::eth_signable_message(b"45", b"Pangoro", &encoded)),
		"0xe0048b398f49e08acbe5d5acc8beceecf2734c2cd4e73ec683302822ecc8811e"
	);
}
