//! Module to relay blocks from Ethereum Network
//!
//! In this module,
//! the offchain worker will keep fetch the next block info and relay to Darwinia Network
//!
//! The ethscan apikey may or may not be inserted by following js command
//! `api.rpc.offchain.localStorageSet(1,"eapi","XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX")`
//!
//! Request on this web site to get your ethsacn apikey
//! https://etherscan.io/apis
//!
//! Here is the basic flow.
//! The starting point is the `offchain_worker`
//! - base on block schedule, the `relay_header` will be called
//! - then base on the `ApiKey`, the `relay_header` will get ethereum blocks from EthScan or
//!   Cloudflare Ethereum Gateway
//! - After the http response corrected fetched, we will validate not only the format of http
//!   response but also the correct the Ethereum header as the light client do
//! - After all, the corrected Ethereum header will be recorded on Darwinia Network by
//!   `submit_header`
//!
//! More details can get in these PRs
//! https://github.com/darwinia-network/darwinia/pull/335
//! https://github.com/darwinia-network/darwinia-common/pull/43
//! https://github.com/darwinia-network/darwinia-common/pull/63
#![cfg_attr(not(feature = "std"), no_std)]

pub mod crypto {
	// --- substrate ---
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	// --- darwinia ---
	use crate::KEY_TYPE;

	app_crypto!(sr25519, KEY_TYPE);
}

mod ethscan_url {
	pub const GET_BLOCK: &'static [u8] =
		b"https://api.etherscan.io/api?module=proxy&action=eth_getBlockByNumber&tag=0x";
}

mod ethgateway_url {
	pub const GET_BLOCK: &'static [u8] = b"https://cloudflare-eth.com/";
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
	DispatchError, KeyTypeId,
};
use sp_std::prelude::*;
// --- darwinia ---
use eth_primitives::header::EthHeader;

type ApiKey = [u8; 34];

type EthRelay<T> = darwinia_eth_relay::Module<T>;
type EthRelayCall<T> = darwinia_eth_relay::Call<T>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"rlwk");

const MAX_REDIRECT_TIMES: u8 = 3;

#[derive(Default)]
struct OffchainRequest {
	location: Vec<u8>,
	may_payload: Option<Vec<u8>>,
	redirect_times: u8,
	cookie: Option<Vec<u8>>,
}

/// The OffhcainRequest handle the request session
/// - set cookie if returns
/// - handle the redirect actions if happends
impl OffchainRequest {
	pub fn new(url: Vec<u8>, may_payload: Option<Vec<u8>>) -> Self {
		OffchainRequest {
			location: url.clone(),
			may_payload,
			..Default::default()
		}
	}

	pub fn send(mut self) -> Option<Vec<u8>> {
		for _ in 0..=MAX_REDIRECT_TIMES {
			let payload;
			let mut request = if self.may_payload.is_some() {
				payload = self.may_payload.clone().unwrap();
				Request::post(
					from_utf8(&self.location).unwrap_or_default(),
					vec![&payload[..]],
				)
				.add_header("Content-Type", "application/json")
			} else {
				Request::get(from_utf8(&self.location).unwrap_or_default())
			};
			if let Some(cookie) = self.cookie.clone() {
				request = request.add_header("cookie", from_utf8(&cookie).unwrap_or_default());
			}
			if let Ok(pending) = request.send() {
				if let Ok(mut resp) = pending.wait() {
					if resp.code == 200 {
						let resp_body = resp.body().collect::<Vec<u8>>();
						return Some(resp_body);
					} else if resp.code == 301 || resp.code == 302 {
						self.redirect_times += 1;
						debug::trace!(target: "eoc-req", "[eth-offchain] redirect({}) header: {:?}", self.redirect_times, resp.headers());
						let headers = resp.headers();
						if let Some(cookie) = headers.find("set-cookie") {
							self.cookie = Some(cookie.as_bytes().to_vec());
						}
						if let Some(location) = headers.find("location") {
							self.location = location.as_bytes().to_vec();
							debug::trace!(target: "eoc-req", "[eth-offchain] redirect({}) location: {:?}", self.redirect_times, self.location);
						}
					} else {
						debug::info!(target: "eoc-req", "[eth-offchain] Status Code: {}", resp.code);
						debug::info!(target: "eoc-req", "[eth-offchain] Response: {:?}", resp.body().collect::<Vec<u8>>());
						return None;
					}
				}
			}
		}
		None
	}
}

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
		/// API Response - UNEXPECTED
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

		/// The offchain worker which will be called in a regular block schedule
		/// The relay_header is called when the block meet the schedule timing
		fn offchain_worker(block: T::BlockNumber) {
			let fetch_interval = T::FetchInterval::get().max(1.into());
			if (block % fetch_interval).is_zero() {
				let maybe_key = StorageValueRef::persistent(b"eapi").get::<ApiKey>().unwrap_or(None);
				if let Err(e) = Self::relay_header(maybe_key){
					debug::error!(target: "eoc-ow", "[eth-offchain] Error: {:?}", e);
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// The `relay_header` is try to get Ethereum blocks from ethereum network,
	/// and this will dependence on the ApiKey to fetch the blocks from differe Ethereum
	/// infrastructures. If the EthScan ApiKey is present, we will get the blocks from EthScan,
	/// else the blocks will be got from Cloudflare Ethereum Gateway
	fn relay_header(maybe_key: Option<ApiKey>) -> Result<(), DispatchError> {
		if !T::SubmitSignedTransaction::can_sign() {
			debug::info!(target: "eoc-ow", "[eth-offchain] Not initialize, please provide key for ``rlwk`");
			return Ok(());
		}

		let target_number = Self::get_target_number()?;

		let header = if let Some(key) = maybe_key {
			let block_info_url = Self::build_url(vec![
				ethscan_url::GET_BLOCK.to_vec(),
				base_n_bytes(target_number, 16),
				"&boolean=false&apikey=".as_bytes().to_vec(),
				key.to_vec(),
			])?;
			Self::fetch_header(block_info_url, None)?
		} else {
			let block_info_url = Self::build_url(vec![ethgateway_url::GET_BLOCK.to_vec()])?;
			let mut payload = r#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x"#
				.as_bytes()
				.to_vec();
			payload.append(&mut base_n_bytes(target_number, 16));
			payload.append(&mut r#"",false],"id":1}"#.as_bytes().to_vec());
			Self::fetch_header(block_info_url, Some(payload))?
		};

		Self::submit_header(header);
		Ok(())
	}

	/// Get the last relayed block number, and return the blocknumber of next one as target
	fn get_target_number() -> Result<u64, DispatchError> {
		let target_number = <EthRelay<T>>::header(<EthRelay<T>>::best_header_hash())
			.ok_or(<Error<T>>::BestHeaderNE)?
			.number
			.checked_add(1)
			.ok_or(<Error<T>>::BlockNumberOF)?;
		debug::trace!(target: "eoc-gtn", "[eth-offchain] Target Number: {}", target_number);

		Ok(target_number)
	}

	fn build_url(params: Vec<Vec<u8>>) -> Result<Vec<u8>, DispatchError> {
		let mut url = vec![];
		for mut param in params {
			url.append(&mut param);
		}
		debug::trace!(target: "eoc-bu", "[eth-offchain] Block Info Url: {}", from_utf8(&url).unwrap_or_default());
		Ok(url)
	}

	/// Build the response as EthHeader struct after validating
	fn fetch_header(
		url: Vec<u8>,
		may_payload: Option<Vec<u8>>,
	) -> Result<EthHeader, DispatchError> {
		let maybe_resp_body = OffchainRequest::new(url, may_payload).send();

		let resp_body = Self::validate_response(maybe_resp_body)?;
		let raw_header = from_utf8(&resp_body[33..resp_body.len() - 1]).unwrap_or_default();

		let header = EthHeader::from_str_unchecked(raw_header);
		debug::trace!(target: "eoc-fh", "[eth-offchain] Relay: {:?}", header);

		Ok(header)
	}

	/// Validate the response is a JSON with enough data not simple error message
	fn validate_response(maybe_resp_body: Option<Vec<u8>>) -> Result<Vec<u8>, DispatchError> {
		if let Some(resp_body) = maybe_resp_body {
			debug::trace!(
				target: "eoc-vr",
				"[eth-offchain] Response: {}",
				from_utf8(&resp_body).unwrap_or_default(),
			);
			if resp_body[0] != 123u8 || resp_body.len() < 1362 {
				debug::info!(target: "eoc-vr", "[eth-offchain] malresponse:");
				Err(<Error<T>>::APIRespUnexp)?;
			}
			Ok(resp_body)
		} else {
			debug::info!(target: "eoc-vr", "[eth-offchain] lack response");
			Err(<Error<T>>::APIRespUnexp)?
		}
	}

	/// Submit and record the valid header on Darwinia network
	fn submit_header(header: EthHeader) {
		// FIXME: add proof
		let results = T::SubmitSignedTransaction::submit_signed(<EthRelayCall<T>>::relay_header(
			header,
			vec![],
		));
		for (account, result) in &results {
			debug::trace!(
				target: "eoc-rl",
				"[eth-offchain] Account: {:?}, Relay: {:?}",
				account,
				result,
			);
		}
	}
}

/// Transfer digitals into hex form
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
