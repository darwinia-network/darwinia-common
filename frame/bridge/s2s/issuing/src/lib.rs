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

//! Prototype module for s2s cross chain assets issuing.

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

// --- crates ---
use ethereum_types::{H160, H256, U256};
use sha3::Digest;
// --- substrate ---
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement::*, Get},
	PalletId,
};
use frame_system::ensure_signed;
use sp_runtime::{traits::Convert, DispatchError, SaturatedConversion};
use sp_std::{str, vec::Vec};
// --- darwinia ---
use bp_runtime::{ChainId, Size};
use darwinia_evm::AddressMapping;
use darwinia_support::{
	balance::*,
	evm::POW_9,
	s2s::{source_root_converted_id, RelayMessageCaller, ToEthAddress},
	PalletDigest,
};
use dp_asset::{
	token::{Token, TokenInfo},
	RecipientAccount,
};
use dp_contract::mapping_token_factory::{MappingTokenFactory as mtf, TokenBurnInfo};

const BURN_ACTION: &[u8] = b"burned(address,address,address,address,uint256)";

pub use pallet::*;
pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: dvm_ethereum::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type ReceiverAccountId: From<[u8; 32]> + Into<Self::AccountId> + Clone;
		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;
		type BridgedChainId: Get<ChainId>;
		type ToEthAddressT: ToEthAddress<Self::AccountId>;
		type OutboundPayload: Parameter + Size;
		type CallEncoder: EncodeCall<Self::AccountId, Self::OutboundPayload>;
		type FeeAccount: Get<Option<Self::AccountId>>;
		type MessageSender: RelayMessageCaller<Self::OutboundPayload, RingBalance<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Handle dispatch call from dispatch precompile contract
		///
		/// When user burn their tokens, this handler will receive the event from dispatch
		/// precompile contract, and relay this event to the target chain to unlock asset.
		// TODO: update the weight
		#[pallet::weight(0)]
		#[frame_support::transactional]
		pub fn dispatch_handle(origin: OriginFor<T>, input: Vec<u8>) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;

			// Ensure the input data is long enough
			ensure!(input.len() >= 8, <Error<T>>::InvalidInput);
			// Ensure that the user is mapping token factory contract
			let factory = MappingFactoryAddress::<T>::get();
			let factory_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(factory);
			ensure!(caller == factory_id, <Error<T>>::NotFactoryContract);

			let burn_action = &sha3::Keccak256::digest(&BURN_ACTION)[0..4];
			if &input[4..8] == burn_action {
				let burn_info =
					TokenBurnInfo::decode(&input[8..]).map_err(|_| Error::<T>::InvalidDecoding)?;
				// Ensure the recipient is valid
				ensure!(
					burn_info.recipient.len() == 32,
					<Error<T>>::InvalidAddressLen
				);

				let fee = Self::transform_dvm_balance(burn_info.fee);
				if let Some(fee_account) = T::FeeAccount::get() {
					// Since fee account will represent use to make a cross chain call, give fee to fee account here.
					// the fee transfer path
					// user -> mapping_token_factory(caller) -> fee_account -> fee_fund -> relayers
					<T as Config>::RingCurrency::transfer(&caller, &fee_account, fee, KeepAlive)?;
				}

				Self::burn_and_remote_unlock(fee, burn_info)?;
			} else {
				log::trace!("No action match this input selector");
			}
			Ok(().into())
		}

		/// Handle remote register relay message
		/// Before the token transfer, token should be created first
		#[pallet::weight(0)]
		pub fn remote_register(origin: OriginFor<T>, token: Token) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			let backing = Self::verify_origin(&user)?;
			let (token_type, token_info) = token
				.token_info()
				.map_err(|_| Error::<T>::InvalidTokenType)?;
			let mut mapped_address = Self::mapped_token_address(backing, token_info.address)?;
			ensure!(mapped_address == H160::zero(), "asset has been registered");

			match token_info.option {
				Some(option) => {
					let name =
						str::from_utf8(&option.name[..]).map_err(|_| Error::<T>::StringCF)?;
					let symbol =
						str::from_utf8(&option.symbol[..]).map_err(|_| Error::<T>::StringCF)?;
					let input = mtf::encode_create_erc20(
						Self::digest(),
						token_type,
						&name,
						&symbol,
						option.decimal,
						backing,
						token_info.address,
					)
					.map_err(|_| Error::<T>::InvalidEncodeERC20)?;

					Self::transact_mapping_factory(input)?;
					mapped_address = Self::mapped_token_address(backing, token_info.address)?;
					Self::deposit_event(Event::TokenRegistered(
						user,
						backing,
						token_info.address,
						mapped_address,
					));
				}
				_ => return Err(Error::<T>::InvalidTokenOption.into()),
			}
			Ok(().into())
		}

		/// Handle relay message sent from the source backing pallet with relay message
		#[pallet::weight(0)]
		pub fn remote_issue(
			origin: OriginFor<T>,
			token: Token,
			recipient: H160,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// the s2s message relay has been verified that the message comes from the backing chain with the
			// chainID and backing sender address.
			// here only we need is to check the sender is root
			let backing = Self::verify_origin(&user)?;

			let (_, token_info) = token
				.token_info()
				.map_err(|_| Error::<T>::InvalidTokenType)?;

			let mapped_address = Self::mapped_token_address(backing, token_info.address)?;

			ensure!(
				mapped_address != H160::zero(),
				"asset has not been registered"
			);
			// Redeem process
			if let Some(value) = token_info.value {
				let input = mtf::encode_cross_receive(mapped_address, recipient, value)
					.map_err(|_| Error::<T>::InvalidMintEncoding)?;
				Self::transact_mapping_factory(input)?;
				Self::deposit_event(Event::TokenIssued(
					backing,
					mapped_address,
					recipient,
					value,
				));
			}
			Ok(().into())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Create new token
		/// [user, backing, source_address, mapping_address]
		TokenRegistered(AccountId<T>, H160, H160, H160),
		/// Redeem Token
		/// [backing, mapping_address, recipient, amount]
		TokenIssued(H160, H160, H160, U256),
		/// Token Burned and request Remote unlock
		/// [spec_version, weight, tokentype, source, amount, recipient, fee]
		TokenBurned(u32, u64, u32, H160, U256, AccountId<T>, U256),
	}

	#[pallet::error]
	/// Issuing pallet errors.
	pub enum Error<T> {
		/// The input data is not long enough
		InvalidInput,
		/// The address is not from mapping factory contract address
		NotFactoryContract,
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
		/// StringCF
		StringCF,
		/// encode erc20 tx failed
		InvalidEncodeERC20,
		/// encode mint tx failed
		InvalidMintEncoding,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// invalid token type
		InvalidTokenType,
		/// invalid token option
		InvalidTokenOption,
		/// decode event failed
		InvalidDecoding,
		/// invalid source origin
		InvalidOrigin,
		/// encode dispatch call failed
		EncodeInvalid,
		/// send relay message failed
		SendMessageFailed,
		/// call mapping factory failed
		MappingFactoryCallFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn mapping_factory_address)]
	pub type MappingFactoryAddress<T: Config> = StorageValue<_, H160, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub mapping_factory_address: H160,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				mapping_factory_address: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<MappingFactoryAddress<T>>::put(&self.mapping_factory_address);
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn digest() -> PalletDigest {
		let mut digest: PalletDigest = Default::default();
		let pallet_digest = sha3::Keccak256::digest(T::PalletId::get().encode().as_slice());
		digest.copy_from_slice(&pallet_digest[..4]);
		digest
	}

	pub fn mapped_token_address(backing: H160, source: H160) -> Result<H160, DispatchError> {
		let factory_address = <MappingFactoryAddress<T>>::get();
		let bytes = mtf::encode_mapping_token(backing, source)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapped_address = dvm_ethereum::Pallet::<T>::do_call(factory_address, bytes)
			.map_err(|e| -> &'static str { e.into() })?;
		if mapped_address.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapped_address.as_slice()[12..]))
	}

	/// Make a call to mapping factory contract
	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResultWithPostInfo {
		let contract = MappingFactoryAddress::<T>::get();
		dvm_ethereum::Pallet::<T>::do_call(contract, input)
			.map_err(|_| Error::<T>::MappingFactoryCallFailed)?;
		Ok(().into())
	}

	pub fn transform_dvm_balance(value: U256) -> RingBalance<T> {
		(value / POW_9).low_u128().saturated_into()
	}

	/// Burn and send message to bridged chain
	pub fn burn_and_remote_unlock(
		fee: RingBalance<T>,
		burn_info: TokenBurnInfo,
	) -> Result<(), DispatchError> {
		let (spec_version, weight, token_type, address, amount) = (
			burn_info.spec_version,
			burn_info.weight,
			burn_info.token_type,
			burn_info.source,
			burn_info.amount,
		);
		let account_id: T::ReceiverAccountId =
			array_bytes::dyn_into!(burn_info.recipient.as_slice(), 32);
		let token: Token = (token_type, TokenInfo::new(address, Some(amount), None)).into();
		let account = RecipientAccount::DarwiniaAccount(account_id.clone().into());

		let payload = T::CallEncoder::encode_remote_unlock(spec_version, weight, token, account)
			.map_err(|_| Error::<T>::EncodeInvalid)?;
		T::MessageSender::send_message(payload, fee).map_err(|_| Error::<T>::SendMessageFailed)?;
		Self::deposit_event(Event::TokenBurned(
			spec_version,
			weight,
			token_type,
			address,
			amount,
			account_id.into(),
			burn_info.fee,
		));
		Ok(())
	}

	fn verify_origin(account: &T::AccountId) -> Result<H160, DispatchError> {
		let source_root = source_root_converted_id::<T::AccountId, T::BridgedAccountIdConverter>(
			T::BridgedChainId::get(),
		);
		ensure!(account == &source_root, Error::<T>::InvalidOrigin);
		Ok(T::ToEthAddressT::into_ethereum_id(account))
	}
}

pub trait EncodeCall<AccountId, Payload> {
	fn encode_remote_unlock(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<Payload, ()>;
}
