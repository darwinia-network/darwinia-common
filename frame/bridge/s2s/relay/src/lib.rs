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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

// --- crates ---
use ethereum_primitives::EthereumAddress;
use sha3::Digest;
// --- substrate ---
use bp_runtime::{derive_account_id, ChainId, Size, SourceAccount};
use frame_support::{
	dispatch::{Dispatchable, PostDispatchInfo},
	pallet_prelude::*,
	Parameter,
};
use frame_support::{traits::Get, PalletId};
use frame_system::{pallet_prelude::*, RawOrigin};
use sp_core::hash::H256;
use sp_runtime::{
	traits::{AccountIdConversion, Convert},
	AccountId32, DispatchError,
};
// --- darwinia ---
use darwinia_relay_primitives::{Relay, RelayAccount, RelayDigest};
use darwinia_support::traits::CallToPayload;
use dp_asset::{token::Token, BridgedAssetReceiver};

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;

		type BridgedChainId: Get<ChainId>;
		type OutboundPayload: Parameter + Size;
		type OutboundMessageFee: From<u64>;

		type CallToPayload: CallToPayload<Self::OutboundPayload>;
		type BridgedAssetReceiverT: BridgedAssetReceiver<RelayAccount<AccountId<Self>>>;
		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;
		type ToEthAddressT: ToEthAddress<Self::AccountId>;
		type MessageRelayCallT: MessageRelayCall<Self::OutboundPayload, Self::Call>;
	}

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// new message relayed
		NewMessageRelayed(T::AccountId, u8),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// The proof is not in backing list
		InvalidProof,
		/// Invalid Backing address
		InvalidBackingAddr,
		/// Encode Invalid
		EncodeInvalid,
		/// Dispatch Message Relay Failed
		DispatchFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn remote_root_id)]
	pub type RemoteRootId<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: PhantomData<(T, I)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self {
				phantom: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			let chain_id = T::BridgedChainId::get();
			let hex_id = derive_account_id::<T::AccountId>(chain_id, SourceAccount::Root);
			let target_id = T::BridgedAccountIdConverter::convert(hex_id);
			<RemoteRootId<T, I>>::put(target_id);
		}
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}
	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {}
}

impl<T, I> Relay for Pallet<T, I>
where
	T: Config<I>,
	I: 'static,
	<T::Call as Dispatchable>::Origin: From<RawOrigin<T::AccountId>>,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo>,
{
	type RelayProof = AccountId<T>;
	type RelayMessage = (u32, Token, RelayAccount<AccountId<T>>);
	type VerifiedResult = Result<EthereumAddress, DispatchError>;
	type RelayMessageResult = DispatchResult;

	fn verify(proof: &Self::RelayProof) -> Self::VerifiedResult {
		let source_root = <RemoteRootId<T, I>>::get();
		ensure!(&source_root == proof, <Error<T, I>>::InvalidProof);
		Ok(T::ToEthAddressT::into_ethereum_id(proof))
	}

	fn relay_message(message: &Self::RelayMessage) -> Self::RelayMessageResult {
		let (spec_version, token, relay_account) = message.clone();
		let encoded = T::BridgedAssetReceiverT::encode_call(token, relay_account)
			.map_err(|_| <Error<T, I>>::EncodeInvalid)?;
		let relay_id: AccountId<T> = T::PalletId::get().into_account();
		let payload = T::CallToPayload::to_payload(spec_version, encoded);

		T::MessageRelayCallT::encode_call(payload)
			.dispatch(RawOrigin::Signed(relay_id).into())
			.map_err(|_| <Error<T, I>>::DispatchFailed)?;
		Ok(())
	}

	fn digest() -> RelayDigest {
		let mut digest: RelayDigest = Default::default();
		let pallet_digest = sha3::Keccak256::digest(T::PalletId::get().encode().as_slice());
		digest.copy_from_slice(&pallet_digest[..4]);
		digest
	}
}

pub trait ToEthAddress<A> {
	fn into_ethereum_id(address: &A) -> EthereumAddress;
}

pub struct TruncateToEthAddress;
impl ToEthAddress<AccountId32> for TruncateToEthAddress {
	fn into_ethereum_id(address: &AccountId32) -> EthereumAddress {
		let account20: &[u8] = &address.as_ref();
		EthereumAddress::from_slice(&account20[..20])
	}
}

pub trait MessageRelayCall<P, C> {
	fn encode_call(payload: P) -> C;
}
