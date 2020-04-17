//! Module to relay blocks from Ethereum Network
//!
//! In this module,
//! the offchain worker will keep fetch the next block info and relay to Darwinia Network.
//! The worker will fetch blocks from a nonexistent domain, ie http://eth-resource/,
//! such that it can be proxy to any source and do any reprocessing or cache on the node.
//! Now the source may be EtherScan, Cloudflare Ethereum Gateway, or a Ethereum full node.
//! Please our anothre project, darwinia.js.
//! https://github.com/darwinia-network/darwinia.js
//!
//! Here is the basic flow.
//! The starting point is the `offchain_worker`
//! - base on block schedule, the `relay_header` will be called
//! - then the `relay_header` will get ethereum blocks from from http://eth-resource/
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

#[cfg(all(feature = "std", test))]
mod mock;
#[cfg(all(feature = "std", test))]
mod tests;
#[cfg(feature = "easy-testing")]
static ETHRESOURCE: &'static [u8] = b"https://cloudflare-eth.com/";
#[cfg(not(feature = "easy-testing"))]
static ETHRESOURCE: &'static [u8] = b"http://eth-resource";

// --- core ---
use core::str::from_utf8;
// --- substrate ---
use frame_support::{debug, decl_error, decl_event, decl_module, traits::Get};
use frame_system::{self as system, offchain::SubmitSignedTransaction};
use sp_runtime::{offchain::http::Request, traits::Zero, DispatchError, KeyTypeId};
use sp_std::prelude::*;
// --- darwinia ---
use eth_primitives::header::EthHeader;

type EthRelay<T> = darwinia_eth_relay::Module<T>;
type EthRelayCall<T> = darwinia_eth_relay::Call<T>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"rlwk");

const MAX_REDIRECT_TIMES: u8 = 3;

#[derive(Default)]
struct OffchainRequest {
	location: Vec<u8>,
	payload: Vec<u8>,
	redirect_times: u8,
	cookie: Option<Vec<u8>>,
}

/// The OffhcainRequest handle the request session
/// - set cookie if returns
/// - handle the redirect actions if happends
impl OffchainRequest {
	pub fn new(url: Vec<u8>, payload: Vec<u8>) -> Self {
		OffchainRequest {
			location: url.clone(),
			payload,
			..Default::default()
		}
	}

	pub fn send(mut self) -> Option<Vec<u8>> {
		for _ in 0..=MAX_REDIRECT_TIMES {
			let p = self.payload.clone();
			let request =
				Request::post(from_utf8(&self.location).unwrap_or_default(), vec![&p[..]])
					.add_header("Content-Type", "application/json");
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
		/// Local accounts - UNAVAILABLE (Consider adding one via `author_insertKey` RPC)
		AccountUnavail,

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
				if let Err(e) = Self::relay_header(){
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
	fn relay_header() -> Result<(), DispatchError> {
		if !T::SubmitSignedTransaction::can_sign() {
			Err(<Error<T>>::AccountUnavail)?;
		}

		let target_number = Self::get_target_number()?;

		let mut payload = r#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x"#
			.as_bytes()
			.to_vec();
		payload.append(&mut base_n_bytes(target_number, 16));
		payload.append(&mut r#"",false],"id":1}"#.as_bytes().to_vec());
		let header = Self::fetch_header(ETHRESOURCE.to_vec(), payload)?;

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

	/// Build the response as EthHeader struct after validating
	fn fetch_header(url: Vec<u8>, payload: Vec<u8>) -> Result<EthHeader, DispatchError> {
		let maybe_resp_body = OffchainRequest::new(url, payload).send();

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
