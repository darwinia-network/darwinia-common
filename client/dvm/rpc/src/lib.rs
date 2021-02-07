// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Frontier.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate. If not, see <http://www.gnu.org/licenses/>.

mod eth;
mod eth_pubsub;

pub use eth::{EthApi, EthApiServer, NetApi, NetApiServer, Web3Api, Web3ApiServer};
pub use eth_pubsub::{EthPubSubApi, EthPubSubApiServer, HexEncodedIdProvider};
use ethereum_types::H160;

use darwinia_evm::ExitReason;
use jsonrpc_core::{Error, ErrorCode, Value};
use rustc_hex::ToHex;

pub fn internal_err<T: ToString>(message: T) -> Error {
	Error {
		code: ErrorCode::InternalError,
		message: message.to_string(),
		data: None,
	}
}
pub fn error_on_execution_failure(reason: &ExitReason, data: &[u8]) -> Result<(), Error> {
	match reason {
		ExitReason::Succeed(_) => Ok(()),
		ExitReason::Error(e) => Err(Error {
			code: ErrorCode::InternalError,
			message: format!("evm error: {:?}", e),
			data: Some(Value::String("0x".to_string())),
		}),
		ExitReason::Revert(_) => {
			let mut message = "VM Exception while processing transaction: revert".to_string();
			// A minimum size of error function selector (4) + offset (32) + string length (32)
			// should contain a utf-8 encoded revert reason.
			if data.len() > 68 {
				let message_len = data[36..68].iter().sum::<u8>();
				let body: &[u8] = &data[68..68 + message_len as usize];
				if let Ok(reason) = std::str::from_utf8(body) {
					message = format!("{} {}", message, reason.to_string());
				}
			}

			Err(Error {
				code: ErrorCode::InternalError,
				message,
				data: Some(Value::String(data.to_hex())),
			})
		}
		ExitReason::Fatal(e) => Err(Error {
			code: ErrorCode::InternalError,
			message: format!("evm fatal: {:?}", e),
			data: Some(Value::String("0x".to_string())),
		}),
	}
}

/// A generic Ethereum signer.
pub trait EthSigner: Send + Sync {
	/// Available accounts from this signer.
	fn accounts(&self) -> Vec<H160>;
	/// Sign a transaction message using the given account in message.
	fn sign(&self, message: ethereum::TransactionMessage) -> Result<ethereum::Transaction, Error>;
}
