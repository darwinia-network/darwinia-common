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

// --- std ---
use std::{env, path::PathBuf};
// --- paritytech ---
use sc_cli::{Error as CliError, Result as CliResult, Role, RuntimeVersion, SubstrateCli};
use sc_service::{ChainSpec, DatabaseSource};
#[cfg(feature = "try-runtime")]
use sc_service::{Error as ServiceError, TaskManager};
use sp_core::crypto::{self, Ss58AddressFormat};
// --- darwinia-network ---
use crate::cli::*;
#[cfg(any(feature = "try-runtime", feature = "runtime-benchmarks"))]
use drml_primitives::OpaqueBlock as Block;
use drml_service::*;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Darwinia Runtime Module Library".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn executable_name() -> String {
		"drml".into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/darwinia-network/darwinia-common/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2018
	}

	fn native_runtime_version(spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		#[cfg(feature = "template")]
		if spec.is_template() {
			return &template_runtime::VERSION;
		}

		if spec.is_pangolin() {
			&pangolin_runtime::VERSION
		} else {
			&pangoro_runtime::VERSION
		}
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["template", "pangolin", "pangoro"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("pangoro")
		} else {
			id
		};

		Ok(match id.to_lowercase().as_ref() {
			"pangolin" => Box::new(pangolin_chain_spec::config()?),
			"pangolin-genesis" => Box::new(pangolin_chain_spec::genesis_config()),
			"pangolin-dev" | "dev" => Box::new(pangolin_chain_spec::development_config()),
			"pangolin-local" | "local" => Box::new(pangolin_chain_spec::local_testnet_config()),
			"pangoro" => Box::new(pangoro_chain_spec::config()?),
			"pangoro-genesis" => Box::new(pangoro_chain_spec::genesis_config()),
			"pangoro-dev" => Box::new(pangoro_chain_spec::development_config()),
			#[cfg(feature = "template")]
			"template" | "template-dev" => Box::new(template_chain_spec::development_config()),
			_ => {
				let path = PathBuf::from(id);
				let chain_spec =
					Box::new(PangoroChainSpec::from_json_file(path.clone())?) as Box<dyn ChainSpec>;

				if self.run.force_pangolin || chain_spec.is_pangolin() {
					Box::new(PangolinChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			}
		})
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> CliResult<()> {
	macro_rules! async_run {
		(|$cmd:ident, $cli:ident, $config:ident, $client:ident, $backend:ident, $import_queue:ident| $($code:tt)*) => {{
			let runner = $cli.create_runner($cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangolin() {
				runner.async_run(|mut $config| {
					let ($client, $backend, $import_queue, task_manager) = drml_service::new_chain_ops::<
						PangoroRuntimeApi,
						PangolinExecutor,
					>(&mut $config)?;

					{ $( $code )* }.map(|v| (v, task_manager))
				})
			} else {
				runner.async_run(|mut $config| {
					let ($client, $backend, $import_queue, task_manager) = drml_service::new_chain_ops::<
						PangolinRuntimeApi,
						PangoroExecutor,
					>(&mut $config)?;

					{ $( $code )* }.map(|v| (v, task_manager))
				})
			}
		}};
	}

	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			validate_trace_environment(&cli)?;

			let runner = cli.create_runner(&cli.run.base)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			let eth_rpc_config = cli.run.dvm_args.build_eth_rpc_config();

			#[cfg(feature = "template")]
			if chain_spec.is_template() {
				let is_manual_sealing = cli.run.dvm_args.sealing.is_manual();
				let enable_dev_signer = cli.run.dvm_args.enable_dev_signer;

				return runner
					.run_node_until_exit(|config| async move {
						template_service::new_full(
							config,
							is_manual_sealing,
							enable_dev_signer,
							eth_rpc_config,
						)
					})
					.map_err(CliError::from);
			}

			let authority_discovery_disabled = cli.run.authority_discovery_disabled;

			if chain_spec.is_pangolin() {
				runner.run_node_until_exit(|config| async move {
					match config.role {
						Role::Light => panic!("Not support light client"),
						_ => pangolin_service::new_full(
							config,
							authority_discovery_disabled,
							eth_rpc_config,
						)
						.map(|(task_manager, _, _)| task_manager),
					}
					.map_err(CliError::from)
				})
			} else {
				runner.run_node_until_exit(|config| async move {
					match config.role {
						Role::Light => panic!("Not support light client"),
						_ => pangoro_service::new_full(
							config,
							authority_discovery_disabled,
							eth_rpc_config,
						)
						.map(|(task_manager, _, _)| task_manager),
					}
					.map_err(CliError::from)
				})
			}
		}
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			async_run!(|cmd, cli, config, client, _backend, import_queue| Ok(
				cmd.run(client, import_queue)
			))
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			async_run!(|cmd, cli, config, client, _backend, _import_queue| Ok(
				cmd.run(client, config.database)
			))
		}
		Some(Subcommand::ExportState(cmd)) => {
			async_run!(|cmd, cli, config, client, _backend, _import_queue| Ok(
				cmd.run(client, config.chain_spec)
			))
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			async_run!(|cmd, cli, config, client, _backend, import_queue| Ok(
				cmd.run(client, import_queue)
			))
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.sync_run(|config| {
				// Remove dvm offchain db
				let dvm_database_config = DatabaseSource::RocksDb {
					path: drml_service::dvm::db_path(&config),
					cache_size: 0,
				};

				cmd.run(dvm_database_config)?;
				cmd.run(config.database)
			})
		}
		Some(Subcommand::Revert(cmd)) => {
			async_run!(|cmd, cli, config, client, backend, _import_queue| Ok(
				cmd.run(client, backend)
			))
		}
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangolin() {
				runner.async_run(|config| {
					// we don't need any of the components of new_partial, just a runtime, or a task
					// manager to do `async_run`.
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager = TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| CliError::from(ServiceError::Prometheus(e)))?;

					Ok((cmd.run::<Block, PangolinExecutor>(config), task_manager))
				})
			} else {
				runner.async_run(|config| {
					// we don't need any of the components of new_partial, just a runtime, or a task
					// manager to do `async_run`.
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager = TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| CliError::from(ServiceError::Prometheus(e)))?;

					Ok((cmd.run::<Block, PangoroExecutor>(config), task_manager))
				})
			}
		}
		#[cfg(feature = "runtime-benchmarks")]
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangolin() {
				runner.sync_run(|config| cmd.run::<Block, PangolinExecutor>(config))
			} else {
				runner.sync_run(|config| cmd.run::<Block, PangoroExecutor>(config))
			}
		}
	}
}

fn get_exec_name() -> Option<String> {
	env::current_exe().ok()?.file_name().map(|name| name.to_string_lossy().into_owned())
}

fn set_default_ss58_version(spec: &Box<dyn ChainSpec>) {
	let ss58_version = if spec.is_pangoro() {
		Ss58AddressFormat::DarwiniaAccount
	} else {
		Ss58AddressFormat::SubstrateAccount
	};

	crypto::set_default_ss58_version(ss58_version);
}

fn validate_trace_environment(cli: &Cli) -> CliResult<()> {
	if cli
		.run
		.dvm_args
		.ethapi_debug_targets
		.iter()
		.any(|target| matches!(target.as_str(), "debug" | "trace"))
		&& cli.run.base.import_params.wasm_runtime_overrides.is_none()
	{
		Err("`debug` or `trace` namespaces requires `--wasm-runtime-overrides /path/to/overrides`."
			.into())
	} else {
		Ok(())
	}
}
