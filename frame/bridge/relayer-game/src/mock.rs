pub mod mock_relay {
	pub mod types {
		pub type MockTcBlockNumber = u64;
		pub type MockTcHeaderHash = u128;
	}

	// --- crates ---
	use serde::{Deserialize, Serialize};
	// --- substrate ---
	use sp_runtime::{DispatchError, DispatchResult};
	// --- darwinia ---
	use crate::{mock::*, *};
	use darwinia_support::relay::Relayable;
	use types::*;

	decl_storage! {
		trait Store for Module<T: Trait> as DarwiniaRelay {
			pub BestBlockNumber get(fn best_block_number): MockTcBlockNumber;

			pub Headers
				get(fn header_of_block_number)
				: map hasher(identity) MockTcBlockNumber
				=> Option<MockTcHeader>;
		}

		add_extra_genesis {
			config(headers): Vec<MockTcHeader>;
			build(|config: &GenesisConfig| {
				let mut best_block_number = MockTcBlockNumber::zero();

				for header in &config.headers {
					Headers::insert(header.number, header.clone());

					best_block_number = best_block_number.max(header.number);
				}

				BestBlockNumber::put(best_block_number);
			});
		}
	}

	decl_module! {
		pub struct Module<T: Trait> for enum Call
		where
			origin: T::Origin
		{

		}
	}

	impl<T: Trait> Relayable for Module<T> {
		type TcBlockNumber = MockTcBlockNumber;
		type TcHeaderHash = MockTcHeaderHash;
		type TcHeaderMMR = H256;

		fn best_block_number() -> Self::TcBlockNumber {
			Self::best_block_number()
		}

		fn verify_raw_header_thing(
			raw_header_thing: RawHeaderThing,
			with_proposed_raw_header: bool,
		) -> Result<
			(
				TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
				RawHeaderThing,
			),
			DispatchError,
		> {
			let verify = |header: &MockTcHeader| -> DispatchResult {
				ensure!(header.valid, "Header - INVALID");

				Ok(())
			};
			let header =
				MockTcHeader::decode(&mut &*raw_header_thing).map_err(|_| "Decode - FAILED")?;

			verify(&header)?;

			Ok((
				TcHeaderBrief {
					number: header.number,
					hash: header.hash,
					parent_hash: header.parent_hash,
					mmr: (),
					others: vec![],
				},
				if with_proposed_raw_header {
					raw_header_thing
				} else {
					vec![]
				},
			))
		}

		fn on_chain_arbitrate(
			mut header_brief_chain: Vec<
				TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
			>,
		) -> DispatchResult {
			header_brief_chain.sort_by_key(|header_brief| header_brief.number);

			let mut parent_hash = header_brief_chain.pop().unwrap().hash;

			while let Some(header_brief) = header_brief_chain.pop() {
				ensure!(
					parent_hash != header_brief.parent_hash,
					"Continuous - INVALID"
				);

				parent_hash = header_brief.hash;
			}

			Ok(())
		}

		fn store_header(raw_header_thing: RawHeaderThing) -> DispatchResult {
			let header =
				MockTcHeader::decode(&mut &*raw_header_thing).map_err(|_| "Decode - FAILED")?;

			Headers::insert(header.number, header);

			Ok(())
		}
	}

	#[derive(Clone, Debug, PartialEq, Encode, Decode, Serialize, Deserialize)]
	pub struct MockTcHeader {
		pub number: MockTcBlockNumber,
		pub hash: MockTcHeaderHash,
		pub parent_hash: MockTcHeaderHash,
		pub valid: bool,
	}
	impl MockTcHeader {
		pub fn mock(number: MockTcBlockNumber, parent_hash: MockTcHeaderHash, valid: u8) -> Self {
			let valid = match valid {
				0 => false,
				_ => true,
			};

			Self {
				number,
				hash: GENESIS_TIME.with(|v| v.to_owned()).elapsed().as_nanos(),
				parent_hash,
				valid,
			}
		}

		pub fn mock_raw(
			number: MockTcBlockNumber,
			parent_hash: MockTcHeaderHash,
			valid: u8,
		) -> RawHeaderThing {
			Self::mock(number, parent_hash, valid).encode()
		}

		pub fn mock_chain(mut validations: Vec<u8>, valid: bool) -> Vec<Self> {
			if validations.is_empty() {
				return vec![];
			}

			let mut parent_hash = if valid {
				0
			} else {
				GENESIS_TIME.with(|v| v.to_owned()).elapsed().as_nanos()
			};
			let mut chain = vec![Self::mock(1, parent_hash, validations[0])];

			parent_hash = chain[0].hash;

			if validations.len() > 1 {
				validations.remove(0);

				for (valid, number) in validations.into_iter().zip(2..) {
					let header = Self::mock(number, parent_hash, valid);

					parent_hash = header.hash;

					chain.push(header);
				}
			}

			chain.reverse();

			chain
		}

		pub fn mock_raw_chain(validations: Vec<u8>, valid: bool) -> Vec<RawHeaderThing> {
			Self::mock_chain(validations, valid)
				.into_iter()
				.map(|header| header.encode())
				.collect()
		}
	}
}

// --- std ---
use std::{cell::RefCell, time::Instant};
// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, traits::OnFinalize, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
// --- darwinia ---
use crate::*;
use darwinia_support::relay::AdjustableRelayerGame;
use mock_relay::{types::*, MockTcHeader};

pub type AccountId = u64;
pub type AccountIndex = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub type RingInstance = darwinia_balances::Instance0;
pub type _RingError = darwinia_balances::Error<Test, RingInstance>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

pub type KtonInstance = darwinia_balances::Instance1;
pub type _KtonError = darwinia_balances::Error<Test, KtonInstance>;
pub type Kton = darwinia_balances::Module<Test, KtonInstance>;

pub type System = frame_system::Module<Test>;
pub type Relay = mock_relay::Module<Test>;

pub type RelayerGameError = Error<Test, DefaultInstance>;
pub type RelayerGame = Module<Test>;

thread_local! {
	static GENESIS_TIME: Instant = Instant::now();
	static CHALLENGE_TIME: RefCell<BlockNumber> = RefCell::new(3);
	static ESTIMATE_BOND: RefCell<Balance> = RefCell::new(1);
}

impl_outer_origin! {
	pub enum Origin for Test  where system = frame_system {}
}

darwinia_support::impl_account_data! {
	struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
		// other data
	}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
impl Trait for Test {
	type Event = ();
	type RingCurrency = Ring;
	type RingSlash = ();
	type RelayerGameAdjustor = RelayerGameAdjustor;
	type TargetChain = Relay;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const MinimumPeriod: u64 = 5;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = AccountIndex;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}
impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Kton,);
}
impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Ring,);
}

pub struct RelayerGameAdjustor;
impl AdjustableRelayerGame for RelayerGameAdjustor {
	type Moment = BlockNumber;
	type Balance = Balance;
	type TcBlockNumber = MockTcBlockNumber;

	fn challenge_time(_round: Round) -> Self::Moment {
		CHALLENGE_TIME.with(|v| v.borrow().to_owned())
	}

	fn round_from_chain_len(chain_len: u64) -> Round {
		chain_len - 1
	}

	fn chain_len_from_round(round: Round) -> u64 {
		round + 1
	}

	fn update_samples(samples: &mut Vec<Vec<Self::TcBlockNumber>>) {
		samples.push(vec![samples.last().unwrap().last().unwrap() - 1]);
	}

	fn estimate_bond(_round: Round, _proposals_count: u64) -> Self::Balance {
		ESTIMATE_BOND.with(|v| v.borrow().to_owned())
	}
}

pub struct ExtBuilder {
	challenge_time: BlockNumber,
	estimate_bond: Balance,
	headers: Vec<MockTcHeader>,
}
impl ExtBuilder {
	pub fn challenge_time(mut self, challenge_time: BlockNumber) -> Self {
		self.challenge_time = challenge_time;

		self
	}
	pub fn estimate_bond(mut self, estimate_bond: Balance) -> Self {
		self.estimate_bond = estimate_bond;

		self
	}
	pub fn headers(mut self, headers: Vec<MockTcHeader>) -> Self {
		self.headers = headers;

		self
	}

	pub fn set_associated_constants(&self) {
		CHALLENGE_TIME.with(|v| v.replace(self.challenge_time));
		ESTIMATE_BOND.with(|v| v.replace(self.estimate_bond));
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_constants();

		let _ = env_logger::try_init();
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		darwinia_balances::GenesisConfig::<Test, RingInstance> {
			balances: vec![(1, 100), (2, 200), (3, 300)],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		mock_relay::GenesisConfig {
			headers: self.headers.clone(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			challenge_time: RelayerGameAdjustor::challenge_time(0),
			estimate_bond: RelayerGameAdjustor::estimate_bond(0, 0),
			headers: vec![],
		}
	}
}

pub fn run_to_block(n: BlockNumber) {
	RelayerGame::on_finalize(System::block_number());

	for b in System::block_number() + 1..=n {
		System::set_block_number(b);

		if b != n {
			RelayerGame::on_finalize(System::block_number());
		}
	}
}
