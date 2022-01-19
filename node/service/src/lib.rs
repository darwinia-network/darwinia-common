// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

pub mod client;

pub mod chain_spec;
#[cfg(feature = "template")]
pub use chain_spec::template as template_chain_spec;
pub use chain_spec::{
	pangolin as pangolin_chain_spec, pangoro as pangoro_chain_spec, PangolinChainSpec,
	PangoroChainSpec,
};

pub mod service;
#[cfg(feature = "template")]
pub use service::template as template_service;
pub use service::{pangolin as pangolin_service, pangoro as pangoro_service, *};

pub use pangolin_runtime::{self, RuntimeApi as PangolinRuntimeApi};
pub use pangoro_runtime::{self, RuntimeApi as PangoroRuntimeApi};
#[cfg(feature = "template")]
pub use template_runtime;
