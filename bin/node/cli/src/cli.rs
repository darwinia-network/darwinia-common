// --- crates ---
use structopt::StructOpt;
// --- substrate ---
use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};

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

#[allow(missing_docs)]
#[derive(Debug, StructOpt)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub base: sc_cli::RunCmd,

	// TODO: darwinia-cli
	/// Disable the authority discovery module on validator or sentry nodes.
	///
	/// Enabled by default on validator and sentry nodes. Always disabled on non
	/// validator or sentry nodes.
	///
	/// When enabled:
	///
	/// (1) As a validator node: Make oneself discoverable by publishing either
	///     ones own network addresses, or the ones of ones sentry nodes
	///     (configured via the `sentry-nodes` flag).
	///
	/// (2) As a validator or sentry node: Discover addresses of validators or
	///     addresses of their sentry nodes and maintain a permanent connection
	///     to a subset.
	#[structopt(long = "disable-authority-discovery")]
	pub authority_discovery_disabled: bool,
}

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Key management cli utilities
	Key(KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(SignCmd),
}
