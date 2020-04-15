//! prototype module for bridging in ethereum pow blockchain, including mainnet and ropsten.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

//#[cfg(test)]
//mod mock;
//#[cfg(test)]
//mod tests;
#[cfg(test)]
mod mock_mainnet;
#[cfg(test)]
mod tests_mainnet;

// --- crates ---
use codec::{Decode, Encode};
use ethereum_types::{H128, H512, H64};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get,
	weights::SimpleDispatchInfo,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_io::hashing::sha2_256;
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};
use sp_std::prelude::*;
// --- darwinia ---

use eth_primitives::pow::EthashSeal;
use eth_primitives::{
	header::EthHeader, pow::EthashPartial, receipt::Receipt, EthBlockNumber, H256, U256,
};

use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type EthNetwork: Get<EthNetworkType>;
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

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct DoubleNodeWithMerkleProof {
	pub dag_nodes: Vec<H512>, // [H512; 2]
	pub proof: Vec<H128>,
}

impl DoubleNodeWithMerkleProof {
	fn truncate_to_h128(arr: H256) -> H128 {
		let mut data = [0u8; 16];
		data.copy_from_slice(&(arr.0)[16..]);
		H128(data.into())
	}

	fn hash_h128(l: H128, r: H128) -> H128 {
		let mut data = [0u8; 64];
		data[16..32].copy_from_slice(&(l.0));
		data[48..64].copy_from_slice(&(r.0));
		Self::truncate_to_h128(sha2_256(&data).into())
	}

	pub fn apply_merkle_proof(&self, index: u64) -> H128 {
		let mut data = [0u8; 128];
		data[..64].copy_from_slice(&(self.dag_nodes[0].0));
		data[64..].copy_from_slice(&(self.dag_nodes[1].0));

		let mut leaf = Self::truncate_to_h128(sha2_256(&data).into());

		for i in 0..self.proof.len() {
			if (index >> i as u64) % 2 == 0 {
				leaf = Self::hash_h128(leaf, self.proof[i]);
			} else {
				leaf = Self::hash_h128(self.proof[i], leaf);
			}
		}
		leaf
	}
}

/// Familial details concerning a block
#[derive(Clone, Default, PartialEq, Encode, Decode)]
pub struct HeaderInfo {
	/// Total difficulty of the block and all its parents
	pub total_difficulty: U256,
	/// Parent hash of the header
	pub parent_hash: H256,
	/// Block number
	pub number: EthBlockNumber,
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

		pub CanonicalHeaderHashOf get(fn canonical_header_hash_of): map hasher(identity) u64 => H256;

		pub HeaderOf get(fn header_of): map hasher(identity) H256 => Option<EthHeader>;

		pub HeaderInfoOf get(fn header_info_of): map hasher(identity) H256 => Option<HeaderInfo>;

		/// Number of blocks finality
		pub NumberOfBlocksFinality get(fn number_of_blocks_finality) config(): u64;
		pub NumberOfBlocksSafe get(fn number_of_blocks_safe) config(): u64;

		pub CheckAuthorities get(fn check_authorities) config(): bool = true;
		pub Authorities get(fn authorities) config(): Vec<T::AccountId>;
	}
	add_extra_genesis {
		// genesis: Option<Header, Difficulty>
		config(genesis_header): Option<(u64, Vec<u8>)>;
		config(dag_merkle_roots): Vec<H128>;
		build(|config| {
			if let Some((difficulty, header)) = &config.genesis_header {
				let header: EthHeader = rlp::decode(&header).expect(<Error<T>>::RlpDcF.into());
				<Module<T>>::init_genesis_header(&header, *difficulty).expect(<Error<T>>::GenesisHeaderIF.into());
			}

			for i in 0..config.dag_merkle_roots.len() {
				<DagsMerkleRoots>::insert(i as u64, config.dag_merkle_roots[i]);
			}
		});
	}
}

decl_event! {
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId
	{
		SetGenesisHeader(EthHeader, u64),
		RelayHeader(AccountId, EthHeader),
		VerifyProof(AccountId, Receipt, EthReceiptProof),
		AddAuthority(AccountId),
		RemoveAuthority(AccountId),
		ToggleCheckAuthorities(bool),
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

		/// Block Number - MISMATCHED
		BlockNumberMis,
		/// Header Hash - MISMATCHED
		HeaderHashMis,
		/// Mixhash - MISMATCHED
		MixHashMis,

		/// Begin Header - NOT EXISTED
		BeginHeaderNE,
		/// Header - NOT EXISTED
		HeaderNE,
		/// Header Info - NOT EXISTED
		HeaderInfoNE,
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
		/// Genesis Header - INITIALIZATION FAILED
		GenesisHeaderIF,
		/// Seal - PARSING FAILED
		SealPF,
		/// Block Basic - VERIFICATION FAILED
		BlockBasicVF,
		/// Difficulty - VERIFICATION FAILED
		DifficultyVF,
		/// Proof - VERIFICATION FAILED
		ProofVF,
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

		#[weight = SimpleDispatchInfo::FixedNormal(100_000)]
		pub fn reset_genesis_header(origin, header: EthHeader, genesis_difficulty: u64) {
			let _ = ensure_root(origin)?;

			Self::init_genesis_header(&header, genesis_difficulty)?;

			<Module<T>>::deposit_event(RawEvent::SetGenesisHeader(header, genesis_difficulty));
		}

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
		#[weight = SimpleDispatchInfo::FixedNormal(200_000)]
		pub fn relay_header(origin, header: EthHeader, dag_nodes: Vec<DoubleNodeWithMerkleProof>) {
			frame_support::debug::trace!(target: "er-rl", "{:?}", header);
			let relayer = ensure_signed(origin)?;

			if Self::check_authorities() {
				ensure!(Self::authorities().contains(&relayer), <Error<T>>::AccountNP);
			}

			let header_hash = header.hash();

			ensure!(HeaderInfoOf::get(&header_hash).is_none(), <Error<T>>::HeaderAE);

//			let best_header_hash = Self::best_header_hash();
//			if self.best_header_hash == Default::default() {
//				Self::maybe_store_header(&header)?;
//			}

			Self::verify_header(&header, &dag_nodes)?;
			Self::maybe_store_header(&header)?;

			<Module<T>>::deposit_event(RawEvent::RelayHeader(relayer, header));
		}

		/// Check receipt
		///
		/// # <weight>
		/// - `O(1)`.
		/// - Limited Storage reads
		/// - Up to one event
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(100_000)]
		pub fn check_receipt(origin, proof_record: EthReceiptProof) {
			let relayer = ensure_signed(origin)?;
			if Self::check_authorities() {
				ensure!(Self::authorities().contains(&relayer), <Error<T>>::AccountNP);
			}

			let verified_receipt = Self::verify_receipt(&proof_record)?;

			<Module<T>>::deposit_event(RawEvent::VerifyProof(relayer, verified_receipt, proof_record));
		}

		// --- root call ---

		/// Add authority
		///
		/// # <weight>
		/// - `O(A)` where `A` length of `authorities`
		/// - One storage mutation (codec `O(A)`).
		/// - Up to one event
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
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
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
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
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn toggle_check_authorities(origin) {
			ensure_root(origin)?;

			CheckAuthorities::put(!Self::check_authorities());

			<Module<T>>::deposit_event(RawEvent::ToggleCheckAuthorities(Self::check_authorities()));
		}

		/// Set number of blocks finality
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn set_number_of_blocks_finality(origin, #[compact] new: u64) {
			ensure_root(origin)?;
			NumberOfBlocksFinality::put(new);
		}

		/// Set number of blocks finality
		///
		/// # <weight>
		/// - `O(1)`.
		/// - One storage write
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn set_number_of_blocks_safe(origin, #[compact] new: u64) {
			ensure_root(origin)?;
			NumberOfBlocksSafe::put(new);
		}

	}
}

impl<T: Trait> Module<T> {
	pub fn init_genesis_header(header: &EthHeader, genesis_difficulty: u64) -> DispatchResult {
		let header_hash = header.hash();

		ensure!(
			header_hash == header.re_compute_hash(),
			<Error<T>>::HeaderHashMis
		);

		let block_number = header.number;

		HeaderOf::insert(&header_hash, header);

		// initialize header info, including total difficulty.
		HeaderInfoOf::insert(
			&header_hash,
			HeaderInfo {
				parent_hash: header.parent_hash,
				total_difficulty: genesis_difficulty.into(),
				number: block_number,
			},
		);

		// Initialize the the best hash.
		BestHeaderHash::put(header_hash);

		CanonicalHeaderHashOf::insert(block_number, header_hash);

		// Removing headers with larger numbers, if there are.
		for number in block_number
			.checked_add(1)
			.ok_or(<Error<T>>::BlockNumberOF)?..u64::max_value()
		{
			// If the current block hash is 0 (unlikely), or the previous hash matches the
			// current hash, then we chains converged and can stop now.
			if !CanonicalHeaderHashOf::contains_key(&number) {
				break;
			}

			CanonicalHeaderHashOf::remove(&number);
		}

		GenesisHeader::put(header.clone());

		Ok(())
	}

	/// 1. proof of difficulty
	/// 2. proof of pow (mixhash)
	/// 3. challenge
	fn verify_header(
		header: &EthHeader,
		dag_nodes: &[DoubleNodeWithMerkleProof],
	) -> DispatchResult {
		ensure!(
			header.hash() == header.re_compute_hash(),
			<Error<T>>::HeaderHashMis
		);
		frame_support::debug::trace!(target: "er-rl", "Hash OK");

		let begin_header_number = Self::begin_header()
			.ok_or(<Error<T>>::BeginHeaderNE)?
			.number;
		ensure!(header.number >= begin_header_number, <Error<T>>::HeaderTE);
		frame_support::debug::trace!(target: "er-rl", "Number1 OK");

		// There must be a corresponding parent hash
		let prev_header = Self::header_of(header.parent_hash).ok_or(<Error<T>>::HeaderNE)?;
		ensure!(
			header.number
				== prev_header
					.number
					.checked_add(1)
					.ok_or(<Error<T>>::BlockNumberOF)?,
			<Error<T>>::BlockNumberMis,
		);
		frame_support::debug::trace!(target: "er-rl", "Number2 OK");

		// check difficulty
		let ethash_params = match T::EthNetwork::get() {
			EthNetworkType::Mainnet => EthashPartial::production(),
			EthNetworkType::Ropsten => EthashPartial::ropsten_testnet(),
		};
		ethash_params
			.verify_block_basic(header)
			.map_err(|_| <Error<T>>::BlockBasicVF)?;
		frame_support::debug::trace!(target: "er-rl", "Basic OK");

		// verify difficulty
		let difficulty = ethash_params.calculate_difficulty(header, &prev_header);
		ensure!(difficulty == *header.difficulty(), <Error<T>>::DifficultyVF);
		frame_support::debug::trace!(target: "er-rl", "Difficulty OK");

		let seal = EthashSeal::parse_seal(header.seal()).map_err(|_| <Error<T>>::SealPF)?;
		frame_support::debug::trace!(target: "er-rl", "Seal OK");

		let partial_header_hash = header.bare_hash();

		let (mix_hash, _result) =
			Self::hashimoto_merkle(&partial_header_hash, &seal.nonce, header.number, dag_nodes);

		#[cfg(feature = "std")]
		println!("{:?}", mix_hash);

		ensure!(mix_hash == seal.mix_hash, <Error<T>>::MixHashMis);
		frame_support::debug::trace!(target: "er-rl", "MixHash OK");

		// TODO:
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

	fn maybe_store_header(header: &EthHeader) -> DispatchResult {
		let best_header_info =
			Self::header_info_of(Self::best_header_hash()).ok_or(<Error<T>>::HeaderInfoNE)?;

		ensure!(
			best_header_info.number
				<= header
					.number
					.checked_add(Self::number_of_blocks_finality())
					.ok_or(<Error<T>>::BlockNumberOF)?,
			<Error<T>>::HeaderTO,
		);

		let parent_total_difficulty = Self::header_info_of(header.parent_hash)
			.ok_or(<Error<T>>::HeaderInfoNE)?
			.total_difficulty;

		let header_hash = header.hash();
		let header_info = HeaderInfo {
			number: header.number,
			parent_hash: header.parent_hash,
			total_difficulty: parent_total_difficulty
				.checked_add(header.difficulty)
				.ok_or(<Error<T>>::BlockNumberOF)?,
		};

		// Check total difficulty and re-org if necessary.
		if header_info.total_difficulty > best_header_info.total_difficulty
			|| (header_info.total_difficulty == best_header_info.total_difficulty
				&& header.difficulty % 2 == U256::zero())
		{
			// The new header is the tip of the new canonical chain.
			// We need to update hashes of the canonical chain to match the new header.

			// If the new header has a lower number than the previous header, we need to cleaning
			// it going forward.
			if best_header_info.number > header_info.number {
				for number in header_info
					.number
					.checked_add(1)
					.ok_or(<Error<T>>::BlockNumberOF)?..=best_header_info.number
				{
					CanonicalHeaderHashOf::remove(&number);
				}
			}
			// Replacing the global best header hash.
			BestHeaderHash::put(header_hash);

			CanonicalHeaderHashOf::insert(header_info.number, header_hash);

			// Replacing past hashes until we converge into the same parent.
			// Starting from the parent hash.
			let mut current_hash = header_info.parent_hash;
			for number in (0..=header
				.number
				.checked_sub(1)
				.ok_or(<Error<T>>::BlockNumberUF)?)
				.rev()
			{
				let prev_value = CanonicalHeaderHashOf::get(number);
				// If the current block hash is 0 (unlikely), or the previous hash matches the
				// current hash, then we chains converged and can stop now.
				if number == 0 || prev_value == current_hash {
					break;
				}

				CanonicalHeaderHashOf::insert(number, current_hash);

				// Check if there is an info to get the parent hash
				if let Some(info) = HeaderInfoOf::get(current_hash) {
					current_hash = info.parent_hash;
				} else {
					break;
				}
			}
		}

		HeaderOf::insert(header_hash, header);
		HeaderInfoOf::insert(header_hash, header_info.clone());

		Ok(())
	}

	// FXIME: Check the nodes to avoid panics in the hashimoto.
	fn hashimoto_merkle(
		header_hash: &H256,
		nonce: &H64,
		block_number: u64,
		nodes: &[DoubleNodeWithMerkleProof],
	) -> (H256, H256) {
		// Boxed index since ethash::hashimoto gets Fn, but not FnMut
		let index = sp_std::cell::RefCell::new(0);

		// Reuse single Merkle root across all the proofs
		let merkle_root = Self::dag_merkle_root((block_number as usize / 30000) as u64);

		let pair = ethash::hashimoto(
			header_hash.clone(),
			nonce.clone(),
			ethash::get_full_size(block_number as usize / 30000),
			|offset| {
				let idx = *index.borrow_mut();
				*index.borrow_mut() += 1;

				// FIXME: Temp workaround to avoid panic
				if idx / 2 >= nodes.len() {
					return Default::default();
				}

				// Each two nodes are packed into single 128 bytes with Merkle proof
				let node = &nodes[idx / 2];
				if idx % 2 == 0 {
					// Divide by 2 to adjust offset for 64-byte words instead of 128-byte
					//					assert_eq!(merkle_root, node.apply_merkle_proof((offset / 2) as u64));
					//					 FIXME: Temp workaround to avoid panic
					if merkle_root != node.apply_merkle_proof((offset / 2) as u64) {
						return Default::default();
					}
				};

				// FIXME: Temp workaround to avoid panic
				if idx % 2 >= node.dag_nodes.len() {
					return Default::default();
				}

				// Reverse each 32 bytes for ETHASH compatibility
				let mut data = node.dag_nodes[idx % 2].0;
				data[..32].reverse();
				data[32..].reverse();
				data.into()
			},
		);

		pair
	}
}

/// Handler for selecting the genesis validator set.
pub trait VerifyEthReceipts {
	fn verify_receipt(proof_record: &EthReceiptProof) -> Result<Receipt, DispatchError>;
}

impl<T: Trait> VerifyEthReceipts for Module<T> {
	/// confirm that the block hash is right
	/// get the receipt MPT trie root from the block header
	/// Using receipt MPT trie root to verify the proof and index etc.
	fn verify_receipt(proof_record: &EthReceiptProof) -> Result<Receipt, DispatchError> {
		let info =
			Self::header_info_of(&proof_record.header_hash).ok_or(<Error<T>>::HeaderInfoNE)?;

		let canonical_hash = Self::canonical_header_hash_of(info.number);
		ensure!(
			canonical_hash == proof_record.header_hash,
			<Error<T>>::HeaderNC
		);

		let best_info =
			Self::header_info_of(Self::best_header_hash()).ok_or(<Error<T>>::HeaderInfoNE)?;

		ensure!(
			best_info.number
				>= info
					.number
					.checked_add(Self::number_of_blocks_safe())
					.ok_or(<Error<T>>::BlockNumberOF)?,
			<Error<T>>::HeaderNS,
		);

		let header = Self::header_of(&proof_record.header_hash).ok_or(<Error<T>>::HeaderNE)?;
		let proof: Proof = rlp::decode(&proof_record.proof).map_err(|_| <Error<T>>::RlpDcF)?;
		let key = rlp::encode(&proof_record.index);
		let value =
			MerklePatriciaTrie::verify_proof(header.receipts_root().0.to_vec(), &key, proof)
				.map_err(|_| <Error<T>>::ProofVF)?
				.ok_or(<Error<T>>::TrieKeyNE)?;
		let receipt = rlp::decode(&value).map_err(|_| <Error<T>>::ReceiptDsF)?;

		Ok(receipt)
	}
}
