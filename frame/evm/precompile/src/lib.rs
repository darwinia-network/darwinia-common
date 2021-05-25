// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

#![cfg_attr(not(feature = "std"), no_std)]

pub type DarwiniaPrecompiles<Runtime> = (
	darwinia_evm_precompile_simple::ECRecover, // 0x0000000000000000000000000000000000000001
	darwinia_evm_precompile_simple::Sha256,    // 0x0000000000000000000000000000000000000002
	darwinia_evm_precompile_simple::Ripemd160, // 0x0000000000000000000000000000000000000003
	darwinia_evm_precompile_simple::Identity,  // 0x0000000000000000000000000000000000000004
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000005
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000006
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000007
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000008
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000009
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000a
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000b
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000c
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000d
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000e
	darwinia_evm_precompile_empty::Empty,      // 0x000000000000000000000000000000000000000f
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000010
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000011
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000012
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000013
	darwinia_evm_precompile_empty::Empty,      // 0x0000000000000000000000000000000000000014
	darwinia_evm_precompile_withdraw::WithDraw<Runtime>, // 0x0000000000000000000000000000000000000015
	darwinia_evm_precompile_kton::Kton<Runtime>, // 0x0000000000000000000000000000000000000016
	darwinia_evm_precompile_issuing::Issuing<Runtime>, // 0x0000000000000000000000000000000000000017
    darwinia_evm_precompile_dispatch_wrapper::DispatchWrapper<Runtime>, // 0x0000000000000000000000000000000000000018
);
