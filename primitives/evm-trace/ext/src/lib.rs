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

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates.io ---
#[cfg(feature = "std")]
use codec::Decode;
// --- paritytech ---
use evm_tracing_events::StepEventFilter;
#[cfg(feature = "std")]
use evm_tracing_events::{Event, EvmEvent, GasometerEvent, RuntimeEvent};
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

#[runtime_interface]
pub trait DvmExt {
	fn raw_step(&mut self, _data: Vec<u8>) {}

	fn raw_gas(&mut self, _data: Vec<u8>) {}

	fn raw_return_value(&mut self, _data: Vec<u8>) {}

	fn call_list_entry(&mut self, _index: u32, _value: Vec<u8>) {}

	fn call_list_new(&mut self) {}

	// New design, proxy events.
	/// An `Evm` event proxied by the runtime to this host function.
	/// evm -> runtime -> host.
	fn evm_event(&mut self, event: Vec<u8>) {
		if let Ok(event) = EvmEvent::decode(&mut &event[..]) {
			Event::Evm(event).emit();
		}
	}

	/// A `Gasometer` event proxied by the runtime to this host function.
	/// evm_gasometer -> runtime -> host.
	fn gasometer_event(&mut self, event: Vec<u8>) {
		if let Ok(event) = GasometerEvent::decode(&mut &event[..]) {
			Event::Gasometer(event).emit();
		}
	}

	/// A `Runtime` event proxied by the runtime to this host function.
	/// evm_runtime -> runtime -> host.
	fn runtime_event(&mut self, event: Vec<u8>) {
		if let Ok(event) = RuntimeEvent::decode(&mut &event[..]) {
			Event::Runtime(event).emit();
		}
	}

	/// Allow the tracing module in the runtime to know how to filter Step event
	/// content, as cloning the entire data is expensive and most of the time
	/// not necessary.
	fn step_event_filter(&self) -> StepEventFilter {
		evm_tracing_events::step_event_filter().unwrap_or_default()
	}

	/// An event to create a new CallList (currently a new transaction when tracing a block).
	#[version(2)]
	fn call_list_new(&mut self) {
		Event::CallListNew().emit();
	}
}
