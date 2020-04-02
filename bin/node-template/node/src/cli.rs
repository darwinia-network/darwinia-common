// --- crates ---
use structopt::StructOpt;
// --- substrate ---
use sc_cli::{RunCmd, Subcommand};

#[derive(Debug, StructOpt)]
pub struct Cli {
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[structopt(flatten)]
	pub run: RunCmd,
}
