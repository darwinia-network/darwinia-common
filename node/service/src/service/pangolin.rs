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

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

// --- std ---
use std::sync::Arc;
// --- paritytech ---
use sc_executor::NativeExecutionDispatch;
use sc_service::{Configuration, Error as ServiceError, RpcHandlers, TaskManager};
// --- darwinia-network ---
use crate::{
	client::DrmlClient,
	service::{self, FullBackend},
};
use drml_common_primitives::OpaqueBlock as Block;
use drml_rpc::RpcConfig;
use pangolin_runtime::RuntimeApi;

pub struct Executor;
impl NativeExecutionDispatch for Executor {
	type ExtendHostFunctions = (
		frame_benchmarking::benchmarking::HostFunctions,
		dp_evm_trace_ext::dvm_ext::HostFunctions,
	);

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		pangolin_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		pangolin_runtime::native_version()
	}
}

/// Create a new Pangolin service for a full node.
#[cfg(feature = "full-node")]
pub fn new_full(
	config: Configuration,
	authority_discovery_disabled: bool,
	rpc_config: RpcConfig,
) -> Result<
	(
		TaskManager,
		Arc<impl DrmlClient<Block, FullBackend, RuntimeApi>>,
		RpcHandlers,
	),
	ServiceError,
> {
	let (components, client, rpc_handlers) = service::new_full::<RuntimeApi, Executor>(
		config,
		authority_discovery_disabled,
		rpc_config,
	)?;

	Ok((components, client, rpc_handlers))
}

/// Create a new Pangolin service for a light client.
pub fn new_light(config: Configuration) -> Result<(TaskManager, RpcHandlers), ServiceError> {
	service::new_light::<RuntimeApi, Executor>(config)
}
