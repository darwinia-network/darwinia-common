// --- std ---
use std::path::PathBuf;
// --- substrate ---
use sc_cli::{RunCmd, SubstrateCli};
// --- darwinia ---
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use darwinia_cli::{Configuration, DarwiniaCli};

impl SubstrateCli for Cli {
	fn impl_name() -> &'static str {
		"Darwinia Node"
	}

	fn impl_version() -> &'static str {
		env!("SUBSTRATE_CLI_IMPL_VERSION")
	}

	fn executable_name() -> &'static str {
		env!("CARGO_PKG_NAME")
	}

	fn description() -> &'static str {
		env!("CARGO_PKG_DESCRIPTION")
	}

	fn author() -> &'static str {
		env!("CARGO_PKG_AUTHORS")
	}

	fn support_url() -> &'static str {
		"support.anonymous.an"
	}

	fn copyright_start_year() -> i32 {
		2018
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()),
			"" | "local" => Box::new(chain_spec::local_testnet_config()),
			path => Box::new(chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
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

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runtime = Configuration::create_runner_from_cli(cli)?;
			runtime.run_node(
				service::new_light,
				service::new_full,
				node_template_runtime::VERSION,
			)
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;

				runner.sync_run(|config| {
					cmd.run::<node_template_runtime::Block, service::Executor>(config)
				})
			} else {
				println!(
					"Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
				);
				Ok(())
			}
		}
		Some(Subcommand::Base(subcommand)) => {
			let runner = cli.create_runner(subcommand)?;

			runner.run_subcommand(subcommand, |config| Ok(new_full_start!(config).0))
		}
	}
}
