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
//! https://github.com/darwinia-network/darwinia-common/issues/86
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

// --- core ---
use codec::Decode;
use core::str::from_utf8;
// --- substrate ---
use frame_support::{debug::trace, decl_error, decl_event, decl_module, traits::Get};
use frame_system::{self as system, offchain::SubmitSignedTransaction};
use sp_runtime::{offchain::http::Request, traits::Zero, DispatchError, KeyTypeId};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::base_n_bytes_unchecked;
use eth_primitives::header::EthHeader;

type EthRelay<T> = darwinia_eth_relay::Module<T>;
type EthRelayCall<T> = darwinia_eth_relay::Call<T>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"rlwk");

const MAX_REDIRECT_TIMES: u8 = 3;

#[cfg(feature = "easy-testing")]
const ETHRESOURCE: &'static [u8] = b"https://cloudflare-eth.com";
#[cfg(not(feature = "easy-testing"))]
const ETHRESOURCE: &'static [u8] = b"http://eth-resource";

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
						return Some(resp.body().collect::<Vec<_>>());
					} else if resp.code == 301 || resp.code == 302 {
						self.redirect_times += 1;
						trace!(
							target: "eoc-req",
							"[eth-offchain] Redirect({}), Request Header: {:?}",
							self.redirect_times, resp.headers(),
						);

						let headers = resp.headers();
						if let Some(cookie) = headers.find("set-cookie") {
							self.cookie = Some(cookie.as_bytes().to_vec());
						}
						if let Some(location) = headers.find("location") {
							self.location = location.as_bytes().to_vec();
							trace!(
								target: "eoc-req",
								"[eth-offchain] Redirect({}), Location: {:?}",
								self.redirect_times,
								self.location,
							);
						}
					} else {
						trace!(target: "eoc-req", "[eth-offchain] Status Code: {}", resp.code);
						trace!(
							target: "eoc-req",
							"[eth-offchain] Response: {}",
							from_utf8(&resp.body().collect::<Vec<_>>()).unwrap_or_default(),
						);

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
					trace!(target: "eoc-wk", "[eth-offchain] Error: {:?}", e);
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
		let header_without_option = Self::fetch_header(ETHRESOURCE.to_vec(), target_number, false);
		let header = match header_without_option {
			Ok(h) => h,
			Err(e) => {
				trace!(target: "eoc-rh", "[eth-offchain] request without option fail: {:?}", e);
				trace!(target: "eoc-rh", "[eth-offchain] request fail back wth option");
				Self::fetch_header(ETHRESOURCE.to_vec(), target_number, true)?
			}
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
		trace!(target: "eoc-rl", "[eth-offchain] Target Number: {}", target_number);

		Ok(target_number)
	}

	/// Build the response as EthHeader struct after validating
	fn fetch_header(
		url: Vec<u8>,
		target_number: u64,
		option: bool,
	) -> Result<EthHeader, DispatchError> {
		let mut payload =
			r#"{"jsonrpc":"2.0","method":"shadow_getEthHeaderWithProofByNumber","params":{"block_num":"#
				.as_bytes()
				.to_vec();
		payload.append(&mut base_n_bytes_unchecked(target_number, 10));
		payload.append(&mut r#","transcation":false"#.as_bytes().to_vec());
		if option {
			payload.append(&mut r#","options":{"format":"json"}"#.as_bytes().to_vec());
		}
		payload.append(&mut r#"},"id":1}"#.as_bytes().to_vec());
		let maybe_resp_body = OffchainRequest::new(url, payload).send();

		let resp_body = Self::validate_response(maybe_resp_body, option)?;
		let header = if option {
			let raw_header = from_utf8(&resp_body[47..resp_body.len() - 1]).unwrap_or_default();
			EthHeader::from_str_unchecked(raw_header)
		} else {
			let scale_bytes: Vec<u8> = (50..1353)
				.step_by(2)
				.map(|i| {
					u8::from_str_radix(from_utf8(&resp_body[i..i + 2]).unwrap_or_default(), 16)
						.unwrap_or_default()
				})
				.collect();
			let may_decoded_header: Option<EthHeader> =
				Decode::decode::<&[u8]>(&mut &scale_bytes[..]).ok();
			may_decoded_header.unwrap_or_default()
		};
		trace!(target: "eoc-rl", "[eth-offchain] Eth Header: {:?}", header);

		Ok(header)
	}

	/// Validate the response is a JSON with enough data not simple error message
	fn validate_response(
		maybe_resp_body: Option<Vec<u8>>,
		with_option: bool,
	) -> Result<Vec<u8>, DispatchError> {
		if let Some(resp_body) = maybe_resp_body {
			trace!(
				target: "eoc-rl",
				"[eth-offchain] Response: {}",
				from_utf8(&resp_body).unwrap_or_default(),
			);
			if resp_body[0] != 123u8
				|| (with_option && resp_body.len() < 1362)
				|| (!with_option && resp_body.len() < 1353)
			{
				trace!(target: "eoc-rl", "[eth-offchain] Malresponse");
				Err(<Error<T>>::APIRespUnexp)?;
			}
			Ok(resp_body)
		} else {
			trace!(target: "eoc-rl", "[eth-offchain] Lack Response");
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
			trace!(
				target: "eoc-rl",
				"[eth-offchain] Account: {:?}, Relay: {:?}",
				account,
				result,
			);
		}
	}
}
