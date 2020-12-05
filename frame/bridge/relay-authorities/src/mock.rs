// // This file is part of Darwinia.
// //
// // Copyright (C) 2018-2020 Darwinia Network
// // SPDX-License-Identifier: GPL-3.0
// //
// // Darwinia is free software: you can redistribute it and/or modify
// // it under the terms of the GNU General Public License as published by
// // the Free Software Foundation, either version 3 of the License, or
// // (at your option) any later version.
// //
// // Darwinia is distributed in the hope that it will be useful,
// // but WITHOUT ANY WARRANTY; without even the implied warranty of
// // MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// // GNU General Public License for more details.
// //
// // You should have received a copy of the GNU General Public License
// // along with Darwinia.  If not, see <https://www.gnu.org/licenses/>.

// //! # Mock file for relay authorities

// // --- substrate ---
// use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
// use sp_core::H256;
// use sp_io::TestExternalities;
// use sp_runtime::{
// 	testing::Header,
// 	traits::{BlakeTwo256, IdentityLookup},
// 	Perbill,
// };
// // --- darwinia ---
// use crate::*;

// pub type RelayAuthorities = Module<Test>;

// impl_outer_origin! {
// 	pub enum Origin for Test {}
// }

// darwinia_support::impl_test_account_data! {}

// #[derive(Clone, Eq, PartialEq)]
// pub struct Test;
// parameter_types! {
// 	pub const TemplateConst: u32 = 0;
// }
// impl Trait for Test {
// 	type Event = ();
// 	type TemplateConst = TemplateConst;
// 	type WeightInfo = ();
// }

// parameter_types! {
// 	pub const BlockHashCount: u64 = 250;
// 	pub const MaximumBlockWeight: Weight = 1024;
// 	pub const MaximumBlockLength: u32 = 2 * 1024;
// 	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
// }
// impl frame_system::Trait for Test {
// 	type BaseCallFilter = ();
// 	type Origin = Origin;
// 	type Call = ();
// 	type Index = u64;
// 	type BlockNumber = u64;
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type AccountId = u64;
// 	type Lookup = IdentityLookup<Self::AccountId>;
// 	type Header = Header;
// 	type Event = ();
// 	type BlockHashCount = BlockHashCount;
// 	type MaximumBlockWeight = MaximumBlockWeight;
// 	type DbWeight = ();
// 	type BlockExecutionWeight = ();
// 	type ExtrinsicBaseWeight = ();
// 	type MaximumExtrinsicWeight = MaximumBlockWeight;
// 	type MaximumBlockLength = MaximumBlockLength;
// 	type AvailableBlockRatio = AvailableBlockRatio;
// 	type Version = ();
// 	type PalletInfo = ();
// 	type AccountData = ();
// 	type OnNewAccount = ();
// 	type OnKilledAccount = ();
// 	type SystemWeightInfo = ();
// }

// pub fn new_test_ext() -> TestExternalities {
// 	frame_system::GenesisConfig::default()
// 		.build_storage::<Test>()
// 		.unwrap()
// 		.into()
// }
