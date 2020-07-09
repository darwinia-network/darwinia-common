// This file is part of merkle-mountain-range from nervosnetwork
use sp_std::prelude::*;

fn log2(mut n: u64) -> u64 {
	let mut k = 0;
	while n > 1 {
		k += 1;
		n >>= 1;
	}
	k
}

pub fn leaf_index_to_pos(index: u64) -> u64 {
	if index == 0 {
		return 0;
	}
	// leaf_count
	let mut leaves = index + 1;
	let mut tree_node_count = 0;
	let mut height = 0u32;
	while leaves > 1 {
		// get heighest peak height
		height = log2(leaves) as u32;
		// calculate leaves in peak
		let peak_leaves = 1 << height;
		// heighest positon
		let sub_tree_node_count = get_peak_pos_by_height(height) + 1;
		tree_node_count += sub_tree_node_count;
		leaves -= peak_leaves;
	}
	// two leaves can construct a new peak, the only valid number of leaves is 0 or 1.
	debug_assert!(leaves == 0 || leaves == 1, "remain leaves incorrect");
	if leaves == 1 {
		// add one pos for remain leaf
		// equals to `tree_node_count - 1 + 1`
		tree_node_count
	} else {
		let pos = tree_node_count - 1;
		pos - u64::from(height)
	}
}

fn get_peak_pos_by_height(height: u32) -> u64 {
	(1 << (height + 1)) - 2
}
