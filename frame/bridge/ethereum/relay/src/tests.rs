// --- substrate ---
use frame_support::assert_ok;
// --- darwinia ---
use crate::{mock::*, *};

#[test]
fn store_relay_header_parcel_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ethereum_relay_header_parcel: EthereumRelayHeaderParcel = serde_json::from_str(r#"{"header":{"parent_hash":"0x3dd4dc843801af12c0a6dd687642467a3ce835dca09159734dec03109a1c1f1f","timestamp":1479653850,"number":100,"author":"0xc2fa6dcef5a1fbf70028c5636e7f64cd46e7cfd4","transactions_root":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","uncles_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","extra_data":"0xd783010502846765746887676f312e362e33856c696e7578","state_root":"0xf5f18c33ddff06efa928d22a2432fb34a11e6f62cce825cdad1c78e1068e6b7b","receipts_root":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","log_bloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","gas_used":0,"gas_limit":15217318,"difficulty":827755,"seal":["0xa03172866e675b057a294d3f474e9141b588d5a0c622b4d8049e272c6a001e9c4e","0x886d88b33209e0a320"],"hash":"0xb40a0dfde1b270d7c58c3cb505c7e773c50198b28cce3e442c4e2f33ff764582"},"mmr_root":"0x33d834e1e65b96f470374134cf173f359a5b37c910a7e07c7d6148866c1805d7"}"#).unwrap();

		assert!(EthereumRelay::confirmed_parcel_of(100).is_none());
		assert!(!EthereumRelay::confirmed_block_numbers().contains(&100));
		assert!(EthereumRelay::best_confirmed_block_number() != 100);

		assert_eq!(ethereum_relay_header_parcel.header.number, 100);
		assert_ok!(EthereumRelay::store_relay_header_parcel(
			ethereum_relay_header_parcel.clone(),
		));

		assert_eq!(EthereumRelay::confirmed_parcel_of(100).unwrap(), ethereum_relay_header_parcel);
		assert!(EthereumRelay::confirmed_block_numbers().contains(&100));
		assert_eq!(EthereumRelay::best_confirmed_block_number(), 100);
	})
}

// #[test]
// fn proposal_basic_verification_should_sucess() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		for &game in [2].iter() {
// 			for id in 0..=2 {
// 				// eprintln!("{}, {}", game, id);

// 				assert_ok!(<EthereumRelay as Relayable>::verify(
// 					proposal_of_game_with_id(game, id)
// 				));
// 			}
// 		}
// 	})
// }
