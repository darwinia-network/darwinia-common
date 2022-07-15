// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use codec::{Decode, Encode, FullCodec};
use scale_info::TypeInfo;
// --- crates.io ---
use frame_support::{
	traits::{Currency, Get},
	BoundedVec,
};
#[cfg(feature = "std")]
use serde::{de::DeserializeOwned, Serialize};
use sp_io::{crypto, hashing};
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::Config;

pub type EcdsaSigner = [u8; 20];
pub type EcdsaMessage = [u8; 32];
pub type EcdsaSignature = [u8; 65];

pub type OpCode = [u8; 4];
pub type Term = u32;

// Alias only.
pub(super) type AccountId<T> = <T as frame_system::Config>::AccountId;
pub(super) type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
pub(super) type MaxMembers<T> = <T as Config>::MaxMembers;
// Basics.
pub(super) type Balance<T> = <<T as Config>::Currency as Currency<AccountId<T>>>::Balance;
// Sign things.
pub(super) type RelayAuthoritySigner<T> = <<T as Config>::Sign as Sign>::Signer;
pub(super) type RelayAuthorityMessage<T> = <<T as Config>::Sign as Sign>::Message;
pub(super) type RelayAuthoritySignature<T> = <<T as Config>::Sign as Sign>::Signature;
// Authority things.
pub(super) type RelayAuthorityT<T> =
	RelayAuthority<AccountId<T>, RelayAuthoritySigner<T>, Balance<T>, BlockNumber<T>>;
pub(super) type ScheduledAuthoritiesChangeT<T> = ScheduledAuthoritiesChange<
	AccountId<T>,
	RelayAuthoritySigner<T>,
	Balance<T>,
	BlockNumber<T>,
	MaxMembers<T>,
>;

pub trait RelayAuthorityProtocol<BlockNumber> {
	type Signer;

	fn schedule_mmr_root(block_number: BlockNumber) -> DispatchResult;

	fn check_authorities_change_to_sync(
		term: Term,
		authorities: Vec<Self::Signer>,
	) -> DispatchResult;

	fn sync_authorities_change() -> DispatchResult;
}

pub trait Sign {
	type Signature: Clone + Debug + PartialEq + FullCodec + TypeInfo;
	type Message: Clone + Debug + Default + PartialEq + FullCodec + TypeInfo;
	#[cfg(feature = "std")]
	type Signer: Clone
		+ Debug
		+ Default
		+ Ord
		+ PartialEq
		+ FullCodec
		+ TypeInfo
		+ DeserializeOwned
		+ Serialize;
	#[cfg(not(feature = "std"))]
	type Signer: Clone + Debug + Default + Ord + PartialEq + FullCodec + TypeInfo;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message;

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool;
}
pub enum EcdsaSign {}
impl Sign for EcdsaSign {
	type Message = EcdsaMessage;
	type Signature = EcdsaSignature;
	type Signer = EcdsaSigner;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message {
		hashing::keccak_256(raw_message.as_ref())
	}

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool {
		fn eth_signable_message(message: &[u8]) -> Vec<u8> {
			let mut l = message.len();
			let mut rev = Vec::new();

			while l > 0 {
				rev.push(b'0' + (l % 10) as u8);
				l /= 10;
			}

			let mut v = b"\x19Ethereum Signed Message:\n".to_vec();

			v.extend(rev.into_iter().rev());
			v.extend_from_slice(message);

			v
		}

		let message = hashing::keccak_256(&eth_signable_message(message));

		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature, &message) {
			hashing::keccak_256(&public_key)[12..] == signer[..]
		} else {
			false
		}
	}
}

// Avoid duplicate type
// Use `RelayAuthority` instead `Authority`
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct RelayAuthority<AccountId, Signer, RingBalance, BlockNumber> {
	pub(super) account_id: AccountId,
	pub(super) signer: Signer,
	pub(super) stake: RingBalance,
	pub(super) term: BlockNumber,
}
impl<AccountId, Signer, RingBalance, BlockNumber> PartialEq
	for RelayAuthority<AccountId, Signer, RingBalance, BlockNumber>
where
	AccountId: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.account_id == other.account_id
	}
}
impl<AccountId, Signer, RingBalance, BlockNumber> PartialEq<AccountId>
	for RelayAuthority<AccountId, Signer, RingBalance, BlockNumber>
where
	AccountId: PartialEq,
{
	fn eq(&self, account_id: &AccountId) -> bool {
		&self.account_id == account_id
	}
}

/// The scheduled change of authority set.
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(MaxMembers))]
pub struct ScheduledAuthoritiesChange<AccountId, Signer, RingBalance, BlockNumber, MaxMembers>
where
	MaxMembers: Get<u32>,
{
	/// The incoming new authorities.
	pub(super) next_authorities:
		BoundedVec<RelayAuthority<AccountId, Signer, RingBalance, BlockNumber>, MaxMembers>,
	/// The deadline of the previous authorities to sign for the next authorities.
	pub(super) deadline: BlockNumber,
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(MaxMembers))]
pub struct MmrRootToSign<MmrRoot, AccountId, Signature, MaxMembers>
where
	MaxMembers: Get<u32>,
{
	pub(super) mmr_root: MmrRoot,
	pub(super) signatures: BoundedVec<(AccountId, Signature), MaxMembers>,
}
impl<MmrRoot, AccountId, Signature, MaxMembers>
	MmrRootToSign<MmrRoot, AccountId, Signature, MaxMembers>
where
	MaxMembers: Get<u32>,
{
	pub(super) fn new(mmr_root: MmrRoot) -> Self {
		Self { mmr_root, signatures: BoundedVec::default() }
	}
}

#[derive(Encode)]
pub(super) struct Message<_1, _2, _3, _4>
where
	_1: Encode,
	_2: Encode,
	_3: Encode,
	_4: Encode,
{
	pub(super) _1: _1,
	pub(super) _2: _2,
	#[codec(compact)]
	pub(super) _3: _3,
	pub(super) _4: _4,
}
