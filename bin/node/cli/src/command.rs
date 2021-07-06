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
use std::path::PathBuf;
// --- substrate ---
use sc_cli::{Role, RunCmd, RuntimeVersion, SubstrateCli};
// --- darwinia ---
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use darwinia_cli::{Configuration, DarwiniaCli};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Darwinia Runtime Module Library".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
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

	fn executable_name() -> String {
		"drml".into()
	}

	fn native_runtime_version(_spec: &Box<dyn sc_service::ChainSpec>) -> &'static RuntimeVersion {
		&service::pangolin_runtime::VERSION
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["pangolin"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("pangolin")
		} else {
			id
		};

		Ok(match id {
			"pangolin" => Box::new(chain_spec::pangolin_config()?),
			"pangolin-dev" | "dev" => Box::new(chain_spec::pangolin_development_config()),
			"pangolin-genesis" => Box::new(chain_spec::pangolin_build_spec_config()),
			path => Box::new(chain_spec::PangolinChainSpec::from_json_file(
				PathBuf::from(path),
			)?),
		})
	}
}
impl DarwiniaCli for Cli {
	fn conf(&self) -> &Option<PathBuf> {
		&self.conf
	}

	fn base(&self) -> &RunCmd {
		&self.run.base
	}

	fn mut_base(&mut self) -> &mut RunCmd {
		&mut self.run.base
	}
}

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = Configuration::create_runner(Cli::from_args())?;
			runner.run_node_until_exit(|config| async move {
				match config.role {
					Role::Light => {
						service::drml_new_light(config).map(|(task_manager, _)| task_manager)
					}
					_ => service::drml_new_full(config, &cli)
						.map(|(task_manager, _, _)| task_manager),
				}
				.map_err(sc_cli::Error::Service)
			})
		}
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config, &cli)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config, &cli)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config, &cli)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;

				runner.sync_run(|config| {
					cmd.run::<service::pangolin_runtime::Block, service::PangolinExecutor>(config)
				})
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			}
		}
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.task_executor.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

				Ok((
					cmd.run::<service::pangolin_runtime::Block, service::PangolinExecutor>(config),
					task_manager,
				))
			})
		}
	}
}
