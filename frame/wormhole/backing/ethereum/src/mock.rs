// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_election_provider_support::onchain;
use frame_support::{
	traits::{Everything, GenesisBuild, SortedMembers},
	PalletId,
};
use frame_system::{mocking::*, EnsureRoot};
use sp_core::crypto::key_types;
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
	DispatchError, DispatchResult, KeyTypeId, MultiSignature, Perbill, RuntimeDebug,
};
// --- darwinia-network ---
use crate::{self as to_ethereum_backing, pallet::*};
use darwinia_bridge_ethereum::{EthereumRelayHeaderParcel, EthereumRelayProofs, MMRProof};
use darwinia_relay_primitives::*;
use darwinia_staking::{Exposure, ExposureOf};
use ethereum_primitives::{
	header::EthereumHeader, receipt::EthereumReceiptProof, EthereumAddress, EthereumBlockNumber,
	EthereumNetwork,
};

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

pub type EthereumRelayError = darwinia_bridge_ethereum::Error<Test>;

darwinia_support::impl_test_account_data! {}

impl frame_system::Config for Test {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = ();
	type Hash = sp_core::H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

impl pallet_timestamp::Config for Test {
	type MinimumPeriod = ();
	type Moment = u64;
	type OnTimestampSet = ();
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
	type DisabledValidatorsThreshold = ();
	type Event = ();
	type Keys = UintAuthorityId;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = TestSessionHandler;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ();
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = Exposure<AccountId, Balance, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

impl darwinia_balances::Config<KtonInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl darwinia_balances::Config<RingInstance> for Test {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type OtherCurrencies = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl onchain::Config for Test {
	type Accuracy = Perbill;
	type DataProvider = Staking;
}

frame_support::parameter_types! {
	pub const StakingPalletId: PalletId = PalletId(*b"da/staki");
}
impl darwinia_staking::Config for Test {
	type BondingDurationInBlockNumber = ();
	type BondingDurationInEra = ();
	type Cap = ();
	type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
	type Event = ();
	type GenesisElectionProvider = Self::ElectionProvider;
	type KtonCurrency = Kton;
	type KtonReward = ();
	type KtonSlash = ();
	type MaxNominatorRewardedPerValidator = ();
	type NextNewSession = Session;
	type PalletId = StakingPalletId;
	type RingCurrency = Ring;
	type RingReward = ();
	type RingRewardRemainder = ();
	type RingSlash = ();
	type SessionInterface = Self;
	type SessionsPerEra = ();
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SlashDeferDuration = ();
	type SortedListProvider = darwinia_staking::UseNominatorsMap<Self>;
	type TotalPower = ();
	type UnixTime = Timestamp;
	type WeightInfo = ();

	const MAX_NOMINATIONS: u32 = 0;
}

pub struct UnusedRelayerGame;
impl RelayerGameProtocol for UnusedRelayerGame {
	type RelayHeaderId = EthereumBlockNumber;
	type RelayHeaderParcel = EthereumRelayHeaderParcel;
	type RelayProofs = EthereumRelayProofs;
	type Relayer = AccountId;

	fn get_affirmed_relay_header_parcels(
		_: &RelayAffirmationId<Self::RelayHeaderId>,
	) -> Option<Vec<Self::RelayHeaderParcel>> {
		unimplemented!()
	}

	fn best_confirmed_header_id_of(_: &Self::RelayHeaderId) -> Self::RelayHeaderId {
		unimplemented!()
	}

	fn affirm(
		_: &Self::Relayer,
		_: Self::RelayHeaderParcel,
		_: Option<Self::RelayProofs>,
	) -> Result<Self::RelayHeaderId, DispatchError> {
		unimplemented!()
	}

	fn dispute_and_affirm(
		_: &Self::Relayer,
		_: Self::RelayHeaderParcel,
		_: Option<Self::RelayProofs>,
	) -> Result<(Self::RelayHeaderId, u32), DispatchError> {
		unimplemented!()
	}

	fn complete_relay_proofs(
		_: RelayAffirmationId<Self::RelayHeaderId>,
		_: Vec<Self::RelayProofs>,
	) -> DispatchResult {
		unimplemented!()
	}

	fn extend_affirmation(
		_: &Self::Relayer,
		_: RelayAffirmationId<Self::RelayHeaderId>,
		_: Vec<Self::RelayHeaderParcel>,
		_: Option<Vec<Self::RelayProofs>>,
	) -> Result<(Self::RelayHeaderId, u32, u32), DispatchError> {
		unimplemented!()
	}
}

pub struct UnusedTechnicalMembership;
impl SortedMembers<AccountId> for UnusedTechnicalMembership {
	fn sorted_members() -> Vec<AccountId> {
		unimplemented!()
	}
}
frame_support::parameter_types! {
	pub const EthereumRelayPalletId: PalletId = PalletId(*b"da/ethrl");
	pub static EthereumRelayBridgeNetwork: EthereumNetwork = EthereumNetwork::Ropsten;
}
impl darwinia_bridge_ethereum::Config for Test {
	type ApproveOrigin = EnsureRoot<AccountId>;
	type ApproveThreshold = ();
	type BridgedNetwork = EthereumRelayBridgeNetwork;
	type Call = Call;
	type ConfirmPeriod = ();
	type Currency = Ring;
	type Event = ();
	type PalletId = EthereumRelayPalletId;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RejectThreshold = ();
	type RelayerGame = UnusedRelayerGame;
	type TechnicalMembership = UnusedTechnicalMembership;
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
	type AdvancedFee = AdvancedFee;
	type EcdsaAuthorities = EcdsaAuthorities;
	type EthereumRelay = EthereumRelay;
	type Event = ();
	type FeePalletId = EthereumBackingFeePalletId;
	type KtonCurrency = Kton;
	type KtonLockLimit = KtonLockLimit;
	type OnDepositRedeem = Staking;
	type PalletId = EthereumBackingPalletId;
	type RedeemAccountId = AccountId;
	type RingCurrency = Ring;
	type RingLockLimit = RingLockLimit;
	type SyncReward = ();
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
		EthereumBacking: to_ethereum_backing::{Pallet, Call, Storage, Config<T>},
		EthereumRelay: darwinia_bridge_ethereum::{Pallet, Call, Storage},
	}
}

pub struct ExtBuilder {
	pub network: EthereumNetwork,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self { network: EthereumNetwork::Ropsten }
	}
}
impl ExtBuilder {
	pub fn mainnet(mut self) -> Self {
		self.network = EthereumNetwork::Mainnet;

		self
	}

	pub fn set_associated_constants(&self) {
		ETHEREUM_RELAY_BRIDGE_NETWORK.with(|v| v.replace(self.network.clone()));
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_constants();

		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if self.network == EthereumNetwork::Ropsten {
			to_ethereum_backing::GenesisConfig::<Test> {
				token_redeem_address: array_bytes::hex_into_unchecked(
					"0x49262B932E439271d05634c32978294C7Ea15d0C",
				),
				deposit_redeem_address: array_bytes::hex_into_unchecked(
					"0x6EF538314829EfA8386Fc43386cB13B4e0A67D1e",
				),
				set_authorities_address: array_bytes::hex_into_unchecked(
					"0xE4A2892599Ad9527D76Ce6E26F93620FA7396D85",
				),
				ring_token_address: array_bytes::hex_into_unchecked(
					"0xb52FBE2B925ab79a821b261C82c5Ba0814AAA5e0",
				),
				kton_token_address: array_bytes::hex_into_unchecked(
					"0x1994100c58753793D52c6f457f189aa3ce9cEe94",
				),
				backed_ring: 20000000000000,
				backed_kton: 5000000000000,
			}
			.assimilate_storage(&mut t)
			.unwrap();
		} else {
			to_ethereum_backing::GenesisConfig::<Test> {
				token_redeem_address: array_bytes::hex_into_unchecked(
					"0xea7938985898af7fd945b03b7bc2e405e744e913",
				),
				deposit_redeem_address: array_bytes::hex_into_unchecked(
					"0x649fdf6ee483a96e020b889571e93700fbd82d88",
				),
				set_authorities_address: array_bytes::hex_into_unchecked(
					"0xE4A2892599Ad9527D76Ce6E26F93620FA7396D85",
				),
				ring_token_address: array_bytes::hex_into_unchecked(
					"0x9469d013805bffb7d3debe5e7839237e535ec483",
				),
				kton_token_address: array_bytes::hex_into_unchecked(
					"0x9f284e1337a815fe77d2ff4ae46544645b20c5ff",
				),
				backed_ring: 20000000000000,
				backed_kton: 5000000000000,
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		t.into()
	}
}

#[cfg_attr(test, derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct TestReceiptProofThing {
	pub header: EthereumHeader,
	pub receipt_proof: EthereumReceiptProof,
	pub mmr_proof: MMRProof,
}
