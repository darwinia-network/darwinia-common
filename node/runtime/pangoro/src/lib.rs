//! The Pangoro runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

pub mod pallets;
pub use pallets::*;

#[cfg(not(feature = "no-wasm"))]
pub mod wasm {
	//! Make the WASM binary available.

	include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

	/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
	#[cfg(feature = "std")]
	pub fn wasm_binary_unwrap() -> &'static [u8] {
		return WASM_BINARY.expect(
			"Development wasm binary is not available. This means the client is \
			built with `SKIP_WASM_BUILD` flag and it is only usable for \
			production chains. Please rebuild with the flag disabled.",
		);
	}
}
#[cfg(not(feature = "no-wasm"))]
pub use wasm::*;

pub mod pangolin_messages;
use pangolin_messages::{ToPangolinMessagePayload, WithPangolinMessageBridge};

pub use drml_common_primitives as pangoro_primitives;
pub use drml_common_primitives as pangolin_primitives;

pub use common_runtime as pangoro_runtime_system_params;
pub use common_runtime as pangolin_runtime_system_params;

pub use darwinia_balances::Call as BalancesCall;
pub use darwinia_fee_market::Call as FeeMarketCall;
pub use frame_system::Call as SystemCall;
pub use pallet_bridge_grandpa::Call as BridgeGrandpaCall;
pub use pallet_bridge_messages::Call as BridgeMessagesCall;
pub use pallet_sudo::Call as SudoCall;

// --- crates.io ---
use codec::{Decode, Encode};
// --- paritytech ---
use fp_storage::PALLET_ETHEREUM_SCHEMA;
#[allow(unused)]
use frame_support::{log, migration};
use frame_support::{
	traits::{KeyOwnerProofSystem, OnRuntimeUpgrade},
	weights::Weight,
};
use frame_system::{
	offchain::{AppCrypto, CreateSignedTransaction, SendTransactionTypes, SigningTypes},
	ChainContext, CheckEra, CheckGenesis, CheckNonce, CheckSpecVersion, CheckTxVersion,
	CheckWeight, EnsureRoot,
};
use pallet_evm::FeeCalculator;
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use pallet_transaction_payment::{ChargeTransactionPayment, FeeDetails, RuntimeDispatchInfo};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::{AllowedSlots, BabeEpochConfiguration};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	generic,
	traits::{
		Block as BlockT, Dispatchable, Extrinsic, NumberFor, PostDispatchInfoOf, StaticLookup,
		Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
	ApplyExtrinsicResult, MultiAddress, OpaqueExtrinsic, SaturatedConversion,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia-network ---
use common_runtime::*;
use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo as BalancesRuntimeDispatchInfo;
use darwinia_evm::{AccountBasic, Runner};
use darwinia_fee_market_rpc_runtime_api::{Fee, InProcessOrders};
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo as StakingRuntimeDispatchInfo;
use drml_bridge_primitives::{PANGOLIN_CHAIN_ID, PANGORO_CHAIN_ID};
use drml_common_primitives::*;
use dvm_ethereum::EthereumStorageSchema;

pub type Address = MultiAddress<AccountId, ()>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedBlock = generic::SignedBlock<Block>;
pub type SignedExtra = (
	CheckSpecVersion<Runtime>,
	CheckTxVersion<Runtime>,
	CheckGenesis<Runtime>,
	CheckEra<Runtime>,
	CheckNonce<Runtime>,
	CheckWeight<Runtime>,
	ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	ChainContext<Runtime>,
	Runtime,
	AllPallets,
	CustomOnRuntimeUpgrade,
>;
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

pub type Ring = Balances;

pub type RootOrigin = EnsureRoot<AccountId>;

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: sp_runtime::create_runtime_str!("Pangoro"),
	impl_name: sp_runtime::create_runtime_str!("Pangoro"),
	authoring_version: 0,
	spec_version: 2_8_02_0,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: BabeEpochConfiguration = BabeEpochConfiguration {
	c: PRIMARY_PROBABILITY,
	allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots,
};

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = OpaqueBlock,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,

		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 2,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Balances: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 6,

		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 7,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 8,
		Staking: darwinia_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 9,
		Offences: pallet_offences::{Pallet, Storage, Event} = 10,
		Historical: pallet_session_historical::{Pallet} = 11,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 12,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event} = 13,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned} = 14,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config} = 15,

		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 24,

		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 16,

		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 21,

		BridgePangolinDispatch: pallet_bridge_dispatch::<Instance1>::{Pallet, Event<T>} = 18,
		BridgePangolinGrandpa: pallet_bridge_grandpa::<Instance1>::{Pallet, Call, Storage} = 19,
		BridgePangolinMessages: pallet_bridge_messages::<Instance1>::{Pallet, Call, Storage, Event<T>} = 17,

		FeeMarket: darwinia_fee_market::{Pallet, Call, Storage, Event<T>} = 22,
		TransactionPause: module_transaction_pause::{Pallet, Call, Storage, Event<T>} = 23,

		Substrate2SubstrateBacking: to_substrate_backing::{Pallet, Call, Storage, Config<T>, Event<T>} = 20,

		EVM: darwinia_evm::{Pallet, Call, Storage, Config, Event<T>} = 25,
		Ethereum: dvm_ethereum::{Pallet, Call, Storage, Config, Event, Origin} = 26,
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 27,

		Bsc: darwinia_bridge_bsc::{Pallet, Call, Storage, Config} = 46,
	}
);

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
		let period = BlockHashCountForPangoro::get()
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

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			Call::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(&self, info: &Self::SignedInfo) -> Option<TransactionValidity> {
		match self {
			Call::Ethereum(call) => call.validate_self_contained(info),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.pre_dispatch_self_contained(info),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ Call::Ethereum(dvm_ethereum::Call::transact { .. }) => Some(call.dispatch(
				Origin::from(dvm_ethereum::RawOrigin::EthereumTransaction(info)),
			)),
			_ => None,
		}
	}
}

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

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
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
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
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

	impl darwinia_staking_rpc_runtime_api::StakingApi<Block, AccountId, Power> for Runtime {
		fn power_of(account: AccountId) -> StakingRuntimeDispatchInfo<Power> {
			Staking::power_of_rpc(account)
		}
	}

	impl darwinia_fee_market_rpc_runtime_api::FeeMarketApi<Block, Balance> for Runtime {
		fn market_fee() -> Option<Fee<Balance>> {
			if let Some(fee) = FeeMarket::market_fee() {
				return Some(Fee {
					amount: fee,
				});
			}
			None
		}

		fn in_process_orders() -> InProcessOrders {
			return InProcessOrders {
				orders: FeeMarket::in_process_orders(),
			}
		}
	}

	impl drml_bridge_primitives::PangolinFinalityApi<Block> for Runtime {
		fn best_finalized() -> (pangolin_primitives::BlockNumber, pangolin_primitives::Hash) {
			let header = BridgePangolinGrandpa::best_finalized();
			(header.number, header.hash())
		}

		fn is_known_header(hash: pangolin_primitives::Hash) -> bool {
			BridgePangolinGrandpa::is_known_header(hash)
		}
	}

	impl drml_bridge_primitives::ToPangolinOutboundLaneApi<Block, Balance, ToPangolinMessagePayload> for Runtime {
		// fn estimate_message_delivery_and_dispatch_fee(
		// 	_lane_id: bp_messages::LaneId,
		// 	payload: ToPangolinMessagePayload,
		// ) -> Option<Balance> {
		// 	bridge_runtime_common::messages::source::estimate_message_dispatch_and_delivery_fee::<WithPangolinMessageBridge>(
		// 		&payload,
		// 		WithPangolinMessageBridge::RELAYER_FEE_PERCENT,
		// 	).ok()
		// }

		fn message_details(
			lane: bp_messages::LaneId,
			begin: bp_messages::MessageNonce,
			end: bp_messages::MessageNonce,
		) -> Vec<bp_messages::MessageDetails<Balance>> {
			bridge_runtime_common::messages_api::outbound_message_details::<
				Runtime,
				WithPangolinMessages,
				WithPangolinMessageBridge,
			>(lane, begin, end)
		}

		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::outbound_latest_received_nonce(lane)
		}

		fn latest_generated_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::outbound_latest_generated_nonce(lane)
		}
	}

	impl drml_bridge_primitives::FromPangolinInboundLaneApi<Block> for Runtime {
		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::inbound_latest_received_nonce(lane)
		}

		fn latest_confirmed_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::inbound_latest_confirmed_nonce(lane)
		}

		fn unrewarded_relayers_state(lane: bp_messages::LaneId) -> bp_messages::UnrewardedRelayersState {
			BridgePangolinMessages::inbound_unrewarded_relayers_state(lane)
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
			<Runtime as darwinia_evm::Config>::RingAccountBasic::account_basic(&address)
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
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				Vec::new(),
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
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				Vec::new(),
				config.as_ref().unwrap_or(<Runtime as darwinia_evm::Config>::config()),
			).map_err(|err| err.into())
		}


		fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<dvm_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<dvm_ethereum::EthereumReceiptV0>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<dvm_ethereum::Block>,
			Option<Vec<dvm_ethereum::EthereumReceiptV0>>,
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
		) -> Vec<dvm_ethereum::Transaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				Call::Ethereum(dvm_ethereum::Call::transact { transaction }) => Some(transaction),
				_ => None
			}).collect()
		}
	}

	impl dp_evm_trace_apis::DebugRuntimeApi<Block> for Runtime {
		fn trace_transaction(
			_extrinsics: Vec<<Block as BlockT>::Extrinsic>,
			_traced_transaction: &dvm_ethereum::Transaction,
		) -> Result<
			(),
			sp_runtime::DispatchError,
		> {
			#[cfg(feature = "evm-tracing")]
			{
				use dp_evm_tracer::tracer::EvmTracer;
				use dvm_ethereum::Call::transact;
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
				use sha3::{Digest, Keccak256};
				use dvm_ethereum::Call::transact;

				let mut config = <Runtime as darwinia_evm::Config>::config().clone();
				config.estimate = true;

				// Apply all extrinsics. Ethereum extrinsics are traced.
				for ext in _extrinsics.into_iter() {
					match &ext.0.function {
						Call::Ethereum(transact { transaction }) => {
							let eth_extrinsic_hash =
								H256::from_slice(Keccak256::digest(&rlp::encode(transaction)).as_slice());
							if _known_transactions.contains(&eth_extrinsic_hash) {
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
		fn on_runtime_upgrade() -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade().unwrap();

			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block_no_check(block: Block) -> Weight {
			Executive::execute_block_no_check(block)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
			list_benchmark!(list, extra, to_substrate_backing, Substrate2SubstrateBacking);
			list_benchmark!(list, extra, darwinia_bridge_bsc, Bsc);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

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
			add_benchmark!(params, batches, to_substrate_backing, Substrate2SubstrateBacking);
			add_benchmark!(params, batches, darwinia_bridge_bsc, Bsc);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }

			Ok(batches)
		}
	}
}

#[derive(Clone)]
pub struct TransactionConverter;
impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(dvm_ethereum::Call::transact { transaction }.into())
	}
}
impl fp_rpc::ConvertTransaction<OpaqueExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: dvm_ethereum::Transaction) -> OpaqueExtrinsic {
		let extrinsic =
			UncheckedExtrinsic::new_unsigned(dvm_ethereum::Call::transact { transaction }.into());
		let encoded = extrinsic.encode();

		OpaqueExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

fn migrate() -> Weight {
	<darwinia_staking::MinimumValidatorCount<Runtime>>::put(2);
	frame_support::storage::unhashed::put::<EthereumStorageSchema>(
		&PALLET_ETHEREUM_SCHEMA,
		&EthereumStorageSchema::V2,
	);
	if let Ok(bytes) = array_bytes::hex2bytes(
		"0x5cb4b6631001facd57be810d5d1383ee23a31257d2430f097291d25fc1446d4f1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347e9ae3261a475a27bb1028f140bc2a7c843318afda6cd7017374dfe102e82d2b3b8a43dbe1d41cc0e4569f3dc45db6c4e687949ae657f5876113ac9abe5cf0460aa8d6b3b53abfc336cea4ab3ee594586f8b584ca1bfba16a9e34a12ff7c4b88be484ccd8065b90abea026f6c1f97c257fdb4ad2b2c30123db854d838c878e978cd2117896aa092e4ce08f078424e9ec7f2312f1909b35e579fb2702d571a3be04a8f01328e51af205100a7c32e3dd8faf8222fcf03f3545655314abf91c4c0d80cea6aa46f122c2a9c596c6a99d5842786d40667eb195877bbbb128890a824506c81a9e5623d4355e08a16f384bf709bf4db598bbcb88150abcd4ceba89cc798000bdccf5cf4d58d50828d3b7dc2bc5d8a928a32d24b845857da0b5bcf2c5dec8230643d4bec452491ba1260806a9e68a4a530de612e5c2676955a17400ce1d4fd6ff458bc38a8b1826e1c1d24b9516ef84ea6d8721344502a6c732ed7f861bb0ea017d520bad5fa53cfc67c678a2e6f6693c8ee0200000000000000000000000000000000000000000000000000000000000000c8947500000000007af38f030000000000000000000000000000000000000000000000000000000017403601000000000000000000000000000000000000000000000000000000003771ac60000000001508d883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c0670403d7dfc4c816a313885fe04b850f96f27b2e9fd88b147c882ad7caf9b964abfe6543625fcca73b56fe29d3046831574b0681d52bf5383d6f2187b6276c1000000000000000000000000000000000000000000000000000000000000000000200000000000000000"
	) {
		if let Ok(genesis_header) = bsc_primitives::BscHeader::decode(&mut &*bytes) {
			let initial_authority_set = <darwinia_bridge_bsc::Pallet<Runtime>>::extract_authorities(&genesis_header).unwrap();

			<darwinia_bridge_bsc::Authorities<Runtime>>::put(&initial_authority_set);
			<darwinia_bridge_bsc::FinalizedAuthorities<Runtime>>::put(&initial_authority_set);
			<darwinia_bridge_bsc::FinalizedCheckpoint<Runtime>>::put(&genesis_header);
			<darwinia_bridge_bsc::AuthoritiesOfRound<Runtime>>::insert(
				&genesis_header.number / <Runtime as darwinia_bridge_bsc::Config>::BscConfiguration::get().epoch_length,
				(0u32..initial_authority_set.len() as u32).collect::<Vec<u32>>(),
			);
		}
	}

	// 0
	RuntimeBlockWeights::get().max_block
}

pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	fn on_runtime_upgrade() -> Weight {
		migrate()
	}
}

/// Pangolin account ownership digest from Pangoro.
///
/// The byte vector returned by this function should be signed with a Pangolin account private key.
/// This way, the owner of `pangoro_account_id` on Pangoro proves that the Pangolin account private key
/// is also under his control.
pub fn pangoro_to_pangolin_account_ownership_digest<Call, AccountId, SpecVersion>(
	pangolin_call: &Call,
	pangoro_account_id: AccountId,
	pangolin_spec_version: SpecVersion,
) -> sp_std::vec::Vec<u8>
where
	Call: Encode,
	AccountId: Encode,
	SpecVersion: Encode,
{
	pallet_bridge_dispatch::account_ownership_digest(
		pangolin_call,
		pangoro_account_id,
		pangolin_spec_version,
		PANGORO_CHAIN_ID,
		PANGOLIN_CHAIN_ID,
	)
}
