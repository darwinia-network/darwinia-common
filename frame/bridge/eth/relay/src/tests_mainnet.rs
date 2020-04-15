//! Tests for eth-relay.

// --- substrate ---
use frame_support::assert_ok;
// --- darwinia ---
use crate::{mock_mainnet::*, *};

#[test]
fn relay_mainet_header() {
	new_mainnet_test_ext().execute_with(|| {
		//		let (blocks, hashes) = get_blocks(&WEB3RS, 8996776, 8996777);

		let blocks_with_proofs: Vec<BlockWithProofs> = [
			"./src/data/8996776.json",
			"./src/data/8996777.json",
			"./src/data/8996778.json",
		]
		.iter()
		.map(|filename| read_block((&filename).to_string()))
		.collect();

		let header_8996776: EthHeader =
			rlp::decode(&blocks_with_proofs[0].header_rlp.0.to_vec()).unwrap();
		assert_ok!(EthRelay::init_genesis_header(&header_8996776, 0x234ac172));

		println!("{:?}", &header_8996776);

		let header_8996777: EthHeader =
			rlp::decode(&blocks_with_proofs[1].header_rlp.0.to_vec()).unwrap();

		println!("{:?}", &header_8996777);

		// relay
		assert_ok!(EthRelay::verify_header_with_proof(
			&header_8996777,
			&blocks_with_proofs[1].to_double_node_with_merkle_proof_vec()
		));
		assert_ok!(EthRelay::maybe_store_header(&header_8996777));

		let header_8996778: EthHeader =
			rlp::decode(&blocks_with_proofs[2].header_rlp.0.to_vec()).unwrap();

		println!("{:?}", &header_8996778);

		// relay
		assert_ok!(EthRelay::verify_header_with_proof(
			&header_8996778,
			&blocks_with_proofs[2].to_double_node_with_merkle_proof_vec()
		));
		assert_ok!(EthRelay::maybe_store_header(&header_8996778));

		//		for (block, proof) in blocks.into_iter().zip(blocks_with_proofs.into_iter()) {
		//			contract.add_block_header(block, proof.to_double_node_with_merkle_proof_vec());
		//		}
	});
}
