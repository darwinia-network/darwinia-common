//! Tests for eth-relay.

// --- substrate ---
use frame_support::{assert_err, assert_ok};
use frame_system::RawOrigin;
// --- darwinia ---
use crate::{mock::*, *};
use eth_primitives::receipt::TransactionOutcome;

#[test]
fn verify_receipt_proof() {
	new_test_ext().execute_with(|| {
		System::inc_account_nonce(&2);
		assert_ok!(EthRelay::set_number_of_blocks_safe(
			RawOrigin::Root.into(),
			0
		));

		// mock header and proof
		let [_, header, _, _, _] = mock_canonical_relationship();
		let proof_record = mock_canonical_receipt();

		// mock logs
		let mut logs = vec![];
		let mut log_entries = mock_receipt_logs();
		for _ in 0..log_entries.len() {
			logs.push(log_entries.pop().unwrap());
		}

		logs.reverse();

		// mock receipt
		let receipt = Receipt::new(TransactionOutcome::StatusCode(1), 1371263.into(), logs);

		// verify receipt
		assert_ok!(EthRelay::init_genesis_header(&header, 0x234ac172));
		assert_eq!(EthRelay::verify_receipt(&proof_record), Ok(receipt));
	});
}

#[test]
fn relay_header() {
	new_test_ext().execute_with(|| {
		let [origin, grandpa, _, parent, current] = mock_canonical_relationship();
		assert_ok!(EthRelay::init_genesis_header(&origin, 0x234ac172));

		// relay grandpa
		assert_ok!(EthRelay::verify_header(&grandpa));
		assert_ok!(EthRelay::maybe_store_header(&grandpa));

		// relay parent
		assert_ok!(EthRelay::verify_header(&parent));
		assert_ok!(EthRelay::maybe_store_header(&parent));

		// relay current
		assert_ok!(EthRelay::verify_header(&current));
		assert_ok!(EthRelay::maybe_store_header(&current));
	});
}

#[test]
fn build_genesis_header() {
	let genesis_header = EthHeader::from_str_unchecked(MAINNET_GENESIS_HEADER);
	assert_eq!(genesis_header.hash(), genesis_header.re_compute_hash());
	println!("{:?}", rlp::encode(&genesis_header));
}

#[test]
fn mix_hash_should_work() {
	// let header = EthHeader::from_str_unchecked(
	// 	r#"
	// 		{"difficulty":"0x92c07e50de0b9","extraData":"0x7575706f6f6c2e636e2d3163613037623939","gasLimit":"0x98700d","gasUsed":"0x98254e","hash":"0xb972df738904edb8adff9734eebdcb1d3b58fdfc68a48918720a4a247170f15e","logsBloom":"0x0c0110a00144a0082057622381231d842b8977a98d1029841000a1c21641d91946594605e902a5432000159ad24a0300428d8212bf4d1c81c0f8478402a4a818010011437c07a112080e9a4a14822311a6840436f26585c84cc0d50693c148bf9830cf3e0a08970788a4424824b009080d52372056460dec808041b68ea04050bf116c041f25a3329d281068740ca911c0d4cd7541a1539005521694951c286567942d0024852080268d29850000954188f25151d80e4900002122c01ad53b7396acd34209c24110b81b9278642024603cd45387812b0696d93992829090619cf0b065a201082280812020000430601100cb08a3808204571c0e564d828648fb","miner":"0xd224ca0c819e8e97ba0136b3b95ceff503b79f53","mixHash":"0x0ea8027f96c18f474e9bc74ff71d29aacd3f485d5825be0a8dde529eb82a47ed","nonce":"0x55859dc00728f99a","number":"0x8947aa","parentHash":"0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b","receiptsRoot":"0x3fbd99e253ff45045eec1e0011ac1b45fa0bccd641a356727defee3b166dd3bf","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","size":"0x8a17","stateRoot":"0x5dfc6357dda61a7f927292509afacd51453ff158342eb9628ccb419fbe91c638","timestamp":"0x5ddb67a3","totalDifficulty":"0x2c10c7941a5999fb691","transactions":[],"transactionsRoot":"0xefebac0e71cc2de04cf2f509bb038a82bbe92a659e010061b49b5387323b5ea6","uncles":[]}
	// 	"#,
	// );
	// let seal = EthashSeal::parse_seal(header.seal()).unwrap();
	// let light_dag = DAG::new(header.number.into());
	// let partial_header_hash = header.bare_hash();
	// let mix_hash = light_dag.hashimoto(partial_header_hash, seal.nonce).0;
	// assert_eq!(mix_hash, seal.mix_hash);

	// let header = EthHeader::from_str_unchecked(
	// 	r#"
	// 		{"difficulty":"0x3ff800000","extraData":"0x476574682f76312e302e302f6c696e75782f676f312e342e32","gasLimit":"0x1388","gasUsed":"0x0","hash":"0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","miner":"0x05a56e2d52c817161883f50c441c3228cfe54d9f","mixHash":"0x969b900de27b6ac6a67742365dd65f55a0526c41fd18e1b16f1a1215c2e66f59","nonce":"0x539bd4979fef1ec4","number":"0x1","parentHash":"0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","size":"0x219","stateRoot":"0xd67e4d450343046425ae4271474353857ab860dbc0a1dde64b41b5cd3a532bf3","timestamp":"0x55ba4224","totalDifficulty":"0x7ff800000","transactions":[],"transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","uncles":[]}
	// 	"#,
	// );
	// println!("{:?}", header);
	// let seal = EthashSeal::parse_seal(header.seal()).unwrap();
	// let light_dag = DAG::new(header.number.into());
	// let partial_header_hash = header.bare_hash();
	// let mix_hash = light_dag.hashimoto(partial_header_hash, seal.nonce).0;
	// // left: `0xf0f8e2b1be7b97f147c8be99c81e6d44eb45121a661702775ec3ce9d53e6e76b`
	// // right: `0x969b900de27b6ac6a67742365dd65f55a0526c41fd18e1b16f1a1215c2e66f59`
	// assert_eq!(mix_hash, seal.mix_hash);

	// let header = EthHeader::from_str_unchecked(
	// 	r#"
	// 		{"difficulty":"0x7dc19ab85444d","extraData":"0x7070796520e4b883e5bda9e7a59ee4bb99e9b1bc","gasLimit":"0x986fad","gasUsed":"0x2c91b9","hash":"0x4eaded4dc5a10bcd0b758f984741aea4498aee0b89f2522787c27289ea9703f5","logsBloom":"0x000000210040801800a004400200000001008188100080000020800115082182800010008000100820404842920401810148020010000441a5800044000080000800040001010810000000d800301080000e0000008020101040c00000000800007020000024000129004800101200080010122000000a8001000058080080021804c010404003010c8000140891402c800060700c014050034228002010a26401000084004000040600008040c001028008842808000200100402401880b008240000230010460000c00024d40002080400040188c9400404000018001020010000000110200001040800803000200428010002800028002000100108900000","miner":"0x829bd824b016326a401d083b33d092293333a830","mixHash":"0xb24fd6b4816925d12eba78fd50a72bd44920ff17d8c5c0674f33c0dea3e0b17b","nonce":"0x2207b3b00a17fd68","number":"0x93806e","parentHash":"0x5eccf3a95d2ae352a05ced7de02b6b41b99a780c680af67162f7673b9bc9a00f","receiptsRoot":"0x02397d6c66bd6bb3ba7fd7b090c0a4ae716b641c6fdb47b607db17b73b7be24a","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","size":"0x34a8","stateRoot":"0x266ccf1a04285b6fe626f3b2b49613893132c5b807d80ac860a470a742a2ae5c","timestamp":"0x5e6c35d5","totalDifficulty":"0x313df9acc78f805f41c","transactions":[],"transactionsRoot":"0xb8a7a9f3b4c9ad1a686f7dfbde2c69e3a25bdc2155db18707ae16ffe4d93c7f7","uncles":[]}
	// 	"#,
	// );
	// println!("{:?}", header);
	// let seal = EthashSeal::parse_seal(header.seal()).unwrap();
	// let light_dag = DAG::new(header.number.into());
	// let partial_header_hash = header.bare_hash();
	// let mix_hash = light_dag.hashimoto(partial_header_hash, seal.nonce).0;
	// // left: `0x628c052cf34686ac1796d1b08bcda2cc229ab9936825cbfe4dbc67ffd5ca646d`
	// // right: `0xb24fd6b4816925d12eba78fd50a72bd44920ff17d8c5c0674f33c0dea3e0b17b`
	// assert_eq!(mix_hash, seal.mix_hash);
}

/// # Check Receipt Safety
///
/// ## Family Tree
///
/// | pos     | height  | tx                                                                 |
/// |---------|---------|--------------------------------------------------------------------|
/// | origin  | 7575765 |                                                                    |
/// | grandpa | 7575766 | 0xc56be493f656f1c8222006eda5cd3392be5f0c096e8b7fb1c5542088c0f0c889 |
/// | uncle   | 7575766 |                                                                    |
/// | parent  | 7575767 |                                                                    |
/// | current | 7575768 | 0xfc836bf547f1e035e837bf0a8d26e432aa26da9659db5bf6ba69b0341d818778 |
///
/// To help reward miners for when duplicate block solutions are found
/// because of the shorter block times of Ethereum (compared to other cryptocurrency).
/// An uncle is a smaller reward than a full block.
///
/// ## Note:
///
/// check receipt should
/// - succeed when we relayed the correct header
/// - failed when canonical hash was re-orged by the block which contains our tx's brother block
#[test]
fn check_receipt_safety() {
	new_test_ext().execute_with(|| {
		assert_ok!(EthRelay::add_authority(RawOrigin::Root.into(), 0));
		assert_ok!(EthRelay::set_number_of_blocks_safe(
			RawOrigin::Root.into(),
			0
		));

		// family tree
		let [origin, grandpa, uncle, _, _] = mock_canonical_relationship();
		assert_ok!(EthRelay::init_genesis_header(&origin, 0x234ac172));

		let receipt = mock_canonical_receipt();
		assert_ne!(grandpa.hash, uncle.hash);
		assert_eq!(grandpa.number, uncle.number);

		// check receipt should succeed when we relayed the correct header
		assert_ok!(EthRelay::relay_header(Origin::signed(0), grandpa.clone()));
		assert_ok!(EthRelay::check_receipt(Origin::signed(0), receipt.clone()));

		// check should fail when canonical hash was re-orged by
		// the block which contains our tx's brother block
		assert_ok!(EthRelay::relay_header(Origin::signed(0), uncle));
		assert_err!(
			EthRelay::check_receipt(Origin::signed(0), receipt.clone()),
			<Error<Test>>::HeaderNC
		);
	});
}

#[test]
fn canonical_reorg_uncle_should_succeed() {
	new_test_ext().execute_with(|| {
		assert_ok!(EthRelay::add_authority(RawOrigin::Root.into(), 0));
		assert_ok!(EthRelay::set_number_of_blocks_safe(
			RawOrigin::Root.into(),
			0
		));

		let [origin, grandpa, uncle, _, _] = mock_canonical_relationship();
		assert_ok!(EthRelay::init_genesis_header(&origin, 0x234ac172));

		// check relationship
		assert_ne!(grandpa.hash, uncle.hash);
		assert_eq!(grandpa.number, uncle.number);

		let (gh, uh) = (grandpa.hash, uncle.hash);
		let number = grandpa.number;

		// relay uncle header
		assert_ok!(EthRelay::relay_header(Origin::signed(0), uncle));
		assert_eq!(EthRelay::canonical_header_hash_of(number), uh.unwrap());

		// relay grandpa and re-org uncle
		assert_ok!(EthRelay::relay_header(Origin::signed(0), grandpa));
		assert_eq!(EthRelay::canonical_header_hash_of(number), gh.unwrap());
	});
}

#[test]
fn test_safety_block() {
	new_test_ext().execute_with(|| {
		assert_ok!(EthRelay::add_authority(RawOrigin::Root.into(), 0));
		assert_ok!(EthRelay::set_number_of_blocks_safe(
			RawOrigin::Root.into(),
			2
		));

		// family tree
		let [origin, grandpa, parent, uncle, current] = mock_canonical_relationship();

		let receipt = mock_canonical_receipt();

		// not safety after 0 block
		assert_ok!(EthRelay::init_genesis_header(&origin, 0x234ac172));
		assert_ok!(EthRelay::relay_header(Origin::signed(0), grandpa));
		assert_err!(
			EthRelay::check_receipt(Origin::signed(0), receipt.clone()),
			<Error<Test>>::HeaderNS
		);

		// not safety after 2 blocks
		assert_ok!(EthRelay::relay_header(Origin::signed(0), parent));
		assert_ok!(EthRelay::relay_header(Origin::signed(0), uncle));
		assert_err!(
			EthRelay::check_receipt(Origin::signed(0), receipt.clone()),
			<Error<Test>>::HeaderNS
		);

		// safety after 3 blocks
		assert_ok!(EthRelay::relay_header(Origin::signed(0), current));
		assert_ok!(EthRelay::check_receipt(Origin::signed(0), receipt));
	});
}
