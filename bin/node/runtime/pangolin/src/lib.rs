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

//! The Darwinia Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

pub mod pallets;
pub use pallets::*;

pub mod bridges;
pub use bridges::*;

pub mod impls {
	//! Some configurable implementations as associated type for the substrate runtime.

	pub mod relay {
		// --- darwinia ---
		use crate::*;
		use darwinia_relay_primitives::relayer_game::*;
		use ethereum_primitives::EthereumBlockNumber;

		pub struct EthereumRelayerGameAdjustor;
		impl AdjustableRelayerGame for EthereumRelayerGameAdjustor {
			type Moment = BlockNumber;
			type Balance = Balance;
			type RelayHeaderId = EthereumBlockNumber;

			fn max_active_games() -> u8 {
				32
			}

			fn affirm_time(round: u32) -> Self::Moment {
				match round {
					// 1.5 mins
					0 => 15,
					// 0.5 mins
					_ => 5,
				}
			}

			fn complete_proofs_time(round: u32) -> Self::Moment {
				match round {
					// 1.5 mins
					0 => 15,
					// 0.5 mins
					_ => 5,
				}
			}

			fn update_sample_points(sample_points: &mut Vec<Vec<Self::RelayHeaderId>>) {
				sample_points.push(vec![sample_points.last().unwrap().last().unwrap() - 1]);
			}

			fn estimate_stake(round: u32, affirmations_count: u32) -> Self::Balance {
				match round {
					0 => match affirmations_count {
						0 => 1000 * COIN,
						_ => 1500 * COIN,
					},
					_ => 100 * COIN,
				}
			}
		}
	}

	// --- crates ---
	use smallvec::smallvec;
	// --- substrate ---
	use frame_support::{
		traits::{Currency, Imbalance, OnUnbalanced},
		weights::{WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	};
	// --- darwinia ---
	use crate::*;

	darwinia_support::impl_account_data! {
		struct AccountData<Balance>
		for
			RingInstance,
			KtonInstance
		where
			Balance = Balance
		{
			// other data
		}
	}

	pub struct ToAuthor;
	impl OnUnbalanced<RingNegativeImbalance> for ToAuthor {
		fn on_nonzero_unbalanced(amount: RingNegativeImbalance) {
			let numeric_amount = amount.peek();
			let author = Authorship::author();
			Ring::resolve_creating(&Authorship::author(), amount);
			System::deposit_event(<darwinia_balances::Event<Runtime, RingInstance>>::Deposit(
				author,
				numeric_amount,
			));
		}
	}

	pub struct DealWithFees;
	impl OnUnbalanced<RingNegativeImbalance> for DealWithFees {
		fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = RingNegativeImbalance>) {
			if let Some(fees) = fees_then_tips.next() {
				// for fees, 80% to treasury, 20% to author
				let mut split = fees.ration(80, 20);
				if let Some(tips) = fees_then_tips.next() {
					// for tips, if any, 100% to author
					tips.merge_into(&mut split.1);
				}
				Treasury::on_unbalanced(split.0);
				ToAuthor::on_unbalanced(split.1);
			}
		}
	}

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, MAXIMUM_BLOCK_WEIGHT]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// in Crab, extrinsic base weight (smallest non-zero weight) is mapped to 100 MILLI:
			let p = 100 * MILLI;
			let q = Balance::from(ExtrinsicBaseWeight::get());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}

pub mod wasm {
	//! Make the WASM binary available.

	#[cfg(all(
		feature = "std",
		any(target_arch = "x86_64", target_arch = "x86", target_vendor = "apple")
	))]
	include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

	#[cfg(all(
		feature = "std",
		not(any(target_arch = "x86_64", target_arch = "x86", target_vendor = "apple"))
	))]
	pub const WASM_BINARY: &[u8] = include_bytes!("../../../../wasm/pangolin_runtime.compact.wasm");
	#[cfg(all(
		feature = "std",
		not(any(target_arch = "x86_64", target_arch = "x86", target_vendor = "apple"))
	))]
	pub const WASM_BINARY_BLOATY: &[u8] = include_bytes!("../../../../wasm/pangolin_runtime.wasm");

	/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
	#[cfg(feature = "std")]
	pub fn wasm_binary_unwrap() -> &'static [u8] {
		#[cfg(all(
			feature = "std",
			any(target_arch = "x86_64", target_arch = "x86", target_vendor = "apple")
		))]
		return WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is \
			built with `SKIP_WASM_BUILD` flag and it is only usable for \
			production chains. Please rebuild with the flag disabled.",
		);
		#[cfg(all(
			feature = "std",
			not(any(target_arch = "x86_64", target_arch = "x86", target_vendor = "apple"))
		))]
		return WASM_BINARY;
	}
}
pub use wasm::*;

pub use pangolin_constants::*;

pub use darwinia_staking::StakerStatus;

pub use darwinia_balances::Call as BalancesCall;
pub use frame_system::Call as SystemCall;
pub use pallet_bridge_grandpa::Call as BridgeGrandpaCall;
pub use pallet_bridge_messages::Call as BridgeMessagesCall;
pub use pallet_sudo::Call as SudoCall;

// --- crates.io ---
use codec::{Decode, Encode};
// --- substrate ---
use bp_runtime::MILLAU_CHAIN_ID;
use bridge_runtime_common::messages::{
	source::estimate_message_dispatch_and_delivery_fee, MessageBridge,
};
use frame_support::{
	traits::{KeyOwnerProofSystem, OnRuntimeUpgrade},
	weights::{constants::ExtrinsicBaseWeight, Weight},
};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_transaction_payment::FeeDetails;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo as TransactionPaymentRuntimeDispatchInfo;
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::{AllowedSlots, BabeEpochConfiguration};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	generic,
	traits::{Block as BlockT, NumberFor, SaturatedConversion, StaticLookup},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiAddress, OpaqueExtrinsic, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia ---
use bridges::substrate::millau_messages::{ToMillauMessagePayload, WithMillauMessageBridge};
use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo as BalancesRuntimeDispatchInfo;
use darwinia_evm::{Account as EVMAccount, Runner};
use darwinia_header_mmr_rpc_runtime_api::RuntimeDispatchInfo as HeaderMMRRuntimeDispatchInfo;
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo as StakingRuntimeDispatchInfo;
use drml_primitives::*;
use dvm_rpc_runtime_api::TransactionStatus;
use impls::*;
use pangolin_bridge_primitives::PANGOLIN_CHAIN_ID;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
	CustomOnRuntimeUpgrade,
>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

type Ring = Balances;

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: sp_runtime::create_runtime_str!("Pangolin"),
	impl_name: sp_runtime::create_runtime_str!("Pangolin"),
	authoring_version: 1,
	// crate version ~2.5.0 := >=2.5.0, <2.6.0
	spec_version: 2501,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: BabeEpochConfiguration = BabeEpochConfiguration {
	c: PRIMARY_PROBABILITY,
	allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

frame_support::construct_runtime! {
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = OpaqueBlock,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Basic stuff; balances is uncallable initially.
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage} = 1,

		// Must be before session.
		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 2,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Balances: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 6,

		// Consensus support.
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 7,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 8,
		Staking: darwinia_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 9,
		Offences: pallet_offences::{Pallet, Call, Storage, Event} = 10,
		Historical: pallet_session_historical::{Pallet} = 11,
		Session: pallet_session::{Pallet, Call, Storage, Config<T>, Event} = 12,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 13,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned} = 14,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Call, Config} = 15,
		HeaderMMR: darwinia_header_mmr::{Pallet, Call, Storage} = 16,

		// Governance stuff; uncallable initially.
		Democracy: darwinia_democracy::{Pallet, Call, Storage, Config, Event<T>} = 17,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Config<T>, Event<T>} = 18,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Config<T>, Event<T>} = 19,
		PhragmenElection: darwinia_elections_phragmen::{Pallet, Call, Storage, Config<T>, Event<T>} = 20,
		TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 21,
		Treasury: darwinia_treasury::{Pallet, Call, Storage, Event<T>} = 22,

		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 23,

		// Claims. Usable initially.
		Claims: darwinia_claims::{Pallet, Call, Storage, Config, Event<T>, ValidateUnsigned} = 24,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: darwinia_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 25,

		// Utility module.
		Utility: pallet_utility::{Pallet, Call, Event} = 26,

		// Less simple identity module.
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 27,

		// Society module.
		Society: pallet_society::{Pallet, Call, Storage, Event<T>} = 28,

		// Social recovery module.
		Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 29,

		// System scheduler.
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 30,

		// Proxy module. Late addition.
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 31,

		// Multisig module. Late addition.
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 32,

		CrabIssuing: darwinia_crab_issuing::{Pallet, Call, Storage, Config} = 33,
		CrabBacking: darwinia_crab_backing::{Pallet, Storage, Config<T>} = 34,

		EthereumRelay: darwinia_ethereum_relay::{Pallet, Call, Storage, Config<T>, Event<T>} = 35,
		EthereumBacking: darwinia_ethereum_backing::{Pallet, Call, Storage, Config<T>, Event<T>} = 36,
		EthereumIssuing: darwinia_ethereum_issuing::{Pallet, Call, Storage, Config, Event<T>} = 42,
		EthereumRelayerGame: darwinia_relayer_game::<Instance1>::{Pallet, Storage} = 37,
		EthereumRelayAuthorities: darwinia_relay_authorities::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 38,

		TronBacking: darwinia_tron_backing::{Pallet, Config<T>} = 39,

		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>} = 40,
		Ethereum: dvm_ethereum::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 41,
		DynamicFee: dvm_dynamic_fee::{Pallet, Call, Storage, Inherent} = 47,

		BridgeMillauMessages: pallet_bridge_messages::<Instance1>::{Pallet, Call, Storage, Event<T>} = 43,
		BridgeMillauDispatch: pallet_bridge_dispatch::<Instance1>::{Pallet, Event<T>} = 44,
		BridgeMillauGrandpa: pallet_bridge_grandpa::<Instance1>::{Pallet, Call, Storage} = 45,
		Substrate2SubstrateIssuing: darwinia_s2s_issuing::{Pallet, Call, Storage, Config, Event<T>} = 49,

		BSC: darwinia_bridge_bsc::{Pallet, Call, Storage, Config} = 46,
	}
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as sp_runtime::traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		Call,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			darwinia_ethereum_relay::CheckEthereumRelayHeaderParcel::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
		Some((call, (address, signature, extra)))
	}
}
impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}
impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(
			data: sp_inherents::InherentData
		) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: BABE_GENESIS_EPOCH_CONFIG.c,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> TransactionPaymentRuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl darwinia_balances_rpc_runtime_api::BalancesApi<Block, AccountId, Balance> for Runtime {
		fn usable_balance(instance: u8, account: AccountId) -> BalancesRuntimeDispatchInfo<Balance> {
			match instance {
				0 => Ring::usable_balance_rpc(account),
				1 => Kton::usable_balance_rpc(account),
				_ => Default::default()
			}
		}
	}

	impl darwinia_header_mmr_rpc_runtime_api::HeaderMMRApi<Block, Hash> for Runtime {
		fn gen_proof(
			block_number_of_member_leaf: u64,
			block_number_of_last_leaf: u64
		) -> HeaderMMRRuntimeDispatchInfo<Hash> {
			HeaderMMR::gen_proof_rpc(block_number_of_member_leaf, block_number_of_last_leaf)
		}
	}

	impl darwinia_staking_rpc_runtime_api::StakingApi<Block, AccountId, Power> for Runtime {
		fn power_of(account: AccountId) -> StakingRuntimeDispatchInfo<Power> {
			Staking::power_of_rpc(account)
		}
	}

	impl dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as darwinia_evm::Config>::ChainId::get()
		}

		fn gas_price() -> U256 {
			<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price()
		}

		fn account_basic(address: H160) -> EVMAccount {
			use darwinia_evm::AccountBasic;

			<Runtime as darwinia_evm::Config>::RingAccountBasic::account_basic(&address)
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			darwinia_evm::Pallet::<Runtime>::account_codes(address)
		}

		fn author() -> H160 {
			<dvm_ethereum::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			darwinia_evm::Pallet::<Runtime>::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<darwinia_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as darwinia_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.low_u64(),
				gas_price,
				nonce,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Config>::config()),
			).map_err(|err| err.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<darwinia_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			<Runtime as darwinia_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.low_u64(),
				gas_price,
				nonce,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Config>::config()),
			).map_err(|err| err.into())
		}


		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<dvm_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<dvm_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<dvm_ethereum::Block>,
			Option<Vec<dvm_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}
	}

	impl millau_primitives::MillauFinalityApi<Block> for Runtime {
		fn best_finalized() -> (millau_primitives::BlockNumber, millau_primitives::Hash) {
			let header = BridgeMillauGrandpa::best_finalized();
			(header.number, header.hash())
		}

		fn is_known_header(hash: millau_primitives::Hash) -> bool {
			BridgeMillauGrandpa::is_known_header(hash)
		}
	}

	impl millau_primitives::ToMillauOutboundLaneApi<Block, Balance, ToMillauMessagePayload> for Runtime {
		fn estimate_message_delivery_and_dispatch_fee(
			_lane_id: bp_messages::LaneId,
			payload: ToMillauMessagePayload,
		) -> Option<Balance> {
			estimate_message_dispatch_and_delivery_fee::<WithMillauMessageBridge>(
				&payload,
				WithMillauMessageBridge::RELAYER_FEE_PERCENT,
			).ok()
		}

		fn messages_dispatch_weight(
			lane: bp_messages::LaneId,
			begin: bp_messages::MessageNonce,
			end: bp_messages::MessageNonce,
		) -> Vec<(bp_messages::MessageNonce, Weight, u32)> {
			(begin..=end).filter_map(|nonce| {
				let encoded_payload = BridgeMillauMessages::outbound_message_payload(lane, nonce)?;
				let decoded_payload = ToMillauMessagePayload::decode(
					&mut &encoded_payload[..]
				).ok()?;
				Some((nonce, decoded_payload.weight, encoded_payload.len() as _))
			})
			.collect()
		}

		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgeMillauMessages::outbound_latest_received_nonce(lane)
		}

		fn latest_generated_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgeMillauMessages::outbound_latest_generated_nonce(lane)
		}
	}

	impl millau_primitives::FromMillauInboundLaneApi<Block> for Runtime {
		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgeMillauMessages::inbound_latest_received_nonce(lane)
		}

		fn latest_confirmed_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgeMillauMessages::inbound_latest_confirmed_nonce(lane)
		}

		fn unrewarded_relayers_state(lane: bp_messages::LaneId) -> bp_messages::UnrewardedRelayersState {
			BridgeMillauMessages::inbound_unrewarded_relayers_state(lane)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> Result<
			(Weight, Weight),
			sp_runtime::RuntimeString
		> {
			let weight = Executive::try_runtime_upgrade()?;
			Ok((weight, RuntimeBlockWeights::get().max_block))
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, darwinia_s2s_issuing, Substrate2SubstrateIssuing);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

pub struct TransactionConverter;
impl dvm_rpc_runtime_api::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			<dvm_ethereum::Call<Runtime>>::transact(transaction).into(),
		)
	}
}
impl dvm_rpc_runtime_api::ConvertTransaction<OpaqueExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> OpaqueExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			<dvm_ethereum::Call<Runtime>>::transact(transaction).into(),
		);
		let encoded = extrinsic.encode();

		OpaqueExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		// <--- Hack for local test
		use frame_support::traits::Currency;
		let _ = Ring::deposit_creating(&BridgeMillauMessages::relayer_fund_account_id(), 1 << 50);
		// --->

		darwinia_header_mmr::migration::initialize_new_mmr_state::<Runtime>(b"HeaderMMR", 2);
		darwinia_relay_authorities::migration::migrate::<Runtime, EthereumRelayAuthoritiesInstance>(
			b"Instance1DarwiniaRelayAuthorities",
		);

		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		// <--- Hack for local test
		// use frame_support::traits::Currency;
		// let _ = Ring::deposit_creating(&BridgeMillauMessages::relayer_fund_account_id(), 1 << 50);
		// --->

		darwinia_header_mmr::migration::initialize_new_mmr_state::<Runtime>(b"HeaderMMR", 50);
		darwinia_relay_authorities::migration::migrate::<Runtime, EthereumRelayAuthoritiesInstance>(
			b"Instance1DarwiniaRelayAuthorities",
		);

		RuntimeBlockWeights::get().max_block
	}
}

pub fn pangolin_to_millau_account_ownership_digest<Call, AccountId, SpecVersion>(
	millau_call: &Call,
	pangolin_account_id: AccountId,
	millau_spec_version: SpecVersion,
) -> Vec<u8>
where
	Call: Encode,
	AccountId: Encode,
	SpecVersion: Encode,
{
	pallet_bridge_dispatch::account_ownership_digest(
		millau_call,
		pangolin_account_id,
		millau_spec_version,
		PANGOLIN_CHAIN_ID,
		MILLAU_CHAIN_ID,
	)
}
