//! Rust implementation of the Phragmén election algorithm. This is used in several pallets to
//! optimally distribute the weight of a set of voters among an elected set of candidates. In the
//! context of staking this is mapped to validators and nominators.
//!
//! The algorithm has two phases:
//!   - Sequential phragmen: performed in [`elect`] function which is first pass of the distribution
//!     The results are not optimal but the execution time is less.
//!   - Equalize post-processing: tries to further distribute the weight fairly among candidates.
//!     Incurs more execution time.
//!
//! The main objective of the assignments done by phragmen is to maximize the minimum backed
//! candidate in the elected set.
//!
//! Reference implementation: https://github.com/w3f/consensus
//! Further details:
//! https://research.web3.foundation/en/latest/polkadot/NPoS/4.%20Sequential%20Phragm%C3%A9n%E2%80%99s%20method/

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use sp_runtime::{
	helpers_128bit::multiply_by_rational,
	traits::{AtLeast32Bit, Bounded, Member, SaturatedConversion, Saturating, Zero},
	PerThing, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*};

use darwinia_support::structs::Rational64;

/// Power of an account.
pub type Power = u32;

/// A type in which performing operations on power and voters are safe.
///
/// `Power` is `u32`. Hence, `u64` is a safe type for arithmetic operations over them.
///
/// Power type converted to this is referred to as `Votes`.
pub type Votes = u64;

/// The denominator used for loads. For maximum accuracy we simply use u64;
const DEN: u64 = u64::max_value();

/// A candidate entity for phragmen election.
#[derive(Clone, Default, RuntimeDebug)]
pub struct Candidate<AccountId> {
	/// Identifier.
	pub who: AccountId,
	/// Intermediary value used to sort candidates.
	pub score: Rational64,
	/// Sum of the stake of this candidate based on received votes.
	approval_stake: Votes,
	/// Flag for being elected.
	elected: bool,
}

/// A voter entity.
#[derive(Clone, Default, RuntimeDebug)]
pub struct Voter<AccountId> {
	/// Identifier.
	who: AccountId,
	/// List of candidates proposed by this voter.
	edges: Vec<Edge<AccountId>>,
	/// The stake of this voter.
	budget: Votes,
	/// Incremented each time a candidate that this voter voted for has been elected.
	load: Rational64,
}

/// A candidate being backed by a voter.
#[derive(Clone, Default, RuntimeDebug)]
pub struct Edge<AccountId> {
	/// Identifier.
	who: AccountId,
	/// Load of this vote.
	load: Rational64,
	/// Index of the candidate stored in the 'candidates' vector.
	candidate_index: usize,
}

/// Particular `AccountId` was backed by `T`-ratio of a nominator's stake.
pub type PhragmenAssignment<AccountId, T> = (AccountId, T);

/// Particular `AccountId` was backed by `Votes` of a nominator's stake.
#[derive(RuntimeDebug)]
#[cfg_attr(
	feature = "std",
	derive(Eq, PartialEq, serde::Serialize, serde::Deserialize)
)]
pub struct PhragmenStakedAssignment<AccountId, RingBalance, KtonBalance> {
	pub account_id: AccountId,
	pub ring_balance: RingBalance,
	pub kton_balance: KtonBalance,
	pub votes: Votes,
}

/// Final result of the phragmen election.
#[derive(RuntimeDebug)]
pub struct PhragmenResult<AccountId, T: PerThing> {
	/// Just winners zipped with their approval stake. Note that the approval stake is merely the
	/// sub of their received stake and could be used for very basic sorting and approval voting.
	pub winners: Vec<(AccountId, Votes)>,
	/// Individual assignments. for each tuple, the first elements is a voter and the second
	/// is the list of candidates that it supports.
	pub assignments: Vec<(AccountId, Vec<PhragmenAssignment<AccountId, T>>)>,
}

/// A structure to demonstrate the phragmen result from the perspective of the candidate, i.e. how
/// much support each candidate is receiving.
///
/// This complements the [`PhragmenResult`] and is needed to run the equalize post-processing.
///
/// This, at the current version, resembles the `Exposure` defined in the Staking pallet, yet
/// they do not necessarily have to be the same.
#[derive(Default, RuntimeDebug)]
#[cfg_attr(
	feature = "std",
	derive(Eq, PartialEq, serde::Serialize, serde::Deserialize)
)]
pub struct Support<AccountId, RingBalance, KtonBalance> {
	/// The amount of support as the effect of self-vote.
	pub own_ring_balance: RingBalance,
	pub own_kton_balance: KtonBalance,
	pub own_votes: Votes,
	/// Total support.
	pub total_votes: Votes,
	/// Support from voters.
	pub voters: Vec<PhragmenStakedAssignment<AccountId, RingBalance, KtonBalance>>,
}

/// A linkage from a candidate and its [`Support`].
pub type SupportMap<A, R, K> = BTreeMap<A, Support<A, R, K>>;

/// Perform election based on Phragmén algorithm.
///
/// Returns an `Option` the set of winners and their detailed support ratio from each voter if
/// enough candidates are provided. Returns `None` otherwise.
///
/// * `candidate_count`: number of candidates to elect.
/// * `minimum_candidate_count`: minimum number of candidates to elect. If less candidates exist,
///   `None` is returned.
/// * `initial_candidates`: candidates list to be elected from.
/// * `initial_voters`: voters list.
///
/// This function does not strip out candidates who do not have any backing stake. It is the
/// responsibility of the caller to make sure only those candidates who have a sensible economic
/// value are passed in. From the perspective of this function, a candidate can easily be among the
/// winner with no backing stake.
pub fn elect<AccountId, R>(
	candidate_count: usize,
	minimum_candidate_count: usize,
	initial_candidates: Vec<AccountId>,
	initial_voters: Vec<(AccountId, Power, Vec<AccountId>)>,
) -> Option<PhragmenResult<AccountId, R>>
where
	AccountId: Default + Ord + Member,
	R: PerThing,
{
	let to_votes = |p: Power| p as Votes;

	// return structures
	let mut elected_candidates: Vec<(AccountId, Votes)>;
	let mut assigned: Vec<(AccountId, Vec<PhragmenAssignment<AccountId, R>>)>;

	// used to cache and access candidates index.
	let mut c_idx_cache = BTreeMap::<AccountId, usize>::new();

	// voters list.
	let num_voters = initial_candidates.len() + initial_voters.len();
	let mut voters: Vec<Voter<AccountId>> = Vec::with_capacity(num_voters);

	// Iterate once to create a cache of candidates indexes. This could be optimized by being
	// provided by the call site.
	let mut candidates = initial_candidates
		.into_iter()
		.enumerate()
		.map(|(idx, who)| {
			c_idx_cache.insert(who.clone(), idx);
			Candidate {
				who,
				..Default::default()
			}
		})
		.collect::<Vec<Candidate<AccountId>>>();

	// early return if we don't have enough candidates
	if candidates.len() < minimum_candidate_count {
		return None;
	}

	// collect voters. use `c_idx_cache` for fast access and aggregate `approval_stake` of
	// candidates.
	voters.extend(initial_voters.into_iter().map(|(who, voter_stake, votes)| {
		let mut edges: Vec<Edge<AccountId>> = Vec::with_capacity(votes.len());
		for v in votes {
			if let Some(idx) = c_idx_cache.get(&v) {
				// This candidate is valid + already cached.
				candidates[*idx].approval_stake = candidates[*idx]
					.approval_stake
					.saturating_add(to_votes(voter_stake));
				edges.push(Edge {
					who: v.clone(),
					candidate_index: *idx,
					..Default::default()
				});
			} // else {} would be wrong votes. We don't really care about it.
		}
		Voter {
			who,
			edges,
			budget: to_votes(voter_stake),
			load: Rational64::zero(),
		}
	}));

	// we have already checked that we have more candidates than minimum_candidate_count.
	// run phragmen.
	let to_elect = candidate_count.min(candidates.len());
	elected_candidates = Vec::with_capacity(candidate_count);
	assigned = Vec::with_capacity(candidate_count);

	// main election loop
	for _round in 0..to_elect {
		// loop 1: initialize score
		for c in &mut candidates {
			if !c.elected {
				// 1 / approval_stake == (DEN / approval_stake) / DEN. If approval_stake is zero,
				// then the ratio should be as large as possible, essentially `infinity`.
				if c.approval_stake.is_zero() {
					c.score = Rational64::from_unchecked(DEN, 0);
				} else {
					c.score = Rational64::from(DEN / c.approval_stake as u64, DEN);
				}
			}
		}

		// loop 2: increment score
		for n in &voters {
			for e in &n.edges {
				let c = &mut candidates[e.candidate_index];
				if !c.elected && !c.approval_stake.is_zero() {
					let temp_n = Rational64::multiply_by_rational(
						n.load.n(),
						n.budget as _,
						c.approval_stake as _,
					);
					let temp_d = n.load.d();
					let temp = Rational64::from(temp_n, temp_d);
					c.score = c.score.lazy_saturating_add(temp);
				}
			}
		}

		// loop 3: find the best
		if let Some(winner) = candidates
			.iter_mut()
			.filter(|c| !c.elected)
			.min_by_key(|c| c.score)
		{
			// loop 3: update voter and edge load
			winner.elected = true;
			for n in &mut voters {
				for e in &mut n.edges {
					if e.who == winner.who {
						e.load = winner.score.lazy_saturating_sub(n.load);
						n.load = winner.score;
					}
				}
			}

			elected_candidates.push((winner.who.clone(), winner.approval_stake));
		} else {
			break;
		}
	} // end of all rounds

	// update backing stake of candidates and voters
	for n in &mut voters {
		let mut assignment = (n.who.clone(), vec![]);
		for e in &mut n.edges {
			if elected_candidates
				.iter()
				.position(|(ref c, _)| *c == e.who)
				.is_some()
			{
				let per_bill_parts: R::Inner = {
					if n.load == e.load {
						// Full support. No need to calculate.
						R::ACCURACY
					} else {
						if e.load.d() == n.load.d() {
							// return e.load / n.load.
							let desired_scale = R::ACCURACY.saturated_into() as _;
							let parts = Rational64::multiply_by_rational(
								desired_scale,
								e.load.n(),
								n.load.n(),
							);

							TryFrom::try_from(parts)
								// If the result cannot fit into R::Inner. Defensive only. This can
								// never happen. `desired_scale * e / n`, where `e / n < 1` always
								// yields a value smaller than `desired_scale`, which will fit into
								// R::Inner.
								.unwrap_or(Bounded::max_value())
						} else {
							// defensive only. Both edge and nominator loads are built from
							// scores, hence MUST have the same denominator.
							Zero::zero()
						}
					}
				};

				let per_thing = R::from_parts(per_bill_parts);

				assignment.1.push((e.who.clone(), per_thing));
			}
		}

		if assignment.1.len() > 0 {
			// To ensure an assertion indicating: no stake from the nominator going to waste,
			// we add a minimal post-processing to equally assign all of the leftover stake ratios.
			let vote_count: R::Inner = assignment.1.len().saturated_into();
			let len = assignment.1.len();
			let mut sum: R::Inner = Zero::zero();
			assignment
				.1
				.iter()
				.for_each(|a| sum = sum.saturating_add(a.1.deconstruct()));
			let accuracy = R::ACCURACY;
			let diff = accuracy.saturating_sub(sum);
			let diff_per_vote = (diff / vote_count).min(accuracy);

			if !diff_per_vote.is_zero() {
				for i in 0..len {
					let current_ratio = assignment.1[i % len].1;
					let next_ratio = current_ratio.saturating_add(R::from_parts(diff_per_vote));
					assignment.1[i % len].1 = next_ratio;
				}
			}

			// `remainder` is set to be less than maximum votes of a nominator (currently 16).
			// safe to cast it to usize.
			let remainder = diff - diff_per_vote * vote_count;
			for i in 0..remainder.saturated_into::<usize>() {
				let current_ratio = assignment.1[i % len].1;
				let next_ratio = current_ratio.saturating_add(R::from_parts(1u8.into()));
				assignment.1[i % len].1 = next_ratio;
			}
			assigned.push(assignment);
		}
	}

	Some(PhragmenResult {
		winners: elected_candidates,
		assignments: assigned,
	})
}

/// Build the support map from the given phragmen result.
pub fn build_support_map<AccountId, R, RingBalance, KtonBalance, FS, FSS>(
	elected_stashes: &Vec<AccountId>,
	assignments: &Vec<(AccountId, Vec<PhragmenAssignment<AccountId, R>>)>,
	power_of: FS,
	stake_of: FSS,
) -> SupportMap<AccountId, RingBalance, KtonBalance>
where
	AccountId: Default + Ord + Member,
	R: PerThing
		+ sp_std::ops::Mul<Votes, Output = Votes>
		+ sp_std::ops::Mul<RingBalance, Output = RingBalance>
		+ sp_std::ops::Mul<KtonBalance, Output = KtonBalance>,
	RingBalance: Default + Copy + AtLeast32Bit,
	KtonBalance: Default + Copy + AtLeast32Bit,
	for<'r> FS: Fn(&'r AccountId) -> Power,
	for<'r> FSS: Fn(&'r AccountId) -> (RingBalance, KtonBalance),
{
	let to_votes = |p: Power| p as Votes;
	// Initialize the support of each candidate.
	let mut supports = <SupportMap<AccountId, RingBalance, KtonBalance>>::new();
	elected_stashes.iter().for_each(|e| {
		supports.insert(e.clone(), Default::default());
	});

	// build support struct.
	for (n, assignment) in assignments.iter() {
		for (c, per_thing) in assignment.iter() {
			let nominator_stake = to_votes(power_of(n));
			// AUDIT: it is crucially important for the `Mul` implementation of all
			// per-things to be sound.
			let (ring_balance, kton_balance) = {
				let (r, k) = stake_of(n);
				(*per_thing * r, *per_thing * k)
			};
			let other_stake = *per_thing * nominator_stake;
			if let Some(support) = supports.get_mut(c) {
				support.voters.push(PhragmenStakedAssignment {
					account_id: n.clone(),
					ring_balance,
					kton_balance,
					votes: other_stake,
				});
				support.total_votes = support.total_votes.saturating_add(other_stake);
			}
		}
	}
	supports
}

/// Performs equalize post-processing to the output of the election algorithm. This happens in
/// rounds. The number of rounds and the maximum diff-per-round tolerance can be tuned through input
/// parameters.
///
/// No value is returned from the function and the `supports` parameter is updated.
///
/// * `assignments`: exactly the same is the output of phragmen.
/// * `supports`: mutable reference to s `SupportMap`. This parameter is updated.
/// * `tolerance`: maximum difference that can occur before an early quite happens.
/// * `iterations`: maximum number of iterations that will be processed.
/// * `power_of`: something that can return the stake stake of a particular candidate or voter.
pub fn equalize<AccountId, RingBalance, KtonBalance, FS>(
	mut assignments: Vec<(
		AccountId,
		Vec<PhragmenStakedAssignment<AccountId, RingBalance, KtonBalance>>,
	)>,
	supports: &mut SupportMap<AccountId, RingBalance, KtonBalance>,
	tolerance: Votes,
	iterations: usize,
	power_of: FS,
) where
	AccountId: Ord + Clone,
	RingBalance: Copy + AtLeast32Bit,
	KtonBalance: Copy + AtLeast32Bit,
	for<'r> FS: Fn(&'r AccountId) -> Power,
{
	// prepare the data for equalise
	for _i in 0..iterations {
		let mut max_diff = 0;

		for (voter, assignment) in assignments.iter_mut() {
			let voter_budget = power_of(&voter);

			let diff = do_equalize::<_, _, _>(voter, voter_budget, assignment, supports, tolerance);
			if diff > max_diff {
				max_diff = diff;
			}
		}

		if max_diff < tolerance {
			break;
		}
	}
}

/// actually perform equalize. same interface is `equalize`. Just called in loops with a check for
/// maximum difference.
fn do_equalize<AccountId, RingBalance, KtonBalance>(
	voter: &AccountId,
	budget_balance: Power,
	elected_edges: &mut Vec<PhragmenStakedAssignment<AccountId, RingBalance, KtonBalance>>,
	support_map: &mut SupportMap<AccountId, RingBalance, KtonBalance>,
	tolerance: Votes,
) -> Votes
where
	AccountId: Ord + Clone,
	RingBalance: Copy + AtLeast32Bit,
	KtonBalance: Copy + AtLeast32Bit,
{
	let to_votes = |p: Power| p as Votes;
	let budget = to_votes(budget_balance);

	// Nothing to do. This voter had nothing useful.
	// Defensive only. Assignment list should always be populated. 1 might happen for self vote.
	if elected_edges.is_empty() || elected_edges.len() == 1 {
		return 0;
	}

	let stake_used = elected_edges
		.iter()
		.fold(0 as Votes, |s, e| s.saturating_add(e.votes));

	let backed_stakes_iter = elected_edges
		.iter()
		.filter_map(|e| support_map.get(&e.account_id))
		.map(|e| e.total_votes);

	let backing_backed_stake = elected_edges
		.iter()
		.filter(|e| e.votes > 0)
		.filter_map(|e| support_map.get(&e.account_id))
		.map(|e| e.total_votes)
		.collect::<Vec<Votes>>();

	let mut difference;
	if backing_backed_stake.len() > 0 {
		let max_stake = backing_backed_stake
			.iter()
			.max()
			.expect("vector with positive length will have a max; qed");
		let min_stake = backed_stakes_iter
			.min()
			.expect("iterator with positive length will have a min; qed");

		difference = max_stake.saturating_sub(min_stake);
		difference = difference.saturating_add(budget.saturating_sub(stake_used));
		if difference < tolerance {
			return difference;
		}
	} else {
		difference = budget;
	}

	// Undo updates to support
	elected_edges.iter_mut().for_each(|e| {
		if let Some(support) = support_map.get_mut(&e.account_id) {
			support.total_votes = support.total_votes.saturating_sub(e.votes);
			support
				.voters
				.retain(|i_support| i_support.account_id != *voter);
		}
		e.votes = 0;
	});

	elected_edges.sort_unstable_by_key(|e| {
		if let Some(e) = support_map.get(&e.account_id) {
			e.total_votes
		} else {
			Zero::zero()
		}
	});

	let mut cumulative_stake: Votes = 0;
	let mut last_index = elected_edges.len() - 1;
	let mut idx = 0usize;
	for e in &mut elected_edges[..] {
		if let Some(support) = support_map.get_mut(&e.account_id) {
			let stake = support.total_votes;
			let stake_mul = stake.saturating_mul(idx as Votes);
			let stake_sub = stake_mul.saturating_sub(cumulative_stake);
			if stake_sub > budget {
				last_index = idx.checked_sub(1).unwrap_or(0);
				break;
			}
			cumulative_stake = cumulative_stake.saturating_add(stake);
		}
		idx += 1;
	}

	let last_votes = elected_edges[last_index].votes;
	let split_ways = last_index + 1;
	let excess = budget
		.saturating_add(cumulative_stake)
		.saturating_sub(last_votes.saturating_mul(split_ways as Votes));
	elected_edges.iter_mut().take(split_ways).for_each(|e| {
		if let Some(support) = support_map.get_mut(&e.account_id) {
			let votes = (excess / split_ways as Votes)
				.saturating_add(last_votes)
				.saturating_sub(support.total_votes);
			e.ring_balance =
				multiply_by_rational(e.ring_balance.saturated_into(), votes as _, e.votes as _)
					.unwrap_or(0)
					.saturated_into();
			e.kton_balance =
				multiply_by_rational(e.kton_balance.saturated_into(), votes as _, e.votes as _)
					.unwrap_or(0)
					.saturated_into();
			e.votes = votes;

			support.total_votes = support.total_votes.saturating_add(e.votes);
			support.voters.push(PhragmenStakedAssignment {
				account_id: voter.clone(),
				ring_balance: e.ring_balance,
				kton_balance: e.kton_balance,
				votes: e.votes,
			});
		}
	});

	difference
}
