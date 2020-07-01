mod mock_relay {
	// --- substrate ---
	use sp_runtime::DispatchResult;
	// --- darwinia ---
	use crate::{mock::*, *};
	use darwinia_support::relay::Relayable;

	pub type MockTcBlockNumber = u64;
	pub type MockTcHeaderHash = u64;

	decl_storage! {
		trait Store for Module<T: Trait> as DarwiniaRelay {
			pub BestHeader get(fn best_header): (MockTcBlockNumber, MockTcHeaderHash);

			pub Headers
				get(fn header_of_hash)
				: map hasher(identity) MockTcHeaderHash
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
		type TcHeaderHash = MockTcHeaderHash;
		type TcHeaderMMR = ();

		fn best_block_number() -> Self::TcBlockNumber {
			Self::best_header().0
		}

		fn header_existed(block_number: Self::TcBlockNumber) -> bool {
			Self::best_block_number() <= block_number
		}

		fn verify_raw_header_thing(
			raw_header_thing: RawHeaderThing,
		) -> Result<
			TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
			sp_runtime::DispatchError,
		> {
			let header: MockTcHeader =
				Decode::decode(&mut &raw_header_thing[..]).map_err(|_| "Decode - FAILED")?;
			let verify = |header: &MockTcHeader| -> DispatchResult {
				ensure!(header.valid, "Header Hash - MISMATCHED");

				Ok(())
			};

			verify(&header)?;

			Ok(TcHeaderBrief {
				block_number: header.number,
				hash: header.hash,
				parent_hash: header.parent_hash,
				mmr: (),
				others: vec![],
			})
		}

		fn on_chain_arbitrate(
			mut header_briefs_chain: Vec<
				TcHeaderBrief<Self::TcBlockNumber, Self::TcHeaderHash, Self::TcHeaderMMR>,
			>,
		) -> sp_runtime::DispatchResult {
			let best_header = Self::best_header();

			if header_briefs_chain.len() == 1 {
				ensure!(
					header_briefs_chain[0].block_number + 1 == best_header.0,
					"Previous Block Number - MISMATCHED"
				);
				ensure!(
					header_briefs_chain[0].hash == best_header.1,
					"Previous Hash - MISMATCHED"
				);

				return Ok(());
			}

			header_briefs_chain.sort_by_key(|header_briefs| header_briefs.block_number);

			{
				let last_header_briefs = header_briefs_chain.last().unwrap();
				ensure!(
					best_header.0 + 1 == last_header_briefs.block_number,
					"Previous Block Number - MISMATCHED"
				);
				ensure!(
					best_header.1 == last_header_briefs.parent_hash,
					"Previous Hash - MISMATCHED"
				);
			}

			let mut prev_header_briefs = &header_briefs_chain[0];

			for header_briefs in &header_briefs_chain[1..] {
				ensure!(
					prev_header_briefs.block_number + 1 == header_briefs.block_number,
					"Previous Block Number - MISMATCHED"
				);
				ensure!(
					prev_header_briefs.hash == header_briefs.parent_hash,
					"Previous Hash - MISMATCHED"
				);

				prev_header_briefs = header_briefs;
			}

			Ok(())
		}

		fn store_header(raw_header_thing: RawHeaderThing) -> DispatchResult {
			let header: MockTcHeader =
				Decode::decode(&mut &raw_header_thing[..]).map_err(|_| "Decode - FAILED")?;

			Headers::insert(header.hash, header);

			Ok(())
		}
	}

	#[derive(Encode, Decode)]
	pub struct MockTcHeader {
		valid: bool,

		number: MockTcBlockNumber,
		hash: MockTcHeaderHash,
		parent_hash: MockTcHeaderHash,
	}
}

// --- substrate ---
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
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

impl darwinia_balances::Trait<RingInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type DustCollector = (Kton,);
}
impl darwinia_balances::Trait<KtonInstance> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
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
		let prev = samples[0] - 1;

		samples.push(prev);
	}

	fn estimate_bond(_round: Round, _proposals_count: u64) -> Self::Balance {
		1
	}
}
