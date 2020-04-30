//! # Darwinia-eth-relay Module
//!
//! Prototype module for bridging in Ethereum pow blockchain, including Mainnet and Ropsten.
//!
//! ## Overview
//!
//! The darwinia eth relay module itself is a chain relay targeting Ethereum networks to
//! Darwinia networks. This module follows the basic linear chain relay design which
//! requires relayers to relay the headers one by one.
//!
//! ### Relayer Incentive Model
//!
//! There is a points pool recording contribution of relayers, for each finalized and
//! relayed block header, the relayer(origin) will get one unit of contribution point.
//! The income of the points pool come from two parts:
//! 	- The first part comes from clients who use chain relay to verify receipts, they
//!       might need to pay for the check_receipt service, although currently the chain
//!       relay didn't charge service fees, but in future, customers module/call should
//!       pay for this.
//!     - The second part comes from the compensate budget/proposal from system or governance,
//!       for example, someone may submit a proposal from treasury module to compensate the
//!       relay module account (points pool).
//!
//! The points owners can claim their incomes any time(block), the income is calculated according
//! to his points proportion of total points, and after paying to him, the points will be destroyed
//! from the points pool.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type Balance<T> = <CurrencyT<T> as Currency<AccountId<T>>>::Balance;

	type AccountId<T> = <T as system::Trait>::AccountId;

	type CurrencyT<T> = <T as Trait>::Currency;
}

// --- crates ---
use codec::{Decode, Encode};
use ethereum_types::{H128, H512, H64};
// --- substrate ---
use frame_support::{
	debug::trace,
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::Get,
	traits::{Currency, ExistenceRequirement::KeepAlive, ReservableCurrency},
	IsSubType,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_io::hashing::sha2_256;
use sp_runtime::{
	traits::{AccountIdConversion, DispatchInfoOf, Dispatchable, Saturating, SignedExtension},
	transaction_validity::{
		InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
	},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug, SaturatedConversion,
};
use sp_std::{cell::RefCell, prelude::*};
// --- darwinia ---
use darwinia_support::bytes_thing::{array_unchecked, fixed_hex_bytes_unchecked};
use eth_primitives::{
	header::EthHeader,
	pow::{EthashPartial, EthashSeal},
	receipt::Receipt,
	EthBlockNumber, H256, U256,
};
use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};
use types::*;

pub trait Trait: frame_system::Trait {
	/// The eth-relay's module id, used for deriving its sovereign account ID.
	type ModuleId: Get<ModuleId>;

	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type EthNetwork: Get<EthNetworkType>;

	type Call: Dispatchable + From<Call<Self>> + IsSubType<Module<Self>, Self> + Clone;

	type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
}

#[derive(Clone, PartialEq, Encode, Decode)]
pub enum EthNetworkType {
	Mainnet,
	Ropsten,
}

impl Default for EthNetworkType {
	fn default() -> EthNetworkType {
		EthNetworkType::Mainnet
	}
}

#[cfg(feature = "std")]
darwinia_support::impl_genesis! {
	struct DagMerkleRoots {
		dag_merkle_roots: Vec<H128>
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct DoubleNodeWithMerkleProof {
	pub dag_nodes: [H512; 2],
	pub proof: Vec<H128>,
}

impl Default for DoubleNodeWithMerkleProof {
	fn default() -> DoubleNodeWithMerkleProof {
		DoubleNodeWithMerkleProof {
			dag_nodes: <[H512; 2]>::default(),
			proof: Vec::new(),
		}
	}
}

impl DoubleNodeWithMerkleProof {
	pub fn from_str_unchecked(s: &str) -> Self {
		let mut dag_nodes: Vec<H512> = Vec::new();
		let mut proof: Vec<H128> = Vec::new();
		for e in s.splitn(60, '"') {
			let l = e.len();
			if l == 34 {
				proof.push(fixed_hex_bytes_unchecked!(e, 16).into());
			} else if l == 130 {
				dag_nodes.push(fixed_hex_bytes_unchecked!(e, 64).into());
			} else if l > 34 {
				// should not be here
				panic!("the proofs are longer than 25");
			}
		}
		DoubleNodeWithMerkleProof {
			dag_nodes: [dag_nodes[0], dag_nodes[1]],
			proof,
		}
	}
}

impl DoubleNodeWithMerkleProof {
	pub fn apply_merkle_proof(&self, index: u64) -> H128 {
		fn hash_h128(l: H128, r: H128) -> H128 {
			let mut data = [0u8; 64];
			data[16..32].copy_from_slice(&(l.0));
			data[48..64].copy_from_slice(&(r.0));

			// `H256` is 32 length, truncate is safe; qed
			array_unchecked!(sha2_256(&data), 16, 16).into()
		}

		let mut data = [0u8; 128];
		data[..64].copy_from_slice(&(self.dag_nodes[0].0));
		data[64..].copy_from_slice(&(self.dag_nodes[1].0));

		// `H256` is 32 length, truncate is safe; qed
		let mut leaf = array_unchecked!(sha2_256(&data), 16, 16).into();
		for i in 0..self.proof.len() {
			if (index >> i as u64) % 2 == 0 {
				leaf = hash_h128(leaf, self.proof[i]);
			} else {
				leaf = hash_h128(self.proof[i], leaf);
			}
		}

		leaf
	}
}

/// Familial details concerning a block
#[derive(Clone, Default, PartialEq, Encode, Decode)]
pub struct EthHeaderBrief<AccountId> {
	/// Total difficulty of the block and all its parents
	pub total_difficulty: U256,
	/// Parent hash of the header
	pub parent_hash: H256,
	/// Block number
	pub number: EthBlockNumber,
	/// Relayer of the block header
	pub relayer: AccountId,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthReceiptProof {
	pub index: u64,
	pub proof: Vec<u8>,
	pub header_hash: H256,
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEthRelay {
		/// Anchor block that works as genesis block
		pub GenesisHeader get(fn begin_header): Option<EthHeader>;

		/// Dags merkle roots of ethereum epoch (each epoch is 30000)
		pub DagsMerkleRoots get(fn dag_merkle_root): map hasher(identity) u64 => H128;

		/// Hash of best block header
		pub BestHeaderHash get(fn best_header_hash): H256;

		pub CanonicalHeaderHashes get(fn canonical_header_hash): map hasher(identity) u64 => H256;

		pub Headers get(fn header): map hasher(identity) H256 => Option<EthHeader>;
		pub HeaderBriefs get(fn header_brief): map hasher(identity) H256 => Option<EthHeaderBrief::<T::AccountId>>;

		/// Number of blocks finality
		pub NumberOfBlocksFinality get(fn number_of_blocks_finality) config(): u64;
		pub NumberOfBlocksSafe get(fn number_of_blocks_safe) config(): u64;

		pub CheckAuthority get(fn check_authority) config(): bool = true;
		pub Authorities get(fn authorities) config(): Vec<T::AccountId>;

		pub ReceiptVerifyFee get(fn receipt_verify_fee) config(): Balance<T>;

		pub RelayerPoints get(fn relayer_points): map hasher(blake2_128_concat) T::AccountId => u64;
		pub TotalRelayerPoints get(fn total_points): u64 = 0;
	}
	add_extra_genesis {
		// genesis: Option<Header, Difficulty>
		config(genesis_header): Option<(u64, Vec<u8>)>;
		config(dag_merkle_roots): DagMerkleRoots;
		build(|config| {
			let GenesisConfig {
				genesis_header,
				dag_merkle_roots,
				..
			} = config;

			if let Some((total_difficulty, header)) = genesis_header {
				if let Ok(header) = rlp::decode(&header) {
					<Module<T>>::init_genesis_header(&header, *total_difficulty).unwrap();
				} else {
					panic!(<&str>::from(<Error<T>>::RlpDcF));
				}
			}

			for (i, dag_merkle_root) in dag_merkle_roots
				.dag_merkle_roots
				.iter()
				.cloned()
				.enumerate()
			{
				DagsMerkleRoots::insert(i as u64, dag_merkle_root);
			}

			let _ = T::Currency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::Currency::minimum_balance(),
			);
		});
	}
}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
		Balance = Balance<T>,
	{
		SetGenesisHeader(EthHeader, u64),
		RelayHeader(AccountId, EthHeader),
		VerifyProof(AccountId, Receipt, EthReceiptProof),
		AddAuthority(AccountId),
		RemoveAuthority(AccountId),
		ToggleCheckAuthorities(bool),
		ClaimReward(AccountId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Account - NO PRIVILEGES
		AccountNP,

		/// Block Number - OVERFLOW
		BlockNumberOF,
		/// Block Number - UNDERFLOW
		BlockNumberUF,
		/// Index - OUT OF RANGE
		IndexOFR,

		/// Block Number - MISMATCHED
		BlockNumberMis,
		/// Header Hash - MISMATCHED
		HeaderHashMis,
		/// Merkle Root - MISMATCHED
		MerkleRootMis,
		/// Mixhash - MISMATCHED
		MixHashMis,

		/// Begin Header - NOT EXISTED
		BeginHeaderNE,
		/// Header - NOT EXISTED
		HeaderNE,
		/// Header Brief - NOT EXISTED
		HeaderBriefNE,
		/// Trie Key - NOT EXISTED
		TrieKeyNE,

		/// Header - ALREADY EXISTS
		HeaderAE,
		/// Header - NOT CANONICAL
		HeaderNC,
		/// Header - NOT SAFE
		HeaderNS,
		/// Header - TOO EARLY
		HeaderTE,
		/// Header - TOO OLD,
		HeaderTO,

		/// Rlp - DECODE FAILED
		RlpDcF,
		/// Receipt - DESERIALIZE FAILED
		ReceiptDsF,
		/// Seal - PARSING FAILED
		SealPF,
		/// Block Basic - VERIFICATION FAILED
		BlockBasicVF,
		/// Difficulty - VERIFICATION FAILED
		DifficultyVF,
		/// Proof - VERIFICATION FAILED
		ProofVF,

		/// Payout - NO POINTS OR FUNDS
		PayoutNPF,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		const EthNetwork: EthNetworkType = T::EthNetwork::get();

		fn deposit_event() = default;

		/// Relay header of eth block, store the passing header
		/// if it is verified.
		///
		/// # <weight>
		/// - `O(1)`, but takes a lot of computation works
		/// - Limited Storage reads
		/// - One storage read
		/// - One storage write
		/// - Up to one event
		/// # </weight>
		#[weight = 200_000_000]
		pub fn relay_header(origin, header: EthHeader, ethash_proof: Vec<DoubleNodeWithMerkleProof>) {
			trace!(target: "eth-relay", "{:?}", header);
			let relayer = ensure_signed(origin)?;

			if Self::check_authority() {
				ensure!(Self::authorities().contains(&relayer), <Error<T>>::AccountNP);
			}

			let header_hash = header.hash();

			ensure!(<HeaderBriefs<T>>::get(&header_hash).is_none(), <Error<T>>::HeaderAE);

			// 1. proof of difficulty
			// 2. proof of pow (mixhash)
			// 3. challenge
			{
				Self::verify_header_basic(&header)?;
				Self::verify_header_pow(&header, &ethash_proof)?;
			}

			Self::maybe_store_header(&relayer, &header)?;

			<Module<T>>::deposit_event(RawEvent::RelayHeader(relayer, header));
		}

		/// Check receipt
		///
		/// # <weight>
		/// - `O(1)`.
		/// - Limited Storage reads
		/// - Up to one event
		/// # </weight>
		#[weight = 100_000_000]
		pub fn check_receipt(origin, proof_record: EthReceiptProof) {
			let relayer = ensure_signed(origin)?;

			let (verified_receipt, fee) = Self::verify_receipt(&proof_record)?;

			let module_account = Self::account_id();

			T::Currency::transfer(&relayer, &module_account, fee, KeepAlive)?;

			<Module<T>>::deposit_event(RawEvent::VerifyProof(relayer, verified_receipt, proof_record));
		}

		/// Claim Reward for Relayers
		///
		/// # <weight>
		/// - `O(1)`.
		/// - Limited Storage reads
		/// - Up to one event
		/// # </weight>
		#[weight = 10_000_000]
		pub fn claim_reward(origin) {
			let relayer = ensure_signed(origin)?;

			let points = Self::relayer_points(&relayer);
			let total_points = Self::total_points();

			let max_payout = Self::pot().saturated_into();

			ensure!(total_points > 0 && points > 0 && max_payout > 0 && total_points >= points, <Error<T>>::PayoutNPF);

			let payout : Balance<T> = (points as u128 * max_payout / (total_points  as u128)).saturated_into();
			let module_account = Self::account_id();

			T::Currency::transfer(&module_account, &relayer, payout, KeepAlive)?;

			<RelayerPoints<T>>::remove(&relayer);

			TotalRelayerPoints::mutate(|p| *p -= points);

			<Module<T>>::deposit_event(RawEvent::ClaimReward(relayer, payout));
		}

		// --- root call ---

		#[weight = 100_000_000]
		pub fn reset_genesis_header(origin, header: EthHeader, genesis_difficulty: u64) {
			let _ = ensure_root(origin)?;

			Self::init_genesis_header(&header, genesis_difficulty)?;

			<Module<T>>::deposit_event(RawEvent::SetGenesisHeader(header, genesis_difficulty));
		}

		/// Add authority
		///
		/// # <weight>
		/// - `O(A)` where `A` length of `authorities`
		/// - One storage mutation (codec `O(A)`).
		/// - Up to one event
		/// # </weight>
		#[weight = 50_000_000]
		pub fn add_authority(origin, who: T::AccountId) {
			ensure_root(origin)?;

			if !Self::authorities().contains(&who) {
				<Authorities<T>>::mutate(|l| l.push(who.clone()));

				<Module<T>>::deposit_event(RawEvent::AddAuthority(who));
			}
		}

		/// Remove authority
		///
		/// # <weight>
		/// - `O(A)` where `A` length of `authorities`
		/// - One storage mutation (codec `O(A)`).
		/// - Up to one event
		/// # </weight>
		#[weight = 50_000]
		pub fn remove_authority(origin, who: T::AccountId) {
			ensure_root(origin)?;

			if let Some(i) = Self::authorities()
				.into_iter()
				.position(|who_| who_ == who) {
				<Authorities<T>>::mutate(|l| l.remove(i));

				<Module<T>>::deposit_event(RawEvent::RemoveAuthority(who));
			}
		}

		/// Check authorities
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// - Up to one event
		/// # </weight>
		#[weight = 10_000_000]
		pub fn toggle_check_authorities(origin) {
			ensure_root(origin)?;

			CheckAuthority::put(!Self::check_authority());

			<Module<T>>::deposit_event(RawEvent::ToggleCheckAuthorities(Self::check_authority()));
		}

		/// Set number of blocks finality
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_number_of_blocks_finality(origin, #[compact] new: u64) {
			ensure_root(origin)?;

			let old_number = NumberOfBlocksFinality::get();
			let best_header_info = Self::header_brief(Self::best_header_hash()).ok_or(<Error<T>>::HeaderBriefNE);
			if new < old_number && best_header_info.is_ok() {
				let best_header_info_number = best_header_info.unwrap().number;

				for i in 0..(old_number - new) {
					// Adding reward points to the relayer of finalized block
					if best_header_info_number > Self::number_of_blocks_finality() + i + 1 {
						let finalized_block_number = best_header_info_number - Self::number_of_blocks_finality() - i - 1;
						let finalized_block_hash = CanonicalHeaderHashes::get(finalized_block_number);
						if let Some(info) = <HeaderBriefs<T>>::get(finalized_block_hash) {
							let points: u64 = Self::relayer_points(&info.relayer);

							<RelayerPoints<T>>::insert(info.relayer, points + 1);

							TotalRelayerPoints::put(Self::total_points() + 1);
						}
					}
				}
			} else {
				// Finality interval becomes larger, some points might already been claimed.
				// But we just ignore possible double claimed in future here.
			}

			NumberOfBlocksFinality::put(new);
		}

		/// Set number of blocks finality
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_number_of_blocks_safe(origin, #[compact] new: u64) {
			ensure_root(origin)?;
			NumberOfBlocksSafe::put(new);
		}

		/// Set verify receipt fee
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = 10_000_000]
		pub fn set_receipt_verify_fee(origin, #[compact] new: Balance<T>) {
			ensure_root(origin)?;
			<ReceiptVerifyFee<T>>::put(new);
		}

	}
}

impl<T: Trait> Module<T> {
	pub fn init_genesis_header(
		header: &EthHeader,
		genesis_total_difficulty: u64,
	) -> DispatchResult {
		let header_hash = header.hash();

		ensure!(
			header_hash == header.re_compute_hash(),
			<Error<T>>::HeaderHashMis
		);

		let block_number = header.number;

		Headers::insert(&header_hash, header);

		// initialize header info, including total difficulty.
		<HeaderBriefs<T>>::insert(
			&header_hash,
			EthHeaderBrief::<T::AccountId> {
				parent_hash: header.parent_hash,
				total_difficulty: genesis_total_difficulty.into(),
				number: block_number,
				relayer: Default::default(),
			},
		);

		// Initialize the the best hash.
		BestHeaderHash::put(header_hash);

		CanonicalHeaderHashes::insert(block_number, header_hash);

		// Removing header with larger numbers, if there are.
		for number in block_number
			.checked_add(1)
			.ok_or(<Error<T>>::BlockNumberOF)?..u64::max_value()
		{
			// If the current block hash is 0 (unlikely), or the previous hash matches the
			// current hash, then we chains converged and can stop now.
			if !CanonicalHeaderHashes::contains_key(&number) {
				break;
			}

			CanonicalHeaderHashes::remove(&number);
		}

		GenesisHeader::put(header.clone());

		Ok(())
	}

	fn verify_header_basic(header: &EthHeader) -> DispatchResult {
		ensure!(
			header.hash() == header.re_compute_hash(),
			<Error<T>>::HeaderHashMis
		);
		trace!(target: "eth-relay", "Hash OK");

		let begin_header_number = Self::begin_header()
			.ok_or(<Error<T>>::BeginHeaderNE)?
			.number;
		ensure!(header.number >= begin_header_number, <Error<T>>::HeaderTE);
		trace!(target: "eth-relay", "Number1 OK");

		// There must be a corresponding parent hash
		let prev_header = Self::header(header.parent_hash).ok_or(<Error<T>>::HeaderNE)?;
		// block number was verified in `re_compute_hash`,`u64` is enough; qed
		ensure!(
			header.number == prev_header.number + 1,
			<Error<T>>::BlockNumberMis
		);
		trace!(target: "eth-relay", "Number2 OK");

		// check difficulty
		let ethash_params = match T::EthNetwork::get() {
			EthNetworkType::Mainnet => EthashPartial::production(),
			EthNetworkType::Ropsten => EthashPartial::ropsten_testnet(),
		};
		ethash_params
			.verify_block_basic(header)
			.map_err(|_| <Error<T>>::BlockBasicVF)?;
		trace!(target: "eth-relay", "Basic OK");

		// verify difficulty
		let difficulty = ethash_params.calculate_difficulty(header, &prev_header);
		ensure!(difficulty == *header.difficulty(), <Error<T>>::DifficultyVF);
		trace!(target: "eth-relay", "Difficulty OK");

		Ok(())
	}

	fn verify_header_pow(
		header: &EthHeader,
		ethash_proof: &[DoubleNodeWithMerkleProof],
	) -> DispatchResult {
		Self::verify_header_basic(&header)?;

		let seal = EthashSeal::parse_seal(header.seal()).map_err(|_| <Error<T>>::SealPF)?;
		trace!(target: "eth-relay", "Seal OK");

		let partial_header_hash = header.bare_hash();

		let (mix_hash, _result) = Self::hashimoto_merkle(
			&partial_header_hash,
			&seal.nonce,
			header.number,
			ethash_proof,
		)?;

		ensure!(mix_hash == seal.mix_hash, <Error<T>>::MixHashMis);
		trace!(target: "eth-relay", "MixHash OK");

		// TODO: Check other verification condition
		// See YellowPaper formula (50) in section 4.3.4
		// 1. Simplified difficulty check to conform adjusting difficulty bomb
		// 2. Added condition: header.parent_hash() == prev.hash()
		//
		//			ethereum_types::U256::from((result.0).0) < ethash::cross_boundary(header.difficulty.0)
		//				&& (
		//				!self.validate_ethash
		//					|| (
		//					header.difficulty < header.difficulty * 101 / 100
		//						&& header.difficulty > header.difficulty * 99 / 100
		//				)
		//			)
		//				&& header.gas_used <= header.gas_limit
		//				&& header.gas_limit < prev.gas_limit * 1025 / 1024
		//				&& header.gas_limit > prev.gas_limit * 1023 / 1024
		//				&& header.gas_limit >= U256(5000.into())
		//				&& header.timestamp > prev.timestamp
		//				&& header.number == prev.number + 1
		//				&& header.parent_hash == prev.hash.unwrap()

		Ok(())
	}

	fn hashimoto_merkle(
		header_hash: &H256,
		nonce: &H64,
		block_number: u64,
		nodes: &[DoubleNodeWithMerkleProof],
	) -> Result<(H256, H256), DispatchError> {
		// Boxed index since ethash::hashimoto gets Fn, but not FnMut
		let index = RefCell::new(0);
		let err = RefCell::new(0u8);

		// Reuse single Merkle root across all the proofs
		let merkle_root = Self::dag_merkle_root((block_number as usize / 30000) as u64);

		let pair = ethash::hashimoto(
			header_hash.clone(),
			nonce.clone(),
			ethash::get_full_size(block_number as usize / 30000),
			|offset| {
				if *err.borrow() != 0 {
					return Default::default();
				}

				let index = index.replace_with(|&mut old| old + 1);

				// Each two nodes are packed into single 128 bytes with Merkle proof
				let node = if let Some(node) = nodes.get(index / 2) {
					node
				} else {
					err.replace(1);
					return Default::default();
				};

				if index % 2 == 0 {
					// Divide by 2 to adjust offset for 64-byte words instead of 128-byte
					if merkle_root != node.apply_merkle_proof((offset / 2) as u64) {
						err.replace(2);
						return Default::default();
					}
				};

				// Reverse each 32 bytes for ETHASH compatibility
				let mut data = if let Some(dag_node) = node.dag_nodes.get(index % 2) {
					dag_node.0
				} else {
					err.replace(1);
					return Default::default();
				};
				data[..32].reverse();
				data[32..].reverse();
				data.into()
			},
		);

		match err.into_inner() {
			0 => Ok(pair),
			1 => Err(<Error<T>>::IndexOFR)?,
			2 => Err(<Error<T>>::MerkleRootMis)?,
			_ => Err("unreachable".into()),
		}
	}

	fn maybe_store_header(relayer: &T::AccountId, header: &EthHeader) -> DispatchResult {
		let best_header_info =
			Self::header_brief(Self::best_header_hash()).ok_or(<Error<T>>::HeaderBriefNE)?;

		ensure!(
			best_header_info.number
				<= header
					.number
					.checked_add(Self::number_of_blocks_finality())
					.ok_or(<Error<T>>::BlockNumberOF)?,
			<Error<T>>::HeaderTO,
		);

		let parent_total_difficulty = Self::header_brief(header.parent_hash)
			.ok_or(<Error<T>>::HeaderBriefNE)?
			.total_difficulty;

		let header_hash = header.hash();
		let header_brief = EthHeaderBrief::<T::AccountId> {
			number: header.number,
			parent_hash: header.parent_hash,
			total_difficulty: parent_total_difficulty
				.checked_add(header.difficulty)
				.ok_or(<Error<T>>::BlockNumberOF)?,
			relayer: relayer.clone(),
		};

		// Check total difficulty and re-org if necessary.
		if header_brief.total_difficulty > best_header_info.total_difficulty
			|| (header_brief.total_difficulty == best_header_info.total_difficulty
				&& header.difficulty % 2 == U256::zero())
		{
			// The new header is the tip of the new canonical chain.
			// We need to update hashes of the canonical chain to match the new header.

			// If the new header has a lower number than the previous header, we need to cleaning
			// it going forward.
			if best_header_info.number > header_brief.number {
				for number in header_brief
					.number
					.checked_add(1)
					.ok_or(<Error<T>>::BlockNumberOF)?..=best_header_info.number
				{
					CanonicalHeaderHashes::remove(&number);
				}
			}
			// Replacing the global best header hash.
			BestHeaderHash::put(header_hash);

			CanonicalHeaderHashes::insert(header_brief.number, header_hash);

			// Replacing past hashes until we converge into the same parent.
			// Starting from the parent hash.
			let mut current_hash = header_brief.parent_hash;
			for number in (0..=header
				.number
				.checked_sub(1)
				.ok_or(<Error<T>>::BlockNumberUF)?)
				.rev()
			{
				let prev_value = CanonicalHeaderHashes::get(number);
				// If the current block hash is 0 (unlikely), or the previous hash matches the
				// current hash, then we chains converged and can stop now.
				if number == 0 || prev_value == current_hash {
					break;
				}

				CanonicalHeaderHashes::insert(number, current_hash);

				// Check if there is an info to get the parent hash
				if let Some(info) = <HeaderBriefs<T>>::get(current_hash) {
					current_hash = info.parent_hash;
				} else {
					break;
				}
			}
		}

		// Adding reward points to the relayer of finalized block
		if header.number > Self::number_of_blocks_finality() {
			let finalized_block_number = header.number - Self::number_of_blocks_finality() - 1;
			let finalized_block_hash = CanonicalHeaderHashes::get(finalized_block_number);
			if let Some(info) = <HeaderBriefs<T>>::get(finalized_block_hash) {
				let points: u64 = Self::relayer_points(&info.relayer);

				<RelayerPoints<T>>::insert(info.relayer, points + 1);

				TotalRelayerPoints::put(Self::total_points() + 1);
			}
		}

		Headers::insert(header_hash, header);
		<HeaderBriefs<T>>::insert(header_hash, header_brief.clone());

		Ok(())
	}

	/// Return the amount of money in the pot.
	// The existential deposit is not part of the pot so eth-relay account never gets deleted.
	fn pot() -> Balance<T> {
		T::Currency::free_balance(&Self::account_id())
			// Must never be less than 0 but better be safe.
			.saturating_sub(T::Currency::minimum_balance())
	}
}

/// Handler for selecting the genesis validator set.
pub trait VerifyEthReceipts<Balance, AccountId> {
	fn verify_receipt(proof_record: &EthReceiptProof) -> Result<(Receipt, Balance), DispatchError>;

	fn account_id() -> AccountId;
}

impl<T: Trait> VerifyEthReceipts<Balance<T>, T::AccountId> for Module<T> {
	/// confirm that the block hash is right
	/// get the receipt MPT trie root from the block header
	/// Using receipt MPT trie root to verify the proof and index etc.
	fn verify_receipt(
		proof_record: &EthReceiptProof,
	) -> Result<(Receipt, Balance<T>), DispatchError> {
		let info =
			Self::header_brief(&proof_record.header_hash).ok_or(<Error<T>>::HeaderBriefNE)?;

		let canonical_hash = Self::canonical_header_hash(info.number);
		ensure!(
			canonical_hash == proof_record.header_hash,
			<Error<T>>::HeaderNC
		);

		let best_info =
			Self::header_brief(Self::best_header_hash()).ok_or(<Error<T>>::HeaderBriefNE)?;

		ensure!(
			best_info.number
				>= info
					.number
					.checked_add(Self::number_of_blocks_safe())
					.ok_or(<Error<T>>::BlockNumberOF)?,
			<Error<T>>::HeaderNS,
		);

		let header = Self::header(&proof_record.header_hash).ok_or(<Error<T>>::HeaderNE)?;
		let proof: Proof = rlp::decode(&proof_record.proof).map_err(|_| <Error<T>>::RlpDcF)?;
		let key = rlp::encode(&proof_record.index);
		let value =
			MerklePatriciaTrie::verify_proof(header.receipts_root().0.to_vec(), &key, proof)
				.map_err(|_| <Error<T>>::ProofVF)?
				.ok_or(<Error<T>>::TrieKeyNE)?;
		let receipt = rlp::decode(&value).map_err(|_| <Error<T>>::ReceiptDsF)?;

		Ok((receipt, Self::receipt_verify_fee()))
	}

	/// The account ID of the eth relay pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}

/// `SignedExtension` that checks if a transaction has duplicate header hash to avoid coincidence
/// header between several relayers
#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct CheckEthRelayHeaderHash<T: Trait + Send + Sync>(sp_std::marker::PhantomData<T>);
impl<T: Trait + Send + Sync> Default for CheckEthRelayHeaderHash<T> {
	fn default() -> Self {
		Self(sp_std::marker::PhantomData)
	}
}
impl<T: Trait + Send + Sync> sp_std::fmt::Debug for CheckEthRelayHeaderHash<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "CheckEthRelayHeaderHash")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}
impl<T: Trait + Send + Sync> SignedExtension for CheckEthRelayHeaderHash<T> {
	const IDENTIFIER: &'static str = "CheckEthRelayHeaderHash";
	type AccountId = T::AccountId;
	type Call = <T as Trait>::Call;
	type AdditionalSigned = ();
	type Pre = ();

	fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		_who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		let call = match call.is_sub_type() {
			Some(call) => call,
			None => return Ok(ValidTransaction::default()),
		};

		match call {
			Call::relay_header(ref header, _) => {
				sp_runtime::print("check eth-relay header hash was received.");
				let header_hash = header.hash();

				if <HeaderBriefs<T>>::get(&header_hash).is_none() {
					Ok(ValidTransaction::default())
				} else {
					InvalidTransaction::Custom(<Error<T>>::HeaderAE.as_u8()).into()
				}
			}
			_ => Ok(Default::default()),
		}
	}
}
