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
		"Node Template".into()
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
		"node-template".into()
	}

	fn native_runtime_version(_spec: &Box<dyn sc_service::ChainSpec>) -> &'static RuntimeVersion {
		&service::node_template_runtime::VERSION
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["node-template"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("node-template")
		} else {
			id
		};

		Ok(match id {
			"node-template-dev" | "dev" => Box::new(chain_spec::node_template_development_config()),
			"node-template-local" => Box::new(chain_spec::node_template_local_testnet_config()),
			path => Box::new(chain_spec::NodeTemplateChainSpec::from_json_file(
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

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	fn set_default_ss58_version(_spec: &Box<dyn sc_service::ChainSpec>) {
		let ss58_version = Ss58AddressFormat::PolkadotAccount;

		sp_core::crypto::set_default_ss58_version(ss58_version);
	};

	match &cli.subcommand {
		None => {
			let runtime = Configuration::create_runner(cli)?;
			let chain_spec = &runtime.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runtime.run_node_until_exit(|config| match config.role {
				Role::Light => service::node_template_new_light(config),
				_ => service::node_template_new_full(config).map(|(components, _)| components),
			})
		}
		Some(Subcommand::Base(subcommand)) => {
			let runtime = cli.create_runner(subcommand)?;
			let chain_spec = &runtime.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runtime.run_subcommand(subcommand, |config| {
				service::new_chain_ops::<
					service::node_template_runtime::RuntimeApi,
					service::NodeTemplateExecutor,
				>(config)
			})
		}
		Some(Subcommand::Key(cmd)) => cmd.run(),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
	}
}
