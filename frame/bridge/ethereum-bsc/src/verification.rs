// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use crate::*;
use bp_bsc::BSCHeader;

/// Perform checks that require access to parent header.
pub fn contextual_checks(
	config: &BSCConfiguration,
	header: &BSCHeader,
	parent: &BSCHeader,
) -> Result<(), Error> {
	// parent sanity check
	if parent.compute_hash() != header.parent_hash || parent.number + 1 != header.number {
		return Err(Error::UnknownAncestor);
	}

	// Ensure that the block's timestamp isn't too close to it's parent
	// And header.timestamp is greater than parents'
	if header.timestamp < parent.timestamp.saturating_add(config.period) {
		return Err(Error::HeaderTimestampTooClose);
	}

	Ok(())
}
