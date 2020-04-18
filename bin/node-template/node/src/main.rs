//! Darwinia Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod rpc;

fn main() -> sc_cli::Result<()> {
	let version = sc_cli::VersionInfo {
		name: "Darwinia Node",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "node-template",
		author: "Anonymous",
		description: "Template Node",
		support_url: "https://github.com/darwinia-network/darwinia-common/issues/new",
		copyright_start_year: 2020,
	};

	command::run(version)
}
