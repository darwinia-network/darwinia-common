pub mod mock_relay {
	// --- substrate ---
	use sp_runtime::{DispatchError, DispatchResult};
	// --- darwinia ---
	use crate::*;
	use darwinia_support::relay::Relayable;

	pub type MockTcBlockNumber = u64;

	decl_storage! {
		trait Store for Module<T: Trait> as DarwiniaRelay {
			pub BestBlockNumber get(fn best_block_number): MockTcBlockNumber;

			pub Headers
				get(fn header_of_block_number)
				: map hasher(identity) MockTcBlockNumber
				=> Option<MockTcHeader>;
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
		type TcHeaderHash = ();
		type TcHeaderMMR = ();

		fn best_block_number() -> Self::TcBlockNumber {
			Self::best_block_number()
		}

		fn header_existed(block_number: Self::TcBlockNumber) -> bool {
			Self::best_block_number() >= block_number
		}

		fn verify_raw_header_thing(
			raw_header_thing: RawHeaderThing,
			with_raw_header: bool,
		) -> Result<
			(
				TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
				RawHeaderThing,
			),
			DispatchError,
		> {
			let header =
				MockTcHeader::decode(&mut &*raw_header_thing).map_err(|_| "Decode - FAILED")?;
			let verify = |header: &MockTcHeader| -> DispatchResult {
				ensure!(header.valid != Validation::HashInvalid, "Header - INVALID");

				Ok(())
			};

			verify(&header)?;

			Ok((
				TcHeaderBrief {
					number: header.number,
					hash: (),
					parent_hash: (),
					mmr: (),
					others: header.valid.encode(),
				},
				if with_raw_header {
					raw_header_thing
				} else {
					vec![]
				},
			))
		}

		fn on_chain_arbitrate(
			header_brief_chain: Vec<
				TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
			>,
		) -> DispatchResult {
			for header_briefs in header_brief_chain {
				let validation = Validation::decode(&mut &*header_briefs.others)
					.map_err(|_| "Decode - FAILED")?;

				ensure!(
					validation != Validation::ContinuousInvalid,
					"Continuous - INVALID"
				);
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

	#[derive(Debug, PartialEq, Encode, Decode)]
	pub struct MockTcHeader {
		pub number: MockTcBlockNumber,
		pub valid: Validation,
	}
	impl MockTcHeader {
		pub fn new(number: MockTcBlockNumber, valid: u8) -> Self {
			Self {
				number,
				valid: valid.into(),
			}
		}

		pub fn new_raw(number: MockTcBlockNumber, valid: u8) -> RawHeaderThing {
			Self::new(number, valid).encode()
		}
	}

	#[derive(Debug, PartialEq, Encode, Decode)]
	pub enum Validation {
		Valid,
		HashInvalid,
		ContinuousInvalid,
	}
	impl Into<Validation> for u8 {
		fn into(self) -> Validation {
			match self {
				0 => Validation::Valid,
				1 => Validation::HashInvalid,
				2 => Validation::ContinuousInvalid,
				_ => unreachable!(),
			}
		}
	}
}

// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, traits::OnFinalize, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
// --- darwinia ---
use crate::*;
use darwinia_support::relay::AdjustableRelayerGame;

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

impl Trait for Test {
	type Event = ();
	type RingCurrency = Ring;
	type RingSlash = ();
	type RelayerGameAdjustor = RelayerGameAdjustor;
	type TargetChain = Relay;
}

pub struct RelayerGameAdjustor;
impl AdjustableRelayerGame for RelayerGameAdjustor {
	type Moment = BlockNumber;
	type Balance = Balance;
	type TcBlockNumber = mock_relay::MockTcBlockNumber;

	fn challenge_time(_round: Round) -> Self::Moment {
		3
	}

	fn round_from_chain_len(chain_len: u64) -> Round {
		chain_len - 1
	}

	fn chain_len_from_round(round: Round) -> u64 {
		round + 1
	}

	fn update_samples(_round: Round, samples: &mut Vec<Self::TcBlockNumber>) {
		samples.push(samples.last().unwrap() - 1);
	}

	fn estimate_bond(_round: Round, _proposals_count: u64) -> Self::Balance {
		1
	}
}

pub struct ExtBuilder {}
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		darwinia_balances::GenesisConfig::<Test, RingInstance> {
			balances: vec![(1, 100), (2, 200), (3, 300)],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
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
