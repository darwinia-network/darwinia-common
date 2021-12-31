// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Net rpc interface.
use fc_rpc_core::types::PeerCount;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

pub use rpc_impl_NetApi::gen_server::NetApi as NetApiServer;

/// Net rpc interface.
#[rpc(server)]
pub trait NetApi {
	/// Returns protocol version.
	#[rpc(name = "net_version")]
	fn version(&self) -> Result<String>;

	/// Returns number of peers connected to node.
	#[rpc(name = "net_peerCount")]
	fn peer_count(&self) -> Result<PeerCount>;

	/// Returns true if client is actively listening for network connections.
	/// Otherwise false.
	#[rpc(name = "net_listening")]
	fn is_listening(&self) -> Result<bool>;
}
