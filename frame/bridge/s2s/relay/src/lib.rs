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

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
	}

	use frame_support::{traits::Get, PalletId};
	pub use types::*;

	use sp_runtime::{AccountId32, DispatchError};

	use darwinia_relay_primitives::{Relay, RelayAccount, RelayDigest};
	use darwinia_support::traits::CallToPayload;

	use darwinia_asset_primitives::{token::Token, RemoteAssetReceiver};
	use ethereum_primitives::EthereumAddress;
	use frame_system::RawOrigin;
	use sp_runtime::traits::{AccountIdConversion, Convert};

	use bp_runtime::{derive_account_id, ChainId, Size, SourceAccount};
	use frame_support::{
		dispatch::{Dispatchable, PostDispatchInfo},
		pallet_prelude::*,
		Parameter,
	};

	use crate::weights::WeightInfo;
	use frame_system::pallet_prelude::*;
	use sha3::Digest;

	pub trait ToEthereumAddress<A> {
		fn into_ethereum_id(address: &A) -> EthereumAddress;
	}

	pub struct ConcatToEthereumAddress;
	impl ToEthereumAddress<AccountId32> for ConcatToEthereumAddress {
		fn into_ethereum_id(address: &AccountId32) -> EthereumAddress {
			let account20: &[u8] = &address.as_ref();
			EthereumAddress::from_slice(&account20[..20])
		}
	}

	pub trait MessageRelayCall<P, C> {
		fn encode_call(payload: P) -> C;
	}

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The ethereum-relay's module id, used for deriving its sovereign account ID.
		type PalletId: Get<PalletId>;

		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type RemoteAssetReceiverT: RemoteAssetReceiver<RelayAccount<AccountId<Self>>>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		type OutboundPayload: Parameter + Size;

		type OutboundMessageFee: From<u64>;

		type CallToPayload: CallToPayload<Self::OutboundPayload>;

		type RemoteAccountIdConverter: Convert<sp_core::hash::H256, Self::AccountId>;

		type ToEthereumAddressT: ToEthereumAddress<Self::AccountId>;

		type RemoteChainId: Get<ChainId>;

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
		EncodeInv,
		/// Dispatch Message Relay Failed
		DispatchFD,
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
			let chain_id = T::RemoteChainId::get();
			let hex_id = derive_account_id::<T::AccountId>(chain_id, SourceAccount::Root);
			let target_id = T::RemoteAccountIdConverter::convert(hex_id);
			<RemoteRootId<T, I>>::put(target_id);
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {}

	impl<T: Config<I>, I: 'static> Relay for Pallet<T, I>
	where
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
			Ok(T::ToEthereumAddressT::into_ethereum_id(proof))
		}

		fn relay_message(message: &Self::RelayMessage) -> Self::RelayMessageResult {
			let msg = message.clone();
			let encoded = T::RemoteAssetReceiverT::encode_call(msg.1, msg.2)
				.map_err(|_| <Error<T, I>>::EncodeInv)?;
			let relay_id: AccountId<T> = T::PalletId::get().into_account();
			let payload = T::CallToPayload::to_payload(msg.0, encoded);
			T::MessageRelayCallT::encode_call(payload)
				.dispatch(RawOrigin::Signed(relay_id).into())
				.map_err(|_| <Error<T, I>>::DispatchFD)?;
			Ok(())
		}

		fn digest() -> RelayDigest {
			let mut digest: RelayDigest = Default::default();
			let pallet_digest = sha3::Keccak256::digest(T::PalletId::get().encode().as_slice());
			digest.copy_from_slice(&pallet_digest[..4]);
			digest
		}
	}
}

pub use pallet::*;
