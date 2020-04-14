#![cfg_attr(not(feature = "std"), no_std)]

pub mod crypto {
	// --- third-party ---
	use sp_runtime::app_crypto::{app_crypto, sr25519};

	// --- custom ---
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
						debug::trace!(target: "eoc-req", "[eth-offchain] redirect header: {:?}", resp.headers());
						let headers = resp.headers();
						if let Some(cookie) = headers.find("set-cookie") {
							self.cookie = Some(cookie.as_bytes().to_vec());
						}
						if let Some(location) = headers.find("location") {
							self.location = location.as_bytes().to_vec();
							debug::trace!(target: "eoc-req", "[eth-offchain] redirect location: {:?}", self.location);
						}
					} else {
						debug::trace!(target: "eoc-req", "[eth-offchain] Status Code: {}", resp.code);
						debug::trace!(target: "eoc-req", "[eth-offchain] Response: {:?}", resp.body().collect::<Vec<u8>>());
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
	fn relay_header(maybe_key: Option<ApiKey>) -> Result<(), DispatchError> {
		if !T::SubmitSignedTransaction::can_sign() {
			Err(<Error<T>>::AccountUnavail)?;
		}

		let target_number = Self::get_target_number()?;

		let header = if let Some(key) = maybe_key {
			let block_info_url = Self::build_url(vec![
				ethscan_url::GET_BLOCK.to_vec(),
				base_n_bytes(target_number, 16),
				"&boolean=true&apikey=".as_bytes().to_vec(),
				key.to_vec(),
			])?;
			Self::fetch_header(block_info_url, None)?
		} else {
			let block_info_url = Self::build_url(vec![ethgateway_url::GET_BLOCK.to_vec()])?;
			let mut payload = r#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x"#
				.as_bytes()
				.to_vec();
			payload.append(&mut base_n_bytes(target_number, 16));
			payload.append(&mut r#"",true],"id":1}"#.as_bytes().to_vec());
			Self::fetch_header(block_info_url, Some(payload))?
		};

		Self::submit_header(header);
		Ok(())
	}

	fn get_target_number() -> Result<u64, DispatchError> {
		let target_number = <EthRelay<T>>::header_of(<EthRelay<T>>::best_header_hash())
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

	fn validate_response(maybe_resp_body: Option<Vec<u8>>) -> Result<Vec<u8>, DispatchError> {
		if let Some(resp_body) = maybe_resp_body {
			debug::trace!(
				target: "eoc-vr",
				"[eth-offchain] Response: {}",
				from_utf8(&resp_body).unwrap_or_default(),
			);
			if resp_body[0] != 123u8 || resp_body.len() < 1362 {
				Err(<Error<T>>::APIRespUnexp)?;
			}
			Ok(resp_body)
		} else {
			Err(<Error<T>>::APIRespUnexp)?
		}
	}

	fn submit_header(header: EthHeader) {
		let results =
			T::SubmitSignedTransaction::submit_signed(<EthRelayCall<T>>::relay_header(header));
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
