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

//! Mock file for ethereum-backing.

#[macro_export]
macro_rules! decl_tests {
	($($pallet:tt)*) => {
		// --- substrate ---
		use frame_election_provider_support::onchain;
		use frame_support::{weights::Weight, traits::{Currency, GenesisBuild}, PalletId};
		use frame_system::mocking::*;
		use sp_core::crypto::key_types;
		use sp_runtime::{
			testing::{Header, TestXt, UintAuthorityId},
			traits::{IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
		 	{KeyTypeId, MultiSignature, Perbill},
			DispatchResult
		};
		// --- darwinia ---
		use crate::{pallet::*, *, self as darwinia_ethereum_backing};
		use darwinia_relay_primitives::*;
		use darwinia_staking::{EraIndex, Exposure, ExposureOf};
		use ethereum_primitives::EthereumAddress;

		type Block = MockBlock<Test>;
		type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
		type Extrinsic = TestXt<Call, ()>;

		/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
		type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
		/// Some way of identifying an account on the chain. We intentionally make it equivalent
		/// to the public key of our transaction signing scheme.
		type Signature = MultiSignature;
		type Balance = u128;
		type BlockNumber = u64;

		darwinia_support::impl_test_account_data! {}

		impl frame_system::Config for Test {
			type BaseCallFilter = ();
			type BlockWeights = ();
			type BlockLength = ();
			type DbWeight = ();
			type Origin = Origin;
			type Call = Call;
			type Index = u64;
			type BlockNumber = BlockNumber;
			type Hash = sp_core::H256;
			type Hashing = ::sp_runtime::traits::BlakeTwo256;
			type AccountId = AccountId;
			type Lookup = IdentityLookup<Self::AccountId>;
			type Header = Header;
			type Event = ();
			type BlockHashCount = ();
			type Version = ();
			type PalletInfo = PalletInfo;
			type AccountData = AccountData<Balance>;
			type OnNewAccount = ();
			type OnKilledAccount = ();
			type SystemWeightInfo = ();
			type SS58Prefix = ();
			type OnSetCode = ();
		}

		impl pallet_timestamp::Config for Test {
			type Moment = u64;
			type OnTimestampSet = ();
			type MinimumPeriod = ();
			type WeightInfo = ();
		}

		pub struct TestSessionHandler;
		impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
			const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

			fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

			fn on_new_session<Ks: OpaqueKeys>(
				_changed: bool,
				_validators: &[(AccountId, Ks)],
				_queued_validators: &[(AccountId, Ks)],
			) {
			}

			fn on_disabled(_validator_index: usize) {}
		}
		frame_support::parameter_types! {
			pub const Period: BlockNumber = 1;
			pub const Offset: BlockNumber = 0;
		}
		impl pallet_session::Config for Test {
			type Event = ();
			type ValidatorId = AccountId;
			type ValidatorIdOf = ();
			type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
			type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
			type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
			type SessionHandler = TestSessionHandler;
			type Keys = UintAuthorityId;
			type DisabledValidatorsThreshold = ();
			type WeightInfo = ();
		}

		impl pallet_session::historical::Config for Test {
			type FullIdentification = Exposure<AccountId, Balance, Balance>;
			type FullIdentificationOf = ExposureOf<Test>;
		}

		impl darwinia_balances::Config<KtonInstance> for Test {
			type Balance = Balance;
			type DustRemoval = ();
			type Event = ();
			type ExistentialDeposit = ();
			type BalanceInfo = AccountData<Balance>;
			type AccountStore = System;
			type MaxLocks = ();
			type OtherCurrencies = ();
			type WeightInfo = ();
		}
		impl darwinia_balances::Config<RingInstance> for Test {
			type Balance = Balance;
			type DustRemoval = ();
			type Event = ();
			type ExistentialDeposit = ();
			type BalanceInfo = AccountData<Balance>;
			type AccountStore = System;
			type MaxLocks = ();
			type OtherCurrencies = ();
			type WeightInfo = ();
		}

		impl onchain::Config for Test {
			type AccountId = AccountId;
			type BlockNumber = BlockNumber;
			type BlockWeights = ();
			type Accuracy = Perbill;
			type DataProvider = Staking;
		}

		frame_support::parameter_types! {
			pub const StakingPalletId: PalletId = PalletId(*b"da/staki");
		}
		impl darwinia_staking::Config for Test {
			const MAX_NOMINATIONS: u32 = 0;
			type Event = ();
			type PalletId = StakingPalletId;
			type UnixTime = Timestamp;
			type SessionsPerEra = ();
			type BondingDurationInEra = ();
			type BondingDurationInBlockNumber = ();
			type SlashDeferDuration = ();
			type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
			type SessionInterface = Self;
			type NextNewSession = Session;
			type MaxNominatorRewardedPerValidator = ();
			type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
			type RingCurrency = Ring;
			type RingRewardRemainder = ();
			type RingSlash = ();
			type RingReward = ();
			type KtonCurrency = Kton;
			type KtonSlash = ();
			type KtonReward = ();
			type Cap = ();
			type TotalPower = ();
			type WeightInfo = ();
		}

		pub struct EcdsaAuthorities;
		impl RelayAuthorityProtocol<BlockNumber> for EcdsaAuthorities {
			type Signer = EthereumAddress;

			fn schedule_mmr_root(_: BlockNumber) -> DispatchResult {
				Ok(())
			}

			fn check_authorities_change_to_sync(_: Term, _: Vec<Self::Signer>) -> DispatchResult {
				Ok(())
			}

			fn sync_authorities_change() -> DispatchResult {
				Ok(())
			}
		}
		frame_support::parameter_types! {
			pub const EthereumBackingPalletId: PalletId = PalletId(*b"da/backi");
			pub const EthereumBackingFeePalletId: PalletId = PalletId(*b"da/ethfe");
			pub const RingLockLimit: Balance = 1000;
			pub const KtonLockLimit: Balance = 1000;
			pub const AdvancedFee: Balance = 1;
		}
		impl Config for Test {
			type PalletId = EthereumBackingPalletId;
			type FeePalletId = EthereumBackingFeePalletId;
			type Event = ();
			type RedeemAccountId = AccountId;
			type EthereumRelay = EthereumRelay;
			type OnDepositRedeem = Staking;
			type RingCurrency = Ring;
			type KtonCurrency = Kton;
			type RingLockLimit = RingLockLimit;
			type KtonLockLimit = KtonLockLimit;
			type AdvancedFee = AdvancedFee;
			type SyncReward = ();
			type EcdsaAuthorities = EcdsaAuthorities;
			type WeightInfo = ();
		}

		impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
		where
			Call: From<LocalCall>,
		{
			type Extrinsic = Extrinsic;
			type OverarchingCall = Call;
		}

		frame_support::construct_runtime! {
			pub enum Test
			where
				Block = Block,
				NodeBlock = Block,
				UncheckedExtrinsic = UncheckedExtrinsic
			{
				System: frame_system::{Pallet, Call, Storage, Config},
				Timestamp: pallet_timestamp::{Pallet, Call, Storage},
				Ring: darwinia_balances::<Instance1>::{Pallet, Call, Storage},
				Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage},
				Staking: darwinia_staking::{Pallet, Call, Storage},
				Session: pallet_session::{Pallet, Call, Storage},
				EthereumBacking: darwinia_ethereum_backing::{Pallet, Call, Storage, Config<T>},
				$($pallet)*,
			}
		}
	};
}
