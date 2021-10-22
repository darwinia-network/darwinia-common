pub use darwinia_evm_precompile_dispatch::Dispatch;
pub use darwinia_evm_precompile_misc::Misc;
pub use darwinia_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
pub use darwinia_evm_precompile_transfer::Transfer;
use darwinia_support::s2s::{nonce_to_message_id, RelayMessageSender, TokenMessageId};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Pangoro;

// --- crates.io ---
use evm::{executor::PrecompileOutput, Context, ExitError};
// --- paritytech ---
use codec::{Decode, Encode};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{FindAuthor, PalletInfoAccess},
	ConsensusEngineId,
};
use sp_core::{crypto::Public, H160, U256};
use sp_runtime::DispatchErrorWithPostInfo;
use sp_std::marker::PhantomData;
// --- darwinia-network ---
use crate::*;
use darwinia_evm::{runner::stack::Runner, Config, EnsureAddressTruncated, FeeCalculator};
use darwinia_support::evm::ConcatConverter;
use dp_evm::{Precompile, PrecompileSet};
use dvm_ethereum::{
	account_basic::{DvmAccountBasic, KtonRemainBalance, RingRemainBalance},
	EthereumBlockHashMapping,
};

pub struct EthereumFindAuthor<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Babe::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.0.to_raw_vec()[4..24]));
		}
		None
	}
}

pub struct ToPangoroMessageSender;

impl ToPangoroMessageSender {
	fn send_message_call(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<Call, &'static str> {
		let payload = ToPangoroMessagePayload::decode(&mut payload.as_slice())
			.map_err(|_| "decode pangoro payload failed")?;

		let call: Call = match pallet_index {
			_ if pallet_index as usize == <BridgePangoroMessages as PalletInfoAccess>::index() => {
				BridgeMessagesCall::<Runtime, Pangoro>::send_message(
					lane_id,
					payload,
					fee.saturated_into(),
				)
				.into()
			}
			_ => {
				return Err("invalid pallet index".into());
			}
		};
		Ok(call)
	}
}

impl RelayMessageSender for ToPangoroMessageSender {
	fn encode_send_message(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		let call = Self::send_message_call(pallet_index, lane_id, payload, fee)?;
		Ok(call.encode())
	}

	fn send_message_by_root(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call = Self::send_message_call(pallet_index, lane_id, payload, fee)?;
		call.dispatch(RawOrigin::Root.into())
	}

	fn latest_token_message_id(lane_id: [u8; 4]) -> TokenMessageId {
		let nonce: u64 = BridgePangoroMessages::outbound_latest_generated_nonce(lane_id).into();
		nonce_to_message_id(&lane_id, nonce)
	}

	fn latest_received_token_message_id(lane_id: [u8; 4]) -> TokenMessageId {
		let nonce: u64 = BridgePangoroMessages::inbound_latest_received_nonce(lane_id).into();
		nonce_to_message_id(&lane_id, nonce)
	}
}

pub struct PangolinPrecompiles<R>(PhantomData<R>);
impl<R> PrecompileSet for PangolinPrecompiles<R>
where
	R: from_substrate_issuing::Config + from_ethereum_issuing::Config,
	R: darwinia_evm::Config,
	R::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode + Decode,
	<R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
	R::Call: From<from_ethereum_issuing::Call<R>> + From<from_substrate_issuing::Call<R>>,
{
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> Option<Result<PrecompileOutput, ExitError>> {
		let addr = |n: u64| -> H160 { H160::from_low_u64_be(n) };

		match address {
			// Ethereum precompiles
			_ if address == addr(1) => Some(ECRecover::execute(input, target_gas, context)),
			_ if address == addr(2) => Some(Sha256::execute(input, target_gas, context)),
			_ if address == addr(3) => Some(Ripemd160::execute(input, target_gas, context)),
			_ if address == addr(4) => Some(Identity::execute(input, target_gas, context)),
			// Darwinia precompiles
			_ if address == addr(21) => Some(<Transfer<R>>::execute(input, target_gas, context)),
			_ if address == addr(24) => Some(<Misc<R, ToPangoroMessageSender>>::execute(
				input, target_gas, context,
			)),
			_ if address == addr(25) => Some(<Dispatch<R>>::execute(input, target_gas, context)),
			_ => None,
		}
	}
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		U256::from(1 * COIN)
	}
}

frame_support::parameter_types! {
	pub const ChainId: u64 = 43;
	pub BlockGasLimit: U256 = u32::max_value().into();
}

impl Config for Runtime {
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressTruncated<Self::AccountId>;
	type IntoAccountId = ConcatConverter<Self::AccountId>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Event = Event;
	type Precompiles = PangolinPrecompiles<Self>;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type RingAccountBasic = DvmAccountBasic<Self, Ring, RingRemainBalance>;
	type KtonAccountBasic = DvmAccountBasic<Self, Kton, KtonRemainBalance>;
	type Runner = Runner<Self>;
}
