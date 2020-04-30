use serde_json;
// --- darwinia ---
use crate::{mock::*, *};

/// Extract value from JSON response
#[test]
fn test_extract_value_from_json_response() {
	let result_part =
		extract_from_json_str(&SUPPOSED_SHADOW_FAKE_RESPONSE[..], b"result" as &[u8]).unwrap();
	assert_eq!(
		result_part,
		br#""eth_header":{eth-content},"proof":[proof-content]"# as &[u8]
	);
	let eth_header_part = extract_from_json_str(result_part, b"eth_header" as &[u8]).unwrap();
	assert_eq!(eth_header_part, br#"eth-content"# as &[u8]);
	let proof_header_part = extract_from_json_str(result_part, b"proof" as &[u8]).unwrap();
	assert_eq!(proof_header_part, br#"proof-content"# as &[u8]);
}

/// Extract value from JSON response with not alphabetical order
#[test]
fn test_extract_value_from_non_order_json_response() {
	let result_part =
		extract_from_json_str(&SUPPOSED_SHADOW_NON_ORDER_RESPONSE[..], b"result" as &[u8]).unwrap();
	assert_eq!(
		result_part,
		br#""proof":[proof-content],"eth_header":{eth-content}"# as &[u8]
	);
	let eth_header_part = extract_from_json_str(result_part, b"eth_header" as &[u8]).unwrap();
	assert_eq!(eth_header_part, br#"eth-content"# as &[u8]);
	let proof_header_part = extract_from_json_str(result_part, b"proof" as &[u8]).unwrap();
	assert_eq!(proof_header_part, br#"proof-content"# as &[u8]);
}

/// Basice JSON response handle
#[test]
fn test_build_eth_header_from_json_response() {
	let eth_header_part =
		extract_from_json_str(&SUPPOSED_SHADOW_JSON_RESPONSE[..], b"eth_header" as &[u8])
			.unwrap_or_default();
	let header = EthHeader::from_str_unchecked(from_utf8(eth_header_part).unwrap_or_default());
	assert_eq!(header.hash.unwrap(), header.re_compute_hash());

	let proof_part = extract_from_json_str(&SUPPOSED_SHADOW_JSON_RESPONSE[..], b"proof" as &[u8])
		.unwrap_or_default();
	let double_node_with_proof_list =
		EthOffchain::parse_double_node_with_proof_list_from_json_str(proof_part).unwrap();
	assert_eq!(1, double_node_with_proof_list.len());
}

/// Basice SCALE response handle
#[test]
fn test_build_eth_header_from_scale_response() {
	let eth_header_part =
		extract_from_json_str(&SUPPOSED_SHADOW_SCALE_RESPONSE[..], b"eth_header" as &[u8])
			.unwrap_or_default();
	let scale_bytes = hex_bytes_unchecked(from_utf8(eth_header_part).unwrap_or_default());
	let scale_decode_header: EthHeader =
		Decode::decode::<&[u8]>(&mut &scale_bytes[..]).unwrap_or_default();

	let header = EthHeader::from_str_unchecked(SUPPOSED_ETH_HEADER);
	assert_eq!(scale_decode_header, header);

	let proof_part = extract_from_json_str(&SUPPOSED_SHADOW_SCALE_RESPONSE[..], b"proof" as &[u8])
		.unwrap_or_default();
	let decoded_double_node_with_proof =
		EthOffchain::parse_double_node_with_proof_list_from_scale_str(proof_part).unwrap();

	assert_eq!(
		vec![DoubleNodeWithMerkleProof::default()],
		decoded_double_node_with_proof
	);
}

/// Request format should be json
#[test]
fn test_request_payload_format() {
	let payload_without_option = EthOffchain::build_payload(1, false);
	assert!(serde_json::from_str::<serde_json::value::Value>(
		from_utf8(&payload_without_option[..]).unwrap()
	)
	.is_ok());

	let payload_with_option = EthOffchain::build_payload(1, true);
	assert!(serde_json::from_str::<serde_json::value::Value>(
		from_utf8(&payload_with_option[..]).unwrap()
	)
	.is_ok());
}

/// Test offchain worker before any header relayed
#[test]
fn test_should_error_when_best_header_not_set() {
	ExtBuilder::default().build().execute_with(|| {
		// assert_noop!(EthOffchain::relay_header(), EthOffchainError::BestHeaderNE);
	});
}

/// Test offchain worker with different shadow service
#[test]
fn test_should_handle_different_shadow_service() {
	// NOTE:`set_shadow_service` is unsafe
	// Keep this test run in a single thread

	// should error when shadow service is non exists
	set_shadow_service(None);
	ExtBuilder::default()
		.set_genesis_header()
		.build()
		.execute_with(|| {
			// assert_noop!(EthOffchain::relay_header(), EthOffchainError::APIRespUnexp);
		});

	// handle the scale response from shadow service
	set_shadow_service(Some(ShadowService::Scale));
	ExtBuilder::default()
		.set_genesis_header()
		.build()
		.execute_with(|| {
			// assert_ok!(EthOffchain::relay_header());
		});

	// handle the json response from shadow service
	set_shadow_service(Some(ShadowService::Json));
	ExtBuilder::default()
		.set_genesis_header()
		.build()
		.execute_with(|| {
			// assert_ok!(EthOffchain::relay_header());
		});
}
