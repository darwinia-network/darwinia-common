// --- substrate ---
use sc_cli::VersionInfo;
// --- darwinia ---
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};

/// Parse command line arguments into service configuration.
pub fn run<I, T>(args: I, version: VersionInfo) -> sc_cli::Result<()>
where
	I: Iterator<Item = T>,
	T: Into<std::ffi::OsString> + Clone,
{
	sc_cli::reset_signal_pipe_handler()?;

	let args: Vec<_> = args.collect();
	let opt = sc_cli::from_iter::<Cli, _>(args.clone(), &version);

	let mut config = sc_service::Configuration::from_version(&version);

	match opt.subcommand {
		Some(Subcommand::Base(subcommand)) => {
			subcommand.init(&version)?;
			subcommand.update_config(&mut config, chain_spec::load_spec, &version)?;
			subcommand.run(config, |config: _| Ok(new_full_start!(config).0))
		}
		Some(Subcommand::Benchmark(cmd)) => {
			cmd.init(&version)?;
			cmd.update_config(&mut config, chain_spec::load_spec, &version)?;

			cmd.run::<node_template_runtime::Block, service::Executor>(config)
		}
		None => {
			opt.run.init(&version)?;
			opt.run
				.update_config(&mut config, chain_spec::load_spec, &version)?;
			opt.run
				.run(config, service::new_light, service::new_full, &version)
		}
	}
}
