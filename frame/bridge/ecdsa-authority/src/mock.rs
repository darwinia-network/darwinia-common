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

// --- core ---
use core::iter;
// --- crates.io ---
use libsecp256k1::{PublicKey, SecretKey};
// --- paritytech ---
use frame_support::traits::{Everything, GenesisBuild, OnInitialize};
use frame_system::mocking::*;
use sp_io::{hashing, TestExternalities};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
// --- darwinia-network ---
use crate::{self as darwinia_ecdsa_authority, *};

pub(crate) type EcdsaAuthorityError = Error<Test>;

type Block = MockBlock<Test>;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;

type BlockNumber = u64;
type AccountId = u64;
type Index = u64;

impl frame_system::Config for Test {
	type AccountData = ();
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ();
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = Index;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

frame_support::parameter_types! {
	  pub const MaxAuthorities: u32 = 3;
	  pub const MaxPendingPeriod: BlockNumber = 5;
	  pub const SignThreshold: Perbill = Perbill::from_percent(60);
	  pub const SyncInterval: BlockNumber = 3;
	  pub static MessageRoot: Option<Hash> = Some(Default::default());
}
impl Config for Test {
	type Event = Event;
	type MaxAuthorities = MaxAuthorities;
	type MaxPendingPeriod = MaxPendingPeriod;
	type MessageRoot = MessageRoot;
	type SignThreshold = SignThreshold;
	type SyncInterval = SyncInterval;
	type WeightInfo = ();
}

frame_support::construct_runtime! {
	pub enum Test
	where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		EcdsaAuthority: darwinia_ecdsa_authority::{Pallet, Call, Storage, Config, Event<T>}
	}
}

#[derive(Default)]
pub(crate) struct ExtBuilder {
	authorities: Vec<Address>,
}
impl ExtBuilder {
	pub(crate) fn authorities(mut self, authorities: Vec<Address>) -> Self {
		self.authorities = authorities;

		self
	}

	pub(crate) fn build(self) -> TestExternalities {
		let Self { authorities } = self;
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		<darwinia_ecdsa_authority::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
			&darwinia_ecdsa_authority::GenesisConfig { authorities },
			&mut storage,
		)
		.unwrap();

		let mut ext = TestExternalities::from(storage);

		ext.execute_with(|| {
			System::set_block_number(1);
			<EcdsaAuthority as OnInitialize<_>>::on_initialize(1);
		});

		ext
	}
}

pub(crate) fn gen_pair(byte: u8) -> (SecretKey, Address) {
	let seed = iter::repeat(byte).take(32).collect::<Vec<_>>();
	let secret_key = SecretKey::parse_slice(&seed).unwrap();
	let public_key = PublicKey::from_secret_key(&secret_key).serialize();
	let address = Address::from_slice(&hashing::keccak_256(&public_key[1..65])[12..]);

	(secret_key, address)
}

pub(crate) fn sign(secret_key: &SecretKey, message: &Message) -> Signature {
	let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(message), secret_key);
	let mut signature = [0u8; 65];

	signature[0..64].copy_from_slice(&sig.serialize()[..]);
	signature[64] = recovery_id.serialize();

	Signature(signature)
}

pub(crate) fn clear_authorities_change() {
	<AuthoritiesChangeToSign<Test>>::kill();
}

pub(crate) fn new_message_root(byte: u8) {
	MESSAGE_ROOT.with(|v| *v.borrow_mut() = Some(Hash::repeat_byte(byte)));
}

pub(crate) fn run_to_block(n: BlockNumber) {
	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		<EcdsaAuthority as OnInitialize<_>>::on_initialize(b);
	}
}

pub(crate) fn ecdsa_authority_events() -> Vec<crate::Event<Test>> {
	fn events() -> Vec<Event> {
		let events = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

		System::reset_events();

		events
	}

	events()
		.into_iter()
		.filter_map(|e| match e {
			Event::EcdsaAuthority(e) => Some(e),
			_ => None,
		})
		.collect::<Vec<_>>()
}
