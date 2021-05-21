use crate::{frontier_backend_client, overrides::OverrideHandle};
pub use dvm_rpc_core::EthPubSubApiServer;
// --- darwinia ---
use dp_rpc::{
	pubsub::{Kind, Params, PubSubSyncStatus, Result as PubSubResult},
	Bytes, FilteredParams, Header, Log, Rich,
};
use dvm_rpc_core::EthPubSubApi::{self as EthPubSubApiT};
use dvm_rpc_runtime_api::{EthereumRuntimeRPCApi, TransactionStatus};
// --- substrate ---
use sc_client_api::{
	backend::{Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use sc_network::{ExHashT, NetworkService};
use sc_rpc::Metadata;
use sp_api::{BlockId, ProvideRuntimeApi};
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_io::hashing::twox_128;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT, UniqueSaturatedInto};
use sp_storage::{StorageData, StorageKey};
use sp_transaction_pool::TransactionPool;
// --- std ---
use codec::Decode;
use ethereum_types::{H256, U256};
use futures::{StreamExt as _, TryStreamExt as _};
use jsonrpc_core::{
	futures::{Future, Sink},
	Result as JsonRpcResult,
};
use jsonrpc_pubsub::{
	manager::{IdProvider, SubscriptionManager},
	typed::Subscriber,
	SubscriptionId,
};
use log::warn;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use std::{iter, marker::PhantomData, sync::Arc};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct HexEncodedIdProvider {
	len: usize,
}

impl Default for HexEncodedIdProvider {
	fn default() -> Self {
		Self { len: 16 }
	}
}

impl IdProvider for HexEncodedIdProvider {
	type Id = String;
	fn next_id(&self) -> Self::Id {
		let mut rng = thread_rng();
		let id: String = iter::repeat(())
			.map(|()| rng.sample(Alphanumeric))
			.take(self.len)
			.collect();

		array_bytes::bytes2hex("0x", id.as_bytes())
	}
}

pub struct EthPubSubApi<B: BlockT, P, C, BE, H: ExHashT> {
	_pool: Arc<P>,
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	subscriptions: SubscriptionManager<HexEncodedIdProvider>,
	overrides: Arc<OverrideHandle<B>>,
	_marker: PhantomData<(B, BE)>,
}
impl<B: BlockT, P, C, BE, H: ExHashT> EthPubSubApi<B, P, C, BE, H>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C::Api: EthereumRuntimeRPCApi<B>,
	C: Send + Sync + 'static,
{
	pub fn new(
		_pool: Arc<P>,
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		subscriptions: SubscriptionManager<HexEncodedIdProvider>,
		overrides: Arc<OverrideHandle<B>>,
	) -> Self {
		Self {
			_pool,
			client: client.clone(),
			network,
			subscriptions,
			overrides,
			_marker: PhantomData,
		}
	}
}

struct SubscriptionResult {}
impl SubscriptionResult {
	pub fn new() -> Self {
		SubscriptionResult {}
	}
	pub fn new_heads(&self, block: ethereum::Block) -> PubSubResult {
		PubSubResult::Header(Box::new(Rich {
			inner: Header {
				hash: Some(H256::from_slice(
					Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
				)),
				parent_hash: block.header.parent_hash,
				uncles_hash: block.header.ommers_hash,
				author: block.header.beneficiary,
				miner: block.header.beneficiary,
				state_root: block.header.state_root,
				transactions_root: block.header.transactions_root,
				receipts_root: block.header.receipts_root,
				number: Some(block.header.number),
				gas_used: block.header.gas_used,
				gas_limit: block.header.gas_limit,
				extra_data: Bytes(block.header.extra_data.clone()),
				logs_bloom: block.header.logs_bloom,
				timestamp: U256::from(block.header.timestamp),
				difficulty: block.header.difficulty,
				seal_fields: vec![
					Bytes(block.header.mix_hash.as_bytes().to_vec()),
					Bytes(block.header.nonce.as_bytes().to_vec()),
				],
				size: Some(U256::from(rlp::encode(&block).len() as u32)),
			},
			extra_info: BTreeMap::new(),
		}))
	}
	pub fn logs(
		&self,
		block: ethereum::Block,
		receipts: Vec<ethereum::Receipt>,
		params: &FilteredParams,
	) -> Vec<Log> {
		let block_hash = Some(H256::from_slice(
			Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
		));
		let mut logs: Vec<Log> = vec![];
		let mut log_index: u32 = 0;
		for (receipt_index, receipt) in receipts.into_iter().enumerate() {
			let mut transaction_log_index: u32 = 0;
			let transaction_hash: Option<H256> = if receipt.logs.len() > 0 {
				Some(H256::from_slice(
					Keccak256::digest(&rlp::encode(&block.transactions[receipt_index as usize]))
						.as_slice(),
				))
			} else {
				None
			};
			for log in receipt.logs {
				if self.add_log(block_hash.unwrap(), &log, &block, params) {
					logs.push(Log {
						address: log.address,
						topics: log.topics,
						data: Bytes(log.data),
						block_hash,
						block_number: Some(block.header.number),
						transaction_hash,
						transaction_index: Some(U256::from(receipt_index)),
						log_index: Some(U256::from(log_index)),
						transaction_log_index: Some(U256::from(transaction_log_index)),
						removed: false,
					});
				}
				log_index += 1;
				transaction_log_index += 1;
			}
		}
		logs
	}
	fn add_log(
		&self,
		block_hash: H256,
		ethereum_log: &ethereum::Log,
		block: &ethereum::Block,
		params: &FilteredParams,
	) -> bool {
		let log = Log {
			address: ethereum_log.address.clone(),
			topics: ethereum_log.topics.clone(),
			data: Bytes(ethereum_log.data.clone()),
			block_hash: None,
			block_number: None,
			transaction_hash: None,
			transaction_index: None,
			log_index: None,
			transaction_log_index: None,
			removed: false,
		};
		if let Some(_) = params.filter {
			let block_number =
				UniqueSaturatedInto::<u64>::unique_saturated_into(block.header.number);
			if !params.filter_block_range(block_number)
				|| !params.filter_block_hash(block_hash)
				|| !params.filter_address(&log)
				|| !params.filter_topics(&log)
			{
				return false;
			}
		}
		true
	}
}

fn storage_prefix_build(module: &[u8], storage: &[u8]) -> Vec<u8> {
	[twox_128(module), twox_128(storage)].concat().to_vec()
}

impl<B: BlockT, P, C, BE, H: ExHashT> EthPubSubApiT for EthPubSubApi<B, P, C, BE, H>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	P: TransactionPool<Block = B> + Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE> + BlockchainEvents<B>,
	C: HeaderBackend<B> + HeaderMetadata<B, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: EthereumRuntimeRPCApi<B>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	type Metadata = Metadata;
	fn subscribe(
		&self,
		_metadata: Self::Metadata,
		subscriber: Subscriber<PubSubResult>,
		kind: Kind,
		params: Option<Params>,
	) {
		let filtered_params = match params {
			Some(Params::Logs(filter)) => FilteredParams::new(Some(filter)),
			_ => FilteredParams::default(),
		};
		let client = self.client.clone();
		let network = self.network.clone();
		let overrides = self.overrides.clone();
		match kind {
			Kind::Logs => {
				self.subscriptions.add(subscriber, |sink| {
					let stream = client
						.import_notification_stream()
						.filter_map(move |notification| {
							if notification.is_new_best {
								let id = BlockId::Hash(notification.hash);
								let schema = frontier_backend_client::onchain_storage_schema::<
									B,
									C,
									BE,
								>(client.as_ref(), id);
								let handler = overrides
									.schemas
									.get(&schema)
									.unwrap_or(&overrides.fallback);

								let block = handler.current_block(&id);
								let receipts = handler.current_receipts(&id);
								match (receipts, block) {
									(Some(receipts), Some(block)) => {
										futures::future::ready(Some((block, receipts)))
									}
									_ => futures::future::ready(None),
								}
							} else {
								futures::future::ready(None)
							}
						})
						.flat_map(move |(block, receipts)| {
							futures::stream::iter(SubscriptionResult::new().logs(
								block,
								receipts,
								&filtered_params,
							))
						})
						.map(|x| {
							return Ok::<Result<PubSubResult, jsonrpc_core::types::error::Error>, ()>(
								Ok(PubSubResult::Log(Box::new(x))),
							);
						})
						.compat();
					sink.sink_map_err(|e| warn!("Error sending notifications: {:?}", e))
						.send_all(stream)
						.map(|_| ())
				});
			}
			Kind::NewHeads => {
				self.subscriptions.add(subscriber, |sink| {
					let stream = client
						.import_notification_stream()
						.filter_map(move |notification| {
							if notification.is_new_best {
								let id = BlockId::Hash(notification.hash);
								let schema = frontier_backend_client::onchain_storage_schema::<
									B,
									C,
									BE,
								>(client.as_ref(), id);
								let handler = overrides
									.schemas
									.get(&schema)
									.unwrap_or(&overrides.fallback);

								let block = handler.current_block(&id);
								futures::future::ready(block)
							} else {
								futures::future::ready(None)
							}
						})
						.map(|block| {
							return Ok::<_, ()>(Ok(SubscriptionResult::new().new_heads(block)));
						})
						.compat();
					sink.sink_map_err(|e| warn!("Error sending notifications: {:?}", e))
						.send_all(stream)
						.map(|_| ())
				});
			}
			Kind::NewPendingTransactions => {
				if let Ok(stream) = client.storage_changes_notification_stream(
					Some(&[StorageKey(storage_prefix_build(b"Ethereum", b"Pending"))]),
					None,
				) {
					self.subscriptions.add(subscriber, |sink| {
						let stream = stream
							.flat_map(|(_block, changes)| {
								let mut transactions: Vec<ethereum::Transaction> = vec![];
								let storage: Vec<Option<StorageData>> = changes
									.iter()
									.filter_map(|(o_sk, _k, v)| {
										if o_sk.is_none() {
											Some(v.cloned())
										} else {
											None
										}
									})
									.collect();
								for change in storage {
									if let Some(data) = change {
										let storage: Vec<(
											ethereum::Transaction,
											TransactionStatus,
											ethereum::Receipt,
										)> = Decode::decode(&mut &data.0[..]).unwrap();
										let tmp: Vec<ethereum::Transaction> =
											storage.iter().map(|x| x.0.clone()).collect();
										transactions.extend(tmp);
									}
								}
								futures::stream::iter(transactions)
							})
							.map(|transaction| {
								return Ok::<
									Result<PubSubResult, jsonrpc_core::types::error::Error>,
									(),
								>(Ok(PubSubResult::TransactionHash(H256::from_slice(
									Keccak256::digest(&rlp::encode(&transaction)).as_slice(),
								))));
							})
							.compat();

						sink.sink_map_err(|e| warn!("Error sending notifications: {:?}", e))
							.send_all(stream)
							.map(|_| ())
					});
				}
			}
			Kind::Syncing => {
				self.subscriptions.add(subscriber, |sink| {
					let mut previous_syncing = network.is_major_syncing();
					let stream = client
						.import_notification_stream()
						.filter_map(move |notification| {
							let syncing = network.is_major_syncing();
							if notification.is_new_best && previous_syncing != syncing {
								previous_syncing = syncing;
								futures::future::ready(Some(syncing))
							} else {
								futures::future::ready(None)
							}
						})
						.map(|syncing| {
							return Ok::<Result<PubSubResult, jsonrpc_core::types::error::Error>, ()>(
								Ok(PubSubResult::SyncState(PubSubSyncStatus { syncing })),
							);
						})
						.compat();
					sink.sink_map_err(|e| warn!("Error sending notifications: {:?}", e))
						.send_all(stream)
						.map(|_| ())
				});
			}
		}
	}

	fn unsubscribe(
		&self,
		_metadata: Option<Self::Metadata>,
		subscription_id: SubscriptionId,
	) -> JsonRpcResult<bool> {
		Ok(self.subscriptions.cancel(subscription_id))
	}
}
