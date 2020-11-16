// --- std ---
use std::path::PathBuf;
// --- substrate ---
use sc_cli::{Role, RunCmd, RuntimeVersion, SubstrateCli};
use sp_core::crypto::Ss58AddressFormat;
// --- darwinia ---
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use darwinia_cli::{Configuration, DarwiniaCli};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Darwinia Pangolin".into()
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
		"support.anonymous.an".into()
	}

	fn copyright_start_year() -> i32 {
		2018
	}

	fn executable_name() -> String {
		"pangolin".into()
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
			"pangolin-dev" | "dev" => Box::new(chain_spec::pangolin_development_config()),
			"pangolin-local" => Box::new(chain_spec::pangolin_local_testnet_config()),
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
		&self.run
	}

	fn mut_base(&mut self) -> &mut RunCmd {
		&mut self.run
	}
}

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

fn set_default_ss58_version(_spec: &Box<dyn sc_service::ChainSpec>) {
	sp_core::crypto::set_default_ss58_version(Ss58AddressFormat::DarwiniaAccount);
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = Configuration::create_runner(cli)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.run_node_until_exit(|config| match config.role {
				Role::Light => service::pangolin_new_light(config),
				_ => service::pangolin_new_full(config).map(|(components, _)| components),
			})
		}
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		// substrate 6804, #6999
		// Some(Subcommand::BuildSyncSpec(cmd)) => {}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops::<
					service::pangolin_runtime::RuntimeApi,
					service::PangolinExecutor,
				>(&mut config)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		}
		Some(Subcommand::Key(cmd)) => cmd.run(),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
	}
}
