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
				let mut best_block_number = 0;

				BestBlockNumber::put(best_block_number);
				Headers::insert(
					best_block_number,
					MockTcHeader {
						number: 0,
						hash: 0,
						parent_hash: 0,
						valid: true,
					}
				);

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
		type HeaderThingWithProof = MockTcHeader;
		type HeaderThing = MockTcHeader;

		fn verify(
			proposal_with_proof: Vec<Self::HeaderThingWithProof>,
		) -> Result<Vec<Self::HeaderThing>, DispatchError> {
			let verify = |header: &Self::HeaderThing| -> DispatchResult {
				ensure!(header.valid, "Header - INVALID");

				Ok(())
			};
			let mut proposal = vec![];

			for header_thing in proposal_with_proof {
				verify(&header_thing)?;

				proposal.push(header_thing);
			}

			Ok(proposal)
		}

		fn best_block_number() -> <Self::HeaderThing as HeaderThing>::Number {
			Self::best_block_number()
		}

		fn on_chain_arbitrate(mut proposal: Vec<Self::HeaderThing>) -> DispatchResult {
			proposal.sort_by_key(|header_thing| header_thing.number);

			let mut parent_hash = proposal.pop().unwrap().parent_hash;

			while let Some(header_thing) = proposal.pop() {
				ensure!(parent_hash == header_thing.hash, "Continuous - INVALID");

				parent_hash = header_thing.parent_hash;
			}

			ensure!(
				parent_hash
					== Self::header_of_block_number(Self::best_block_number())
						.unwrap()
						.hash,
				"Continuous - INVALID"
			);

			Ok(())
		}

		fn store_header(header_thing: Self::HeaderThing) -> DispatchResult {
			BestBlockNumber::mutate(|best_block_number| {
				if header_thing.number > *best_block_number {
					*best_block_number = header_thing.number;

					Headers::insert(header_thing.number, header_thing);
				}
			});

			Ok(())
		}
	}

	#[derive(Clone, Debug, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
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

		pub fn mock_proposal(mut validations: Vec<u8>, valid: bool) -> Vec<Self> {
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
	}
	impl HeaderThing for MockTcHeader {
		type Number = MockTcBlockNumber;
		type Hash = MockTcHeaderHash;

		fn number(&self) -> Self::Number {
			self.number
		}

		fn hash(&self) -> Self::Hash {
			self.hash
		}
	}
}

// --- std ---
use std::{cell::RefCell, time::Instant};
// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	impl_outer_origin, parameter_types,
	traits::{OnFinalize, OnInitialize},
	weights::Weight,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill, RuntimeDebug};
// --- darwinia ---
use crate::*;
use darwinia_relay_primitives::*;
use mock_relay::MockTcHeader;

pub type AccountId = u64;
pub type AccountIndex = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub type RingInstance = darwinia_balances::Instance0;
pub type _RingError = darwinia_balances::Error<Test, RingInstance>;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

pub type KtonInstance = darwinia_balances::Instance1;
pub type _KtonError = darwinia_balances::Error<Test, KtonInstance>;

pub type System = frame_system::Module<Test>;
pub type Relay = mock_relay::Module<Test>;

pub type RelayerGameError = Error<Test, DefaultInstance>;
pub type RelayerGame = Module<Test>;

thread_local! {
	static GENESIS_TIME: Instant = Instant::now();
	static CHALLENGE_TIME: RefCell<BlockNumber> = RefCell::new(3);
	static ESTIMATE_BOND: RefCell<Balance> = RefCell::new(1);
	static CONFIRM_PERIOD: RefCell<BlockNumber> = RefCell::new(0);
}

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
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

pub struct ConfirmPeriod;
impl Get<BlockNumber> for ConfirmPeriod {
	fn get() -> BlockNumber {
		CONFIRM_PERIOD.with(|v| v.borrow().to_owned())
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
	type ConfirmPeriod = ConfirmPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const MinimumPeriod: u64 = 5;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
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
	type PalletInfo = ();
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
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
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
}
impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
}

pub struct RelayerGameAdjustor;
impl AdjustableRelayerGame for RelayerGameAdjustor {
	type Moment = BlockNumber;
	type Balance = Balance;
	type TcBlockNumber = <<Relay as Relayable>::HeaderThing as HeaderThing>::Number;

	fn challenge_time(_round: Round) -> Self::Moment {
		CHALLENGE_TIME.with(|v| v.borrow().to_owned())
	}

	fn round_of_samples_count(samples_count: u64) -> Round {
		samples_count - 1
	}

	fn samples_count_of_round(round: Round) -> u64 {
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
	headers: Vec<MockTcHeader>,
	challenge_time: BlockNumber,
	estimate_bond: Balance,
	confirmed_period: BlockNumber,
}
impl ExtBuilder {
	pub fn headers(mut self, headers: Vec<MockTcHeader>) -> Self {
		self.headers = headers;

		self
	}
	pub fn challenge_time(mut self, challenge_time: BlockNumber) -> Self {
		self.challenge_time = challenge_time;

		self
	}
	pub fn estimate_bond(mut self, estimate_bond: Balance) -> Self {
		self.estimate_bond = estimate_bond;

		self
	}
	pub fn confirmed_period(mut self, confirmed_period: BlockNumber) -> Self {
		self.confirmed_period = confirmed_period;

		self
	}

	pub fn set_associated_constants(&self) {
		CHALLENGE_TIME.with(|v| v.replace(self.challenge_time));
		ESTIMATE_BOND.with(|v| v.replace(self.estimate_bond));
		CONFIRM_PERIOD.with(|v| v.replace(self.confirmed_period));
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_constants();

		let _ = env_logger::try_init();
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		darwinia_balances::GenesisConfig::<Test, RingInstance> {
			balances: (1..10)
				.map(|i: AccountId| vec![(i, 100 * i as Balance), (10 * i, 1000 * i as Balance)])
				.flatten()
				.collect(),
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
			headers: vec![],
			challenge_time: RelayerGameAdjustor::challenge_time(0),
			estimate_bond: RelayerGameAdjustor::estimate_bond(0, 0),
			confirmed_period: CONFIRM_PERIOD.with(|v| v.borrow().to_owned()),
		}
	}
}

pub fn run_to_block(n: BlockNumber) {
	RelayerGame::on_finalize(System::block_number());

	for b in System::block_number() + 1..=n {
		System::set_block_number(b);
		RelayerGame::on_initialize(b);

		if b != n {
			RelayerGame::on_finalize(System::block_number());
		}
	}
}
