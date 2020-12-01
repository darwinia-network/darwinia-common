// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Frontier.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

mod eth;
mod eth_pubsub;

use ethereum_types::H160;
pub use eth::{EthApi, EthApiServer, NetApi, NetApiServer};
pub use eth_pubsub::{EthPubSubApi, EthPubSubApiServer};

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
		ExitReason::Revert(e) => Err(Error {
			code: ErrorCode::InternalError,
			message: format!("evm revert: {:?}", e),
			data: Some(Value::String(data.to_hex())),
		}),
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