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

//! The Darwinia Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::inconsistent_digit_grouping)]
#![recursion_limit = "256"]

pub mod pallets;
pub use pallets::*;

pub mod weights;

pub mod bridges_message;
pub use bridges_message::*;

pub mod wasm {
	//! Make the WASM binary available.

	include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

	/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
	#[cfg(feature = "std")]
	pub fn wasm_binary_unwrap() -> &'static [u8] {
		WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is \
			built with `SKIP_WASM_BUILD` flag and it is only usable for \
			production chains. Please rebuild with the flag disabled.",
		)
	}
}
pub use wasm::*;

mod migrations;
use migrations::*;

pub use darwinia_staking::{Forcing, StakerStatus};

// --- crates.io ---
use codec::Encode;
// --- paritytech ---
use fp_evm::FeeCalculator;
use frame_support::{log, traits::KeyOwnerProofSystem, weights::GetDispatchInfo};
use frame_system::{
	offchain::{AppCrypto, CreateSignedTransaction, SendTransactionTypes, SigningTypes},
	ChainContext, CheckEra, CheckGenesis, CheckNonZeroSender, CheckNonce, CheckSpecVersion,
	CheckTxVersion, CheckWeight, EnsureRoot,
};
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::{AllowedSlots, BabeEpochConfiguration};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	generic,
	traits::{
		Block as BlockT, Dispatchable, Extrinsic, NumberFor, PostDispatchInfoOf,
		SaturatedConversion, StaticLookup, Verify,
	},
	ApplyExtrinsicResult,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia-network ---
use drml_common_runtime::*;
use drml_primitives::*;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	CheckNonZeroSender<Runtime>,
	CheckSpecVersion<Runtime>,
	CheckTxVersion<Runtime>,
	CheckGenesis<Runtime>,
	CheckEra<Runtime>,
	CheckNonce<Runtime>,
	CheckWeight<Runtime>,
	ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	CustomOnRuntimeUpgrade,
>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

pub type RootOrigin = EnsureRoot<AccountId>;

type Ring = Balances;

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: sp_runtime::create_runtime_str!("Pangolin"),
	impl_name: sp_runtime::create_runtime_str!("Pangolin"),
	authoring_version: 0,
	spec_version: 2_10_01_0,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 0,
	state_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: BabeEpochConfiguration = BabeEpochConfiguration {
	c: PRIMARY_PROBABILITY,
	allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

frame_support::construct_runtime! {
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = OpaqueBlock,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 0,

		// Must be before session.
		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 2,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Balances: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 6,

		// Consensus things.
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 7,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 8,
		Staking: darwinia_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 9,
		Offences: pallet_offences::{Pallet, Storage, Event} = 10,
		Historical: pallet_session_historical::{Pallet} = 11,
		Session: pallet_session::{Pallet, Call, Storage, Config<T>, Event} = 12,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 13,
		Beefy: pallet_beefy::{Pallet, Storage, Config<T>} = 55,
		MessageGadget: darwinia_message_gadget::{Pallet, Call, Storage, Config} = 58,
		EcdsaAuthority: darwinia_ecdsa_authority::{Pallet, Call, Storage, Config, Event<T>} = 66,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned} = 14,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config} = 15,
		HeaderMmr: darwinia_header_mmr::{Pallet, Storage} = 16,

		// Governance things.
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 17,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Config<T>, Event<T>} = 18,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Config<T>, Event<T>} = 19,
		PhragmenElection: pallet_elections_phragmen::{Pallet, Call, Storage, Config<T>, Event<T>} = 20,
		TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 21,
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 22,
		KtonTreasury: pallet_treasury::<Instance2>::{Pallet, Call, Storage, Config, Event<T>} = 50,
		Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 51,
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 52,

		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 23,

		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 25,

		Utility: pallet_utility::{Pallet, Call, Event} = 26,

		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 27,

		Society: pallet_society::{Pallet, Call, Storage, Event<T>} = 28,

		Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 29,

		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 30,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 67,

		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 31,

		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 32,

		TronBacking: to_tron_backing::{Pallet, Config<T>} = 39,

		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>} = 40,
		Ethereum: darwinia_ethereum::{Pallet, Call, Storage, Config, Event<T>, Origin} = 41,
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 59,

		BridgePangoroDispatch: pallet_bridge_dispatch::<Instance1>::{Pallet, Event<T>} = 44,
		BridgePangoroGrandpa: pallet_bridge_grandpa::<Instance1>::{Pallet, Call, Storage} = 45,
		BridgePangoroMessages: pallet_bridge_messages::<Instance1>::{Pallet, Call, Storage, Event<T>} = 43,

		BridgeRococoGrandpa: pallet_bridge_grandpa::<Instance2>::{Pallet, Call, Storage} = 60,
		BridgeRococoParachains: pallet_bridge_parachains::<Instance1>::{Pallet, Call, Storage} = 61,

		BridgePangolinParachainDispatch: pallet_bridge_dispatch::<Instance2>::{Pallet, Event<T>} = 62,
		BridgePangolinParachainMessages: pallet_bridge_messages::<Instance2>::{Pallet, Call, Storage, Event<T>} = 63,

		PangoroFeeMarket: pallet_fee_market::<Instance1>::{Pallet, Call, Storage, Event<T>} = 53,
		PangolinParachainFeeMarket: pallet_fee_market::<Instance2>::{Pallet, Call, Storage, Event<T>} = 64,
		TransactionPause: module_transaction_pause::{Pallet, Call, Storage, Event<T>} = 54,

		// pangolin <> pangolin parachain alpha bridge
		BridgeMoonbaseRelayGrandpa: pallet_bridge_grandpa::<Instance3>::{Pallet, Call, Storage} = 68,
		BridgeMoonbaseRelayParachains: pallet_bridge_parachains::<Instance2>::{Pallet, Call, Storage} = 69,
		BridgePangolinParachainAlphaDispatch: pallet_bridge_dispatch::<Instance3>::{Pallet, Event<T>} = 70,
		BridgePangolinParachainAlphaMessages: pallet_bridge_messages::<Instance3>::{Pallet, Call, Storage, Event<T>} = 71,
		PangolinParachainAlphaFeeMarket: pallet_fee_market::<Instance3>::{Pallet, Call, Storage, Event<T>} = 72,

		ToPangolinParachainBacking: to_parachain_backing::{Pallet, Call, Storage, Config<T>, Event<T>} = 65,
	}
}

impl<LocalCall> CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		// take the biggest period possible.
		let period = BlockHashCountForPangolin::get()
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
			CheckNonZeroSender::<Runtime>::new(),
			CheckSpecVersion::<Runtime>::new(),
			CheckTxVersion::<Runtime>::new(),
			CheckGenesis::<Runtime>::new(),
			CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			CheckNonce::<Runtime>::from(nonce),
			CheckWeight::<Runtime>::new(),
			ChargeTransactionPayment::<Runtime>::from(tip),
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
impl SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}
impl<C> SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

drml_common_runtime::impl_self_contained_call!();

sp_api::impl_runtime_apis! {
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
			OpaqueMetadata::new(Runtime::metadata().into())
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
				genesis_authorities: Babe::authorities().to_vec(),
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

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
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

	// impl beefy_primitives::BeefyApi<Block> for Runtime {
	// 	fn validator_set() -> beefy_primitives::ValidatorSet<BeefyId> {
	// 		Beefy::validator_set()
	// 	}
	// }

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: sp_runtime::transaction_validity::TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> sp_runtime::transaction_validity::TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32)
			-> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance>
		{
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32)
			-> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance>
		{
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as darwinia_evm::Config>::ChainId::get()
		}

		fn gas_price() -> U256 {
			<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price()
		}

		fn account_basic(address: H160) -> darwinia_evm::Account {
			EVM::account_basic(&address)
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			darwinia_evm::Pallet::<Runtime>::account_codes(address)
		}

		fn author() -> H160 {
			<darwinia_evm::Pallet<Runtime>>::find_author()
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
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<darwinia_evm::CallInfo, sp_runtime::DispatchError> {
			// --- darwinia-network ---
			use darwinia_evm::Runner;

			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			<Runtime as darwinia_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Config>::config()),
			).map_err(|err| err.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<darwinia_evm::CreateInfo, sp_runtime::DispatchError> {
			// --- darwinia-network ---
			use darwinia_evm::Runner;

			let config = if estimate {
				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			<Runtime as darwinia_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Config>::config()),
			).map_err(|err| err.into())
		}


		fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<darwinia_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<darwinia_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<darwinia_ethereum::Block>,
			Option<Vec<darwinia_ethereum::Receipt>>,
			Option<Vec<fp_rpc::TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<darwinia_ethereum::Transaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				Call::Ethereum(darwinia_ethereum::Call::transact { transaction }) => Some(transaction),
				_ => None
			}).collect()
		}

		fn elasticity() -> Option<Permill> {
			Some(BaseFee::elasticity())
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: darwinia_ethereum::Transaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				darwinia_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	impl moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block> for Runtime {
		fn trace_transaction(
			_extrinsics: Vec<<Block as BlockT>::Extrinsic>,
			_traced_transaction: &darwinia_ethereum::Transaction,
		) -> Result<
			(),
			sp_runtime::DispatchError,
		> {
			#[cfg(feature = "evm-tracing")]
			{
				use dp_evm_tracer::tracer::EvmTracer;
				use darwinia_ethereum::Call::transact;
				// Apply the a subset of extrinsics: all the substrate-specific or ethereum
				// transactions that preceded the requested transaction.
				for ext in _extrinsics.into_iter() {
					let _ = match &ext.0.function {
						Call::Ethereum(transact { transaction }) => {
							if transaction == _traced_transaction {
								EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
								return Ok(());
							} else {
								Executive::apply_extrinsic(ext)
							}
						}
						_ => Executive::apply_extrinsic(ext),
					};
				}

				Err(sp_runtime::DispatchError::Other(
					"Failed to find Ethereum transaction among the extrinsics.",
				))
			}
			#[cfg(not(feature = "evm-tracing"))]
			Err(sp_runtime::DispatchError::Other(
				"Missing `evm-tracing` compile time feature flag.",
			))
		}
		fn trace_block(
			_extrinsics: Vec<<Block as BlockT>::Extrinsic>,
			_known_transactions: Vec<H256>,
		) -> Result<
			(),
			sp_runtime::DispatchError,
		> {
			#[cfg(feature = "evm-tracing")]
			{
				use dp_evm_tracer::tracer::EvmTracer;
				use darwinia_ethereum::Call::transact;

				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;

				// Apply all extrinsics. Ethereum extrinsics are traced.
				for ext in _extrinsics.into_iter() {
					match &ext.0.function {
						Call::Ethereum(transact { transaction }) => {
							if _known_transactions.contains(&transaction.hash()) {
								// Each known extrinsic is a new call stack.
								EvmTracer::emit_new();
								EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
							} else {
								let _ = Executive::apply_extrinsic(ext);
							}
						}
						_ => {
							let _ = Executive::apply_extrinsic(ext);
						}
					};
				}

				Ok(())
			}
			#[cfg(not(feature = "evm-tracing"))]
			Err(sp_runtime::DispatchError::Other(
				"Missing `evm-tracing` compile time feature flag.",
			))
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (frame_support::weights::Weight, frame_support::weights::Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade().unwrap();

			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block_no_check(block: Block) -> frame_support::weights::Weight {
			Executive::execute_block_no_check(block)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, frame_benchmarking, BaselineBench::<Runtime>);
			list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
			list_benchmark!(list, extra, pallet_babe, Babe);
			list_benchmark!(list, extra, pallet_timestamp, Timestamp);
			// list_benchmark!(list, extra, darwinia_balances, Balances);
			// list_benchmark!(list, extra, darwinia_balances, Kton);
			list_benchmark!(list, extra, pallet_election_provider_multi_phase, ElectionProviderMultiPhase);
			// TODO wait for https://github.com/paritytech/substrate/issues/11068
			// list_benchmark!(list, extra, pallet_session, SessionBench::<Runtime>);
			list_benchmark!(list, extra, pallet_grandpa, Grandpa);
			list_benchmark!(list, extra, pallet_im_online, ImOnline);
			// list_benchmark!(list, extra, darwinia_header_mmr, HeaderMMR);
			list_benchmark!(list, extra, pallet_democracy, Democracy);
			list_benchmark!(list, extra, pallet_collective, Council);
			list_benchmark!(list, extra, pallet_collective, TechnicalCommittee);
			list_benchmark!(list, extra, pallet_elections_phragmen, PhragmenElection);
			list_benchmark!(list, extra, pallet_membership, TechnicalMembership);
			list_benchmark!(list, extra, pallet_treasury, Treasury);
			list_benchmark!(list, extra, pallet_treasury, KtonTreasury);
			list_benchmark!(list, extra, pallet_tips, Tips);
			list_benchmark!(list, extra, pallet_bounties, Bounties);
			list_benchmark!(list, extra, pallet_vesting, Vesting);
			list_benchmark!(list, extra, pallet_utility, Utility);
			list_benchmark!(list, extra, pallet_identity, Identity);
			list_benchmark!(list, extra, pallet_scheduler, Scheduler);
			list_benchmark!(list, extra, pallet_proxy, Proxy);
			list_benchmark!(list, extra, pallet_multisig, Multisig);
			// list_benchmark!(list, extra, darwinia_bridge_ethereum, EthereumRelay);
			// list_benchmark!(list, extra, to_ethereum_backing, EthereumBacking);
			// list_benchmark!(list, extra, darwinia_relayer_game, EthereumRelayerGame);
			// list_benchmark!(list, extra, darwinia_relay_authority, EcdsaRelayAuthority);
			// list_benchmark!(list, extra, to_tron_backing, TronBacking);
			list_benchmark!(list, extra, pallet_bridge_grandpa, BridgePangoroGrandpa);
			list_benchmark!(list, extra, pallet_bridge_grandpa, BridgeRococoGrandpa);
			// TODO: https://github.com/darwinia-network/darwinia-parachain/issues/66
			// list_benchmark!(list, extra, pallet_bridge_messages, BridgePangoroMessages);
			// list_benchmark!(list, extra, pallet_bridge_messages, BridgePangolinParachainMessages);
			list_benchmark!(list, extra, pallet_fee_market, PangoroFeeMarket);
			list_benchmark!(list, extra, pallet_fee_market, PangolinParachainFeeMarket);
			// list_benchmark!(list, extra, module_transaction_pause, TransactionPause);
			list_benchmark!(list, extra, from_substrate_issuing, Substrate2SubstrateIssuing);
			list_benchmark!(list, extra, to_parachain_backing, ToPangolinParachainBacking);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				array_bytes::hex2bytes_unchecked("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").into(),
				// Total Issuance
				array_bytes::hex2bytes_unchecked("0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").into(),
				// Execution Phase
				array_bytes::hex2bytes_unchecked("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").into(),
				// Event Count
				array_bytes::hex2bytes_unchecked("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").into(),
				// System Events
				array_bytes::hex2bytes_unchecked("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").into(),
				// Treasury Account
				array_bytes::hex2bytes_unchecked("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9d53c977f01e06f5b4be2d2a8ab85ede86d6f646c64612f74727372790000000000000000000000000000000000000000").into(),
			];
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_benchmarking, BaselineBench::<Runtime>);
			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			// TODO https://github.com/darwinia-network/darwinia-common/issues/1356
			// add_benchmark!(params, batches, pallet_babe, Babe);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);
			// add_benchmark!(params, batches, darwinia_balances, Balances);
			// add_benchmark!(params, batches, darwinia_balances, Kton);
			// FIXME https://github.com/darwinia-network/darwinia-common/issues/1359
			// add_benchmark!(params, batches, pallet_election_provider_multi_phase, ElectionProviderMultiPhase);
			// TODO https://github.com/paritytech/substrate/issues/11068
			// add_benchmark!(params, batches, pallet_session, SessionBench::<Runtime>);
			// TODO https://github.com/darwinia-network/darwinia-common/issues/1356
			// add_benchmark!(params, batches, pallet_grandpa, Grandpa);
			add_benchmark!(params, batches, pallet_im_online, ImOnline);
			// add_benchmark!(params, batches, darwinia_header_mmr, HeaderMMR);
			add_benchmark!(params, batches, pallet_democracy, Democracy);
			add_benchmark!(params, batches, pallet_collective, Council);
			add_benchmark!(params, batches, pallet_collective, TechnicalCommittee);
			add_benchmark!(params, batches, pallet_elections_phragmen, PhragmenElection);
			add_benchmark!(params, batches, pallet_membership, TechnicalMembership);
			add_benchmark!(params, batches, pallet_treasury, Treasury);
			add_benchmark!(params, batches, pallet_treasury, KtonTreasury);
			add_benchmark!(params, batches, pallet_tips, Tips);
			add_benchmark!(params, batches, pallet_bounties, Bounties);
			add_benchmark!(params, batches, pallet_vesting, Vesting);
			add_benchmark!(params, batches, pallet_utility, Utility);
			add_benchmark!(params, batches, pallet_identity, Identity);
			add_benchmark!(params, batches, pallet_scheduler, Scheduler);
			add_benchmark!(params, batches, pallet_proxy, Proxy);
			add_benchmark!(params, batches, pallet_multisig, Multisig);
			// add_benchmark!(params, batches, darwinia_bridge_ethereum, EthereumRelay);
			// add_benchmark!(params, batches, to_ethereum_backing, EthereumBacking);
			// add_benchmark!(params, batches, darwinia_relayer_game, EthereumRelayerGame);
			// add_benchmark!(list, extra, darwinia_relay_authority, EcdsaRelayAuthority);
			// add_benchmark!(params, batches, to_tron_backing, TronBacking);
			add_benchmark!(params, batches, pallet_bridge_grandpa, BridgePangoroGrandpa);
			add_benchmark!(params, batches, pallet_bridge_grandpa, BridgeRococoGrandpa);
			// TODO: https://github.com/darwinia-network/darwinia-parachain/issues/66
			// add_benchmark!(params, batches, pallet_bridge_messages, BridgePangoroMessages);
			// add_benchmark!(params, batches, pallet_bridge_messages, BridgePangolinParachainMessages);
			add_benchmark!(params, batches, pallet_fee_market, PangoroFeeMarket);
			add_benchmark!(params, batches, pallet_fee_market, PangolinParachainFeeMarket);
			// add_benchmark!(params, batches, module_transaction_pause, TransactionPause);
			add_benchmark!(params, batches, from_substrate_issuing, Substrate2SubstrateIssuing);
			add_benchmark!(params, batches, to_parachain_backing, ToPangolinParachainBacking);

			Ok(batches)
		}
	}
}

#[cfg(test)]
mod multiplier_tests {
	use super::*;
	use drml_common_runtime::{MinimumMultiplier, TargetBlockFullness};
	use frame_support::{pallet_prelude::Weight, weights::DispatchClass};
	use sp_runtime::traits::Convert;

	fn run_with_system_weight<F>(w: Weight, mut assertions: F)
	where
		F: FnMut() -> (),
	{
		let mut t: sp_io::TestExternalities =
			frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap().into();
		t.execute_with(|| {
			System::set_block_consumed_resources(w, 0);
			assertions()
		});
	}

	#[test]
	fn multiplier_can_grow_from_zero() {
		let minimum_multiplier = MinimumMultiplier::get();
		let target = TargetBlockFullness::get()
			* RuntimeBlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		// if the min is too small, then this will not change, and we are doomed forever.
		// the weight is 1/100th bigger than target.
		run_with_system_weight(target * 101 / 100, || {
			let next = SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
			assert!(next > minimum_multiplier, "{:?} !>= {:?}", next, minimum_multiplier);
		})
	}
}
