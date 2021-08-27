// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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
use sc_cli::{Role, RuntimeVersion, SubstrateCli};
use sc_service::ChainSpec;
#[cfg(feature = "try-runtime")]
use sc_service::TaskManager;
use sp_core::crypto::Ss58AddressFormat;
// --- darwinia-network ---
use crate::cli::{Cli, Subcommand};
use drml_service::{
	chain_spec::{
		pangolin::{self as pangolin_chain_spec},
		pangoro::{self as pangoro_chain_spec},
		PangolinChainSpec, PangoroChainSpec,
	},
	service::{
		pangolin as pangolin_service, pangolin_runtime, pangoro as pangoro_service,
		pangoro_runtime, IdentifyVariant, PangolinExecutor, PangoroExecutor,
	},
};

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
		if spec.is_pangoro() {
			&pangoro_runtime::VERSION
		} else {
			&pangolin_runtime::VERSION
		}
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["pangolin", "pangoro"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("pangolin")
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
			path => {
				let path = PathBuf::from(path);
				let chain_spec = Box::new(PangolinChainSpec::from_json_file(path.clone())?)
					as Box<dyn ChainSpec>;

				if self.run.force_pangoro || chain_spec.is_pangoro() {
					Box::new(PangoroChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			}
		})
	}
}

fn get_exec_name() -> Option<String> {
	env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

fn set_default_ss58_version(spec: &Box<dyn ChainSpec>) {
	let ss58_version = if spec.is_pangoro() {
		Ss58AddressFormat::SubstrateAccount
	} else {
		Ss58AddressFormat::DarwiniaAccount
	};

	sp_core::crypto::set_default_ss58_version(ss58_version);
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();
	let max_past_logs = cli.run.dvm_args.max_past_logs;
	let target_gas_price = cli.run.dvm_args.target_gas_price;

	match &cli.subcommand {
		None => {
			let authority_discovery_disabled = cli.run.authority_discovery_disabled;
			let runner = cli.create_runner(&cli.run.base)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.run_node_until_exit(|config| async move {
					match config.role {
						Role::Light => pangoro_service::pangoro_new_light(config)
							.map(|(task_manager, _)| task_manager),
						_ => {
							pangoro_service::pangoro_new_full(config, authority_discovery_disabled)
								.map(|(task_manager, _, _)| task_manager)
						}
					}
					.map_err(sc_cli::Error::Service)
				})
			} else {
				runner.run_node_until_exit(|config| async move {
					match config.role {
						Role::Light => pangolin_service::pangolin_new_light(config)
							.map(|(task_manager, _)| task_manager),
						_ => pangolin_service::pangolin_new_full(
							config,
							authority_discovery_disabled,
							max_past_logs,
							target_gas_price,
						)
						.map(|(task_manager, _, _)| task_manager),
					}
					.map_err(sc_cli::Error::Service)
				})
			}
		}
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) = pangoro_service::new_chain_ops::<
						pangoro_runtime::RuntimeApi,
						PangoroExecutor,
					>(&mut config)?;

					Ok((cmd.run(client, import_queue), task_manager))
				})
			} else {
				runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) =
						pangolin_service::new_chain_ops::<
							pangolin_runtime::RuntimeApi,
							PangolinExecutor,
						>(&mut config, max_past_logs, target_gas_price)?;

					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.async_run(|mut config| {
					let (client, _, _, task_manager) = pangoro_service::new_chain_ops::<
						pangoro_runtime::RuntimeApi,
						PangoroExecutor,
					>(&mut config)?;

					Ok((cmd.run(client, config.database), task_manager))
				})
			} else {
				runner.async_run(|mut config| {
					let (client, _, _, task_manager) =
						pangolin_service::new_chain_ops::<
							pangolin_runtime::RuntimeApi,
							PangolinExecutor,
						>(&mut config, max_past_logs, target_gas_price)?;

					Ok((cmd.run(client, config.database), task_manager))
				})
			}
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.async_run(|mut config| {
					let (client, _, _, task_manager) = pangoro_service::new_chain_ops::<
						pangoro_runtime::RuntimeApi,
						PangoroExecutor,
					>(&mut config)?;

					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			} else {
				runner.async_run(|mut config| {
					let (client, _, _, task_manager) =
						pangolin_service::new_chain_ops::<
							pangolin_runtime::RuntimeApi,
							PangolinExecutor,
						>(&mut config, max_past_logs, target_gas_price)?;

					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			}
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) = pangoro_service::new_chain_ops::<
						pangoro_runtime::RuntimeApi,
						PangoroExecutor,
					>(&mut config)?;

					Ok((cmd.run(client, import_queue), task_manager))
				})
			} else {
				runner.async_run(|mut config| {
					let (client, _, import_queue, task_manager) =
						pangolin_service::new_chain_ops::<
							pangolin_runtime::RuntimeApi,
							PangolinExecutor,
						>(&mut config, max_past_logs, target_gas_price)?;

					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.sync_run(|config| cmd.run(config.database))
			} else {
				runner.sync_run(|config| {
					// Remove dvm offchain db
					let dvm_database_config = sc_service::DatabaseConfig::RocksDb {
						path: pangolin_service::dvm_database_dir(&config),
						cache_size: 0,
					};
					cmd.run(dvm_database_config)?;

					cmd.run(config.database)
				})
			}
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.async_run(|mut config| {
					let (client, backend, _, task_manager) = pangoro_service::new_chain_ops::<
						pangoro_runtime::RuntimeApi,
						PangoroExecutor,
					>(&mut config)?;

					Ok((cmd.run(client, backend), task_manager))
				})
			} else {
				runner.async_run(|mut config| {
					let (client, backend, _, task_manager) =
						pangolin_service::new_chain_ops::<
							pangolin_runtime::RuntimeApi,
							PangolinExecutor,
						>(&mut config, max_past_logs, target_gas_price)?;

					Ok((cmd.run(client, backend), task_manager))
				})
			}
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

			if chain_spec.is_pangoro() {
				runner.async_run(|config| {
					// we don't need any of the components of new_partial, just a runtime, or a task
					// manager to do `async_run`.
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager =
						sc_service::TaskManager::new(config.task_executor.clone(), registry)
							.map_err(|e| {
								sc_cli::Error::Service(sc_service::Error::Prometheus(e))
							})?;

					Ok((
						cmd.run::<pangoro_runtime::Block, PangoroExecutor>(config),
						task_manager,
					))
				})
			} else {
				runner.async_run(|config| {
					// we don't need any of the components of new_partial, just a runtime, or a task
					// manager to do `async_run`.
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager =
						sc_service::TaskManager::new(config.task_executor.clone(), registry)
							.map_err(|e| {
								sc_cli::Error::Service(sc_service::Error::Prometheus(e))
							})?;

					Ok((
						cmd.run::<pangolin_runtime::Block, PangolinExecutor>(config),
						task_manager,
					))
				})
			}
		}
		#[cfg(feature = "runtime-benchmarks")]
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			if chain_spec.is_pangoro() {
				runner.sync_run(|config| cmd.run::<pangoro_runtime::Block, PangoroExecutor>(config))
			} else {
				runner
					.sync_run(|config| cmd.run::<pangolin_runtime::Block, PangolinExecutor>(config))
			}
		}
	}
}
