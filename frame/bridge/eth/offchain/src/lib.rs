//! Module to relay blocks from Ethereum Network
//!
//! In this module,
//! the offchain worker will keep fetch the next block info and relay to Darwinia Network.
//! The worker will fetch the header and the Merkle proof information for blocks from a nonexistent domain,
//! ie http://eth-resource/, such that it can be connected with shadow service.
//! Now the shadow service is provided by our anothre project, darwinia.js.
//! https://github.com/darwinia-network/darwinia.js
//!
//!
//! Here is the basic flow.
//! The starting point is the `offchain_worker`
//! - base on block schedule, the `relay_header` will be called
//! - then the `relay_header` will get ethereum blocks and Merkle proof information from from http://eth-resource/
//! - After the http response corrected fetched, we will simple validate the format of http response,
//!   and parse and build Ethereum header and Merkle Proofs.
//! - After all, the corrected Ethereum header with the proofs will be submit and validate on chain of Darwinia Network by
//!   `submit_header`
//!
//! The protocol of shadow service and offchain worker can be scale encoded format or json format,
//! and the worker will use json format as fail back, such that it may be easiler to debug.
//! If you want to build your own shadow service please refer
//! https://github.com/darwinia-network/darwinia-common/issues/86
//!
//! More details about offchain workers in following PRs
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
use frame_support::{
	debug::error, debug::info, debug::trace, decl_error, decl_event, decl_module, traits::Get,
};
use frame_system::{self as system, offchain::SubmitSignedTransaction};
use sp_runtime::{offchain::http::Request, traits::Zero, DispatchError, KeyTypeId};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_eth_relay::DoubleNodeWithMerkleProof;
use darwinia_support::bytes_thing::{base_n_bytes_unchecked, hex_bytes_unchecked};
use eth_primitives::header::EthHeader;

type EthRelay<T> = darwinia_eth_relay::Module<T>;
type EthRelayCall<T> = darwinia_eth_relay::Call<T>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"rlwk");

const MAX_REDIRECT_TIMES: u8 = 3;

// A dummy endpoint, point this to shadow service
const ETHRESOURCE: &'static [u8] = b"http://eth-resource";

#[derive(Default)]
pub struct OffchainRequest {
	location: Vec<u8>,
	payload: Vec<u8>,
	redirect_times: u8,
	cookie: Option<Vec<u8>>,
}

pub trait OffchainRequestTrait {
	fn send(&mut self) -> Option<Vec<u8>>;
}

impl OffchainRequest {
	fn new(url: Vec<u8>, payload: Vec<u8>) -> Self {
		OffchainRequest {
			location: url.clone(),
			payload,
			..Default::default()
		}
	}
}

/// The OffchainRequest handle the request session
/// - set cookie if returns
/// - handle the redirect actions if happends
#[cfg(not(test))]
impl OffchainRequestTrait for OffchainRequest {
	fn send(&mut self) -> Option<Vec<u8>> {
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
		/// API Response - UNEXPECTED
		APIRespUnexp,

		/// Best Header - NOT EXISTED
		BestHeaderNE,
		/// Block Number - OVERFLOW
		BlockNumberOF,

		/// Proof - SCALE DECODE ERROR
		ProofSE,
		/// Proof - JSON DECODE ERROR
		ProofJE,
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
				if T::SubmitSignedTransaction::can_sign() {
					if let Err(e) = Self::relay_header(){
						error!(target: "eoc-wk", "[eth-offchain] Error: {:?}", e);
					}
				} else {
					trace!(target: "eoc-wk", "[eth-offchain] use `author_insertKey` rpc to inscert `rlwk` key to enable worker");
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// The `relay_header` is try to get Ethereum blocks with merkle proofs from shadow service
	/// The default communication will transfer data with scale encoding,
	/// if there are issue to communicate with scale encoding, the failback communication will
	/// be performed with json format(use option: `true`)
	fn relay_header() -> Result<(), DispatchError> {
		let target_number = Self::get_target_number()?;
		let header_without_option = Self::fetch_header(ETHRESOURCE.to_vec(), target_number, false);
		let (header, proof_list) = match header_without_option {
			Ok(r) => r,
			Err(e) => {
				info!(target: "eoc-rh", "[eth-offchain] request without option fail: {:?}", e);
				info!(target: "eoc-rh", "[eth-offchain] request fail back wth option");
				Self::fetch_header(ETHRESOURCE.to_vec(), target_number, true)?
			}
		};

		// The `submit_header` will call event out of eth-offchain pallet,
		// so this function is skiped in the test of pallet
		#[cfg(not(test))]
		Self::submit_header(header, proof_list);
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

	fn build_request_client(url: Vec<u8>, payload: Vec<u8>) -> impl OffchainRequestTrait {
		OffchainRequest::new(url, payload)
	}

	/// Build the response as EthHeader struct after validating
	fn fetch_header(
		url: Vec<u8>,
		target_number: u64,
		option: bool,
	) -> Result<(EthHeader, Vec<DoubleNodeWithMerkleProof>), DispatchError> {
		let payload = Self::build_payload(target_number, option);
		let mut client = Self::build_request_client(url, payload);
		let maybe_resp_body = client.send();

		let resp_body = Self::validate_response(maybe_resp_body, option)?;

		let eth_header_part = Self::extract_from_json_str(&resp_body[..], b"eth_header" as
			&[u8]).unwrap_or_default();
		let header = if option {
			EthHeader::from_str_unchecked(from_utf8(eth_header_part).unwrap_or_default())
		} else {
			let scale_bytes = hex_bytes_unchecked(from_utf8(eth_header_part).unwrap_or_default());
			Decode::decode::<&[u8]>(&mut &scale_bytes[..]).unwrap_or_default()
		};

		let proof_part = Self::extract_from_json_str(&resp_body[..], b"proof" as
			&[u8]).unwrap_or_default();
		let proof_list = if option {
			Self::parse_double_node_with_proof_list_from_json_str(proof_part)?
		} else {
			Self::parse_double_node_with_proof_list_from_scale_str(proof_part)?
		};
		trace!(target: "eoc-rl", "[eth-offchain] Eth Header: {:?}", header);

		Ok((header, proof_list))
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
	fn submit_header(header: EthHeader, proof_list: Vec<DoubleNodeWithMerkleProof>) {
		let results = T::SubmitSignedTransaction::submit_signed(<EthRelayCall<T>>::relay_header(
			header, proof_list,
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

	/// Build a payload to request the json response or scaled encoded response depence on option
	fn build_payload(target_number: u64, option: bool) -> Vec<u8> {
		let header_part: &[u8] = br#"{"jsonrpc":"2.0","method":"shadow_getEthHeaderWithProofByNumber","params":{"block_num":"#;
		let number_part: &[u8] = &base_n_bytes_unchecked(target_number, 10)[..];
		let transcation_part: &[u8] = br#","transcation":false"#;
		let option_part: &[u8] = br#","options":{"format":"json"}"#;
		let tail: &[u8] = br#"},"id":1}"#;

		if option {
			[
				header_part,
				number_part,
				transcation_part,
				option_part,
				tail,
			]
			.concat()
		} else {
			[header_part, number_part, transcation_part, tail].concat()
		}
	}

	/// Build the merkle proof information from json format response
	fn parse_double_node_with_proof_list_from_json_str(
		json_str: &[u8],
	) -> Result<Vec<DoubleNodeWithMerkleProof>, DispatchError> {
		let raw_str = match from_utf8(json_str) {
			Ok(r) => r,
			Err(_) => return Err(<Error<T>>::ProofJE)?,
		};

		let mut proof_list: Vec<DoubleNodeWithMerkleProof> = Vec::new();
		// The proof list is 64 length, and we use 256 just in case.
		for p in raw_str.splitn(256, '}') {
			if p.len() > 0 {
				proof_list.push(DoubleNodeWithMerkleProof::from_str_unchecked(p));
			}
		}
		Ok(proof_list)
	}

	/// Build the merkle proof information from scale encoded response
	fn parse_double_node_with_proof_list_from_scale_str(
		scale_str: &[u8],
	) -> Result<Vec<DoubleNodeWithMerkleProof>, DispatchError> {
		if scale_str.len() < 2 {
			return Err(<Error<T>>::ProofSE)?;
		};
		let proof_scale_bytes = hex_bytes_unchecked(from_utf8(scale_str).unwrap_or_default());
		Ok(Decode::decode::<&[u8]>(&mut &proof_scale_bytes[..]).unwrap_or_default())
	}

	/// Extract the inner value from json str with specific field
	fn extract_from_json_str<'a>(json_str: &'a[u8], field_name: &'static [u8]) -> Option<&'a[u8]> {
		let mut start=0;
		let mut open_part_count = 0;
		let mut open_part = b'\0';
		let mut close_part = b'\0';
		let field_length = field_name.len();
		let mut match_pos = 0;
		let mut has_colon = false;
		for i in 0..json_str.len() {
			if open_part_count > 0 {
				if json_str[i] == close_part {
					open_part_count -= 1;
					if 0 == open_part_count {
							return Some(&json_str[start+1..i])
					}
				} else if json_str[i] == open_part {
					open_part_count += 1;
				}
			} else if has_colon {
				if json_str[i] == b'"' || json_str[i] == b'[' || json_str[i] == b'{' {
					start = i;
					open_part_count += 1;
					open_part =json_str[i];
					close_part = match json_str[i] {
						b'"' => b'"',
						b'[' => b']',
						b'{' => b'}',
						_ => panic!("never here")
					}
				}
			} else if match_pos > 0 && i > match_pos {
				if json_str[i] == b':' {
					has_colon = true;
				}
			} else if json_str[i] == field_name[0] && (json_str.len() - i) >= field_length && json_str[i..i + field_length]  == *field_name {
				match_pos = i + field_length;
			}
		}
		None
	}
}
