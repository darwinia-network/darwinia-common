#![cfg_attr(not(feature = "std"), no_std)]

pub mod crypto {
	// --- third-party ---
	use sp_runtime::app_crypto::{app_crypto, sr25519};

	// --- custom ---
	use crate::KEY_TYPE;

	app_crypto!(sr25519, KEY_TYPE);
}

mod ethscan_url {
	pub const GTE_BLOCK: &'static [u8] =
		b"https://api.etherscan.io/api?module=proxy&action=eth_getBlockByNumber&tag=0x";
}

#[cfg(all(feature = "std", test))]
mod mock;
#[cfg(all(feature = "std", test))]
mod tests;

// --- core ---
use core::str::from_utf8;
// --- substrate ---
use frame_support::{debug, decl_error, decl_event, decl_module, traits::Get};
use frame_system::{self as system, offchain::SubmitSignedTransaction};
use sp_runtime::{
	offchain::{http::Request, storage::StorageValueRef},
	traits::Zero,
	DispatchError, DispatchResult, KeyTypeId,
};
use sp_std::prelude::*;
// --- darwinia ---
use eth_primitives::header::EthHeader;

type ApiKey = [u8; 34];

type EthRelay<T> = darwinia_eth_relay::Module<T>;
type EthRelayCall<T> = darwinia_eth_relay::Call<T>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"rlwk");

const MAX_RETRY: u8 = 3;

pub trait Trait: darwinia_eth_relay::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	type Call: From<EthRelayCall<Self>>;

	type SubmitSignedTransaction: SubmitSignedTransaction<Self, <Self as Trait>::Call>;

	type FetchInterval: Get<Self::BlockNumber>;
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = <T as system::Trait>::AccountId
	{
		OffchainRelayChainApiKey(AccountId), // currently not use, implement someday not now
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Local accounts - UNAVAILABLE (Consider adding one via `author_insertKey` RPC)
		AccountUnavail,

		/// API Resoibse - UNEXPECTED
		APIRespUnexp,

		/// Best Header - NOT EXISTED
		BestHeaderNE,
		/// Block Number - OVERFLOW
		BlockNumberOF,

		/// Request - REACH MAX RETRY
		ReqRMR,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		const FetchInterval: T::BlockNumber = T::FetchInterval::get();

		fn deposit_event() = default;

		fn offchain_worker(block: T::BlockNumber) {
			if let Some(maybe_key) = StorageValueRef::persistent(b"eapi").get::<ApiKey>() {
				if let Some(key) = maybe_key {
					let fetch_interval = T::FetchInterval::get().max(1.into());
					if (block % fetch_interval).is_zero() {
						debug::trace!(
							target: "eoc-ek",
							"[eth-offchain] EtherScan API Key: {:?}",
							from_utf8(&key).unwrap_or_default(),
						);

						let _ = Self::relay_header(key);
					}
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn relay_header(key: ApiKey) -> DispatchResult {
		if !T::SubmitSignedTransaction::can_sign() {
			Err(<Error<T>>::AccountUnavail)?;
		}

		let target_number = Self::get_target_number()?;
		let block_info_url = Self::build_url(vec![
			ethscan_url::GTE_BLOCK.to_vec(),
			base_n_bytes(target_number, 16),
			"&boolean=true&apikey=".as_bytes().to_vec(),
			key.to_vec(),
		])?;
		let header = Self::fetch_header(from_utf8(&block_info_url).unwrap_or_default())?;

		Self::submit_header(header);

		Ok(())
	}

	fn get_target_number() -> Result<u64, DispatchError> {
		let target_number = <EthRelay<T>>::header_of(<EthRelay<T>>::best_header_hash())
			.ok_or(<Error<T>>::BestHeaderNE)?
			.number
			.checked_add(1)
			.ok_or(<Error<T>>::BlockNumberOF)?;
		debug::trace!(target: "eoc-bn", "[eth-offchain] Target Number: {}", target_number);

		Ok(target_number)
	}

	fn build_url(params: Vec<Vec<u8>>) -> Result<Vec<u8>, DispatchError> {
		let mut url = vec![];
		for mut param in params {
			url.append(&mut param);
		}

		debug::trace!(target: "eoc-url", "[eth-offchain] Block Info Url: {}", from_utf8(&url).unwrap_or_default());

		Ok(url)
	}

	fn fetch_header(url: &str) -> Result<EthHeader, DispatchError> {
		let mut maybe_resp_body = None;
		for retry_time in 0..=MAX_RETRY {
			debug::trace!(target: "eoc-req", "[eth-offchain] Retry: {}", retry_time);
			if let Ok(pending) = Request::get(&url).send() {
				if let Ok(resp) = pending.wait() {
					if resp.code == 200 {
						let resp_body = resp.body().collect::<Vec<u8>>();
						if resp_body[0] == 123u8 {
							maybe_resp_body = Some(resp_body);
							break;
						}
					} else {
						debug::trace!(target: "eoc-req", "[eth-offchain] Status Code: {}", resp.code);
					}
				}
			}
		}

		let resp_body = maybe_resp_body.ok_or(<Error<T>>::ReqRMR)?;
		debug::trace!(
			target: "eoc-req",
			"[eth-offchain] Response: {}",
			from_utf8(&resp_body).unwrap_or_default(),
		);
		if resp_body.len() < 1362 {
			Err(<Error<T>>::APIRespUnexp)?;
		}
		let raw_header = from_utf8(&resp_body[33..resp_body.len() - 1]).unwrap_or_default();

		let header = EthHeader::from_str_unchecked(raw_header);
		debug::trace!(target: "eoc-hd", "[eth-offchain] Relay: {:?}", header);

		Ok(header)
	}

	fn submit_header(header: EthHeader) {
		// FIXME: disable the submmit header
		//		let results =
		//			T::SubmitSignedTransaction::submit_signed(<EthRelayCall<T>>::relay_header(header));
		//		for (account, result) in &results {
		//			debug::trace!(
		//				target: "eoc-rl",
		//				"[eth-offchain] Account: {:?}, Relay: {:?}",
		//				account,
		//				result,
		//			);
		//		}
	}
}

fn base_n_bytes(mut x: u64, radix: u64) -> Vec<u8> {
	if radix > 41 {
		return vec![];
	}

	let mut buf = vec![];
	while x > 0 {
		let rem = (x % radix) as u8;
		if rem < 10 {
			buf.push(48 + rem);
		} else {
			buf.push(55 + rem);
		}
		x /= radix;
	}

	buf.reverse();
	buf
}
