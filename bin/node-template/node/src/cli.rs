// --- crates ---
use structopt::StructOpt;
// --- substrate ---
use sc_cli::{KeySubcommand, RunCmd, SignCmd, VanityCmd, VerifyCmd};

/// An overarching CLI command definition.
#[derive(Debug, StructOpt)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub run: RunCmd,

	/// Load the boot configuration json file from <PATH>. Command line input will be overwritten by this.
	#[structopt(long = "conf", value_name = "PATH")]
	pub conf: Option<std::path::PathBuf>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
	/// A set of base subcommands handled by `sc_cli`.
	#[structopt(flatten)]
	Base(sc_cli::Subcommand),

	/// Key management cli utilities
	Key(KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(SignCmd),
}
