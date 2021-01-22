// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

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
	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	// substrate 6804, #6999
	// /// Build a chain specification with a light client sync state.
	// BuildSyncSpec(sc_cli::BuildSyncSpecCmd),
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
