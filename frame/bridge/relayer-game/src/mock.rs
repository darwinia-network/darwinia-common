pub mod alias {
	pub mod relayer_game {
		pub use crate::Event;
	}

	// --- substrate ---
	pub use frame_system as system;
	// --- darwinia ---
	pub use darwinia_balances as balances;
}

pub mod mock_relay {
	pub mod types {
		pub type MockRelayBlockNumber = u32;
		pub type MockRelayHeaderHash = u128;
	}

	pub use types::*;

	// --- crates ---
	use serde::{Deserialize, Serialize};
	// --- substrate ---
	use sp_runtime::{DispatchError, DispatchResult};
	// --- darwinia ---
	use crate::{mock::*, *};

	decl_storage! {
		trait Store for Module<T: Trait> as DarwiniaRelay {
			pub RelaiedBlockNumbers get(fn best_relaied_block_number): MockRelayBlockNumber;

			pub RelaiedHeaders
				get(fn relaied_header_of)
				: map hasher(identity) MockRelayBlockNumber
				=> Option<MockRelayHeader>;
		}

		add_extra_genesis {
			config(headers): Vec<MockRelayHeader>;
			build(|config: &GenesisConfig| {
				let mut best_relaied_block_number = 0;

				RelaiedHeaders::insert(
					best_relaied_block_number,
					MockRelayHeader {
						number: 0,
						hash: 0,
						parent_hash: 0,
						valid: true,
					}
				);

				for header in &config.headers {
					RelaiedHeaders::insert(header.number, header.clone());

					best_relaied_block_number = best_relaied_block_number.max(header.number);
				}

				RelaiedBlockNumbers::put(best_relaied_block_number);
			});
		}
	}

	decl_module! {
		pub struct Module<T: Trait> for enum Call
		where
			origin: T::Origin
		{}
	}

	impl<T: Trait> Relayable for Module<T> {
		type RelayBlockId = MockRelayBlockNumber;
		type RelayParcel = MockRelayHeader;
		type Proofs = ();

		fn best_relaied_block_id() -> Self::RelayBlockId {
			Self::best_relaied_block_number()
		}

		fn verify_proofs(
			_: &Self::RelayBlockId,
			relay_parcel: &Self::RelayParcel,
			_: &Self::Proofs,
			_: Option<&Self::RelayBlockId>,
		) -> DispatchResult {
			ensure!(relay_parcel.valid, "Parcel - INVALID");

			Ok(())
		}

		fn verify_continuous(
			relay_parcels: &Self::RelayParcel,
			extended_relay_parcels: &Self::RelayParcel,
		) -> DispatchResult {
			ensure!(
				relay_parcels.parent_hash == extended_relay_parcels.hash,
				"Continuous - INVALID"
			);

			Ok(())
		}

		fn distance_between(
			relay_block_id: &Self::RelayBlockId,
			best_relaied_block_id: Self::RelayBlockId,
		) -> u32 {
			relay_block_id - best_relaied_block_id
		}

		fn store_relay_parcel(relay_parcel: Self::RelayParcel) -> DispatchResult {
			RelaiedBlockNumbers::mutate(|best_relaied_block_number| {
				if relay_parcel.number > *best_relaied_block_number {
					*best_relaied_block_number = relay_parcel.number;

					RelaiedHeaders::insert(relay_parcel.number, relay_parcel);
				}
			});

			Ok(())
		}
	}

	#[derive(
		Clone, Debug, Default, PartialEq, PartialOrd, Encode, Decode, Serialize, Deserialize,
	)]
	pub struct MockRelayHeader {
		pub number: MockRelayBlockNumber,
		pub hash: MockRelayHeaderHash,
		pub parent_hash: MockRelayHeaderHash,
		pub valid: bool,
	}
	impl MockRelayHeader {
		pub fn gen(
			number: MockRelayBlockNumber,
			parent_hash: MockRelayHeaderHash,
			valid: u8,
		) -> Self {
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

		pub fn gen_continous(
			start: u32,
			mut validations: Vec<u8>,
			continous_valid: bool,
		) -> Vec<Self> {
			if validations.is_empty() {
				return vec![];
			}

			let mut parent_hash = if continous_valid {
				0
			} else {
				GENESIS_TIME.with(|v| v.to_owned()).elapsed().as_nanos()
			};
			let mut chain = vec![Self::gen(start, parent_hash, validations[0])];

			parent_hash = chain[0].hash;

			if validations.len() > 1 {
				validations.remove(0);

				for (valid, number) in validations.into_iter().zip(start + 1..) {
					let header = Self::gen(number, parent_hash, valid);

					parent_hash = header.hash;

					chain.push(header);
				}
			}

			chain.reverse();

			chain
		}
	}
	impl BlockInfo for MockRelayHeader {
		type BlockId = u32;

		fn block_id(&self) -> Self::BlockId {
			self.number
		}
	}
}

// --- std ---
use std::{cell::RefCell, time::Instant};
// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
	traits::{OnFinalize, OnInitialize},
	weights::Weight,
};
use sp_runtime::{Perbill, RuntimeDebug};
// --- darwinia ---
use crate::*;
use alias::*;
use darwinia_relay_primitives::*;
use mock_relay::{MockRelayBlockNumber, MockRelayHeader};

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub type Extrinsic = sp_runtime::testing::TestXt<Call, ()>;

pub type RingInstance = darwinia_balances::Instance0;
pub type Ring = darwinia_balances::Module<Test, RingInstance>;

pub type KtonInstance = darwinia_balances::Instance1;

pub type System = frame_system::Module<Test>;
pub type Relay = mock_relay::Module<Test>;

pub type RelayerGameError = Error<Test, DefaultInstance>;
pub type RelayerGame = Module<Test, DefaultInstance>;

thread_local! {
	static GENESIS_TIME: Instant = Instant::now();
	static CHALLENGE_TIME: RefCell<BlockNumber> = RefCell::new(3);
	static ESTIMATE_BOND: RefCell<Balance> = RefCell::new(1);
	static CONFIRM_PERIOD: RefCell<BlockNumber> = RefCell::new(0);
}

impl_outer_origin! {
	pub enum Origin for Test
	where
		system = frame_system
	{}
}

impl_outer_dispatch! {
	pub enum Call for Test
	where
		origin: Origin
	{
		relayer_game::RelayerGame,
	}
}

impl_outer_event! {
	pub enum Event for Test {
		system <T>,
		balances Instance0<T>,
		relayer_game <T>,
	}
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
	type Call = Call;
	type Event = Event;
	type RingCurrency = Ring;
	type RingSlash = ();
	type RelayerGameAdjustor = RelayerGameAdjustor;
	type RelayableChain = Relay;
	type ConfirmPeriod = ConfirmPeriod;
	type WeightInfo = ();
}

impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = sp_core::H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Header = sp_runtime::testing::Header;
	type Event = Event;
	type BlockHashCount = ();
	type MaximumBlockWeight = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = ();
	type MaximumBlockLength = ();
	type AvailableBlockRatio = ();
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
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = ();
	type OtherCurrencies = ();
	type WeightInfo = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

pub struct RelayerGameAdjustor;
impl AdjustableRelayerGame for RelayerGameAdjustor {
	type Moment = BlockNumber;
	type Balance = Balance;
	type RelayBlockId = MockRelayBlockNumber;

	fn max_active_games() -> u8 {
		32
	}

	fn propose_time(round: u32) -> Self::Moment {
		CHALLENGE_TIME.with(|v| v.borrow().to_owned()) / 2
	}

	fn complete_proofs_time(round: u32) -> Self::Moment {
		CHALLENGE_TIME.with(|v| v.borrow().to_owned()) / 2
	}

	fn update_sample_points(sample_points: &mut Vec<Vec<Self::RelayBlockId>>) {
		sample_points.push(vec![sample_points.last().unwrap().last().unwrap() - 1]);
	}

	fn estimate_bond(_round: u32, _proposals_count: u8) -> Self::Balance {
		ESTIMATE_BOND.with(|v| v.borrow().to_owned())
	}
}

pub struct ExtBuilder {
	headers: Vec<MockRelayHeader>,
	challenge_time: BlockNumber,
	estimate_bond: Balance,
	confirmed_period: BlockNumber,
}
impl ExtBuilder {
	pub fn headers(mut self, headers: Vec<MockRelayHeader>) -> Self {
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
			challenge_time: RelayerGameAdjustor::propose_time(0)
				+ RelayerGameAdjustor::complete_proofs_time(0),
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

pub(crate) fn relayer_game_events() -> Vec<crate::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let Event::relayer_game(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect()
}
