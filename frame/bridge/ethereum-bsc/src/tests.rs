// --- substrate ---
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;
// --- darwinia ---
use crate::mock::*;
use bsc_primitives::BSCHeader;

#[test]
fn recover_creator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let header = BSC::finalized_checkpoint();
		let creator = BSC::recover_creator(Configuration::get().chain_id, &header).unwrap();

		assert_eq!(header.coinbase, creator);
	});
	ExtBuilder::default().testnet().build().execute_with(|| {
		let header = BSC::finalized_checkpoint();
		let creator = BSC::recover_creator(97, &header).unwrap();

		assert_eq!(header.coinbase, creator);
	});
}

#[test]
fn extract_authorities_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let header = BSC::finalized_checkpoint();
		let signers = BSC::extract_authorities(&header).unwrap();
		let expected_signers = [
			"0x2465176c461afb316ebc773c61faee85a6515daa",
			"0x295e26495cef6f69dfa69911d9d8e4f3bbadb89b",
			"0x29a97c6effb8a411dabc6adeefaa84f5067c8bbe",
			"0x2d4c407bbe49438ed859fe965b140dcf1aab71a9",
			"0x3f349bbafec1551819b8be1efea2fc46ca749aa1",
			"0x4430b3230294d12c6ab2aac5c2cd68e80b16b581",
			"0x685b1ded8013785d6623cc18d214320b6bb64759",
			"0x70f657164e5b75689b64b7fd1fa275f334f28e18",
			"0x72b61c6014342d914470ec7ac2975be345796c2b",
			"0x7ae2f5b9e386cd1b50a4550696d957cb4900f03a",
			"0x8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec73",
			"0x9bb832254baf4e8b4cc26bd2b52b31389b56e98b",
			"0x9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88",
			"0xa6f79b60359f141df90a0c745125b131caaffd12",
			"0xb8f7166496996a7da21cf1f1b04d9b3e26a3d077",
			"0xbe807dddb074639cd9fa61b47676c064fc50d62c",
			"0xce2fd7544e0b2cc94692d4a704debef7bcb61328",
			"0xe2d3a739effcd3a99387d015e260eefac72ebea1",
			"0xe9ae3261a475a27bb1028f140bc2a7c843318afd",
			"0xea0a6e3c511bbd10f4519ece37dc24887e11b55d",
			"0xee226379db83cffc681495730c11fdde79ba4c0c",
		]
		.iter()
		.map(array_bytes::hex_into_unchecked)
		.collect::<Vec<_>>();

		assert_eq!(signers, expected_signers);
	});
	ExtBuilder::default().testnet().build().execute_with(|| {
		let header = BSC::finalized_checkpoint();
		let signers = BSC::extract_authorities(&header).unwrap();
		let expected_signers = [
			"0x1284214b9b9c85549ab3d2b972df0deef66ac2c9",
			"0x35552c16704d214347f29fa77f77da6d75d7c752",
			"0x3679479c2402e921db00923e014cd439c606c596",
			"0x7a1a4ad9cc746a70ee58568466f7996dd0ace4e8",
			"0x96c5d20b2a975c050e4220be276ace4892f4b41a",
			"0x980a75ecd1309ea12fa2ed87a8744fbfc9b863d5",
			"0xa2959d3f95eae5dc7d70144ce1b73b403b7eb6e0",
			"0xb71b214cb885500844365e95cd9942c7276e7fd8",
			"0xc89c669357d161d57b0b255c94ea96e179999919",
			"0xe625dd7ad2f7b88723857946a41af646c589c336",
		]
		.iter()
		.map(array_bytes::hex_into_unchecked)
		.collect::<Vec<_>>();

		assert_eq!(signers, expected_signers);
	});
}

#[test]
fn verify_and_update_authority_set_unsigned_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let header = BSCHeader::default();

		assert_noop!(
			BSC::verify_and_update_authority_set_unsigned(Origin::root(), vec![header]),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn verify_and_update_authority_set_signed_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let header = serde_json::from_str::<BSCHeader>(
			r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c675b589d9452d45327429ff925359ca25b1cc0245ffb869dbbcffb5a0d3c72f103a1dcb28b105926c636747dbc265f8dda0090784be3febffdd7909aa6f416d200",
			"gasLimit": "0x391a17f",
			"gasUsed": "0x151a7b2",
			"hash": "0x2af8376a302e60d766a74c4b4bbc98be08611865f3545da840062eabac511aff",
			"logsBloom": "0x4f7a466ebd89d672e9d73378d03b85204720e75e9f9fae20b14a6c5faf1ca5f8dd50d5b1077036e1596ef22860dca322ddd28cc18be6b1638e5bbddd76251bde57fc9d06a7421b5b5d0d88bcb9b920adeed3dbb09fd55b16add5f588deb6bcf64bbd59bfab4b82517a1c8fc342233ba17a394a6dc5afbfd0acfc443a4472212640cf294f9bd864a4ac85465edaea789a007e7f17c231c4ae790e2ced62eaef10835c4864c7e5b64ad9f511def73a0762450659825f60ceb48c9e88b6e77584816a2eb57fdaba54b71d785c8b85de3386e544ccf213ecdc942ef0193afae9ecee93ff04ff9016e06a03393d4d8ae14a250c9dd71bf09fee6de26e54f405d947e1",
			"miner": "0x72b61c6014342d914470ec7ac2975be345796c2b",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759590",
			"parentHash": "0x898c926e404409d6151d0e0ea156770fdaa2b31f8115b5f20bcb1b6cb4dc34c3",
			"receiptsRoot": "0x04aea8f3d2471b7ae64bce5dde7bb8eafa4cf73c65eab5cc049f92b3fda65dcc",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x5d03a66ae7fdcc6bff51e4c0cf40c6ec2d291090bddd9073ca4203d84b099bb9",
			"timestamp": "0x60ac738f",
			"totalDifficulty": "0xea4b80",
			"transactionsRoot": "0xb3db66bc49eac913dbdbe8aeaaee891762a6c5c28990c3f5f161726a8cb1c41d"
		}"#,
		).unwrap();

		assert_noop!(
			BSC::verify_and_update_authority_set_signed(Origin::signed(1), vec![header]),
			BSCError::InvalidHeadersSize
		);
	});
}

#[test]
fn verify_and_update_authority_set_signed_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let headers_7706000_to_7706010 = [
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c675b589d9452d45327429ff925359ca25b1cc0245ffb869dbbcffb5a0d3c72f103a1dcb28b105926c636747dbc265f8dda0090784be3febffdd7909aa6f416d200",
			"gasLimit": "0x391a17f",
			"gasUsed": "0x151a7b2",
			"hash": "0x2af8376a302e60d766a74c4b4bbc98be08611865f3545da840062eabac511aff",
			"logsBloom": "0x4f7a466ebd89d672e9d73378d03b85204720e75e9f9fae20b14a6c5faf1ca5f8dd50d5b1077036e1596ef22860dca322ddd28cc18be6b1638e5bbddd76251bde57fc9d06a7421b5b5d0d88bcb9b920adeed3dbb09fd55b16add5f588deb6bcf64bbd59bfab4b82517a1c8fc342233ba17a394a6dc5afbfd0acfc443a4472212640cf294f9bd864a4ac85465edaea789a007e7f17c231c4ae790e2ced62eaef10835c4864c7e5b64ad9f511def73a0762450659825f60ceb48c9e88b6e77584816a2eb57fdaba54b71d785c8b85de3386e544ccf213ecdc942ef0193afae9ecee93ff04ff9016e06a03393d4d8ae14a250c9dd71bf09fee6de26e54f405d947e1",
			"miner": "0x72b61c6014342d914470ec7ac2975be345796c2b",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759590",
			"parentHash": "0x898c926e404409d6151d0e0ea156770fdaa2b31f8115b5f20bcb1b6cb4dc34c3",
			"receiptsRoot": "0x04aea8f3d2471b7ae64bce5dde7bb8eafa4cf73c65eab5cc049f92b3fda65dcc",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x5d03a66ae7fdcc6bff51e4c0cf40c6ec2d291090bddd9073ca4203d84b099bb9",
			"timestamp": "0x60ac738f",
			"totalDifficulty": "0xea4b80",
			"transactionsRoot": "0xb3db66bc49eac913dbdbe8aeaaee891762a6c5c28990c3f5f161726a8cb1c41d"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b785af1d1bd69b06d22eb4185264881ebfa7e03c13336a379d6e295668d9a2019934a6b95af37bdd281c76c9902d4f92a45b0f17e554fed2660921001db734d86e01",
			"gasLimit": "0x3938700",
			"gasUsed": "0xaba7f5",
			"hash": "0x686bc4a6f643ff9de728a2386a2db77894faa255dc41b8e1f6e9cff4cb27e685",
			"logsBloom": "0xac62c6b962661253cd46815c9c8860fa4e2422209e14a422004fad6682204176241150b66142930017045c2002594102e85215a4925e846817403044336500c40efe34b5a206920a0d84c6090030ea66be9389b11149cd8ec1458010e073a21ac0319a36eb02cc3a5008044048093a3db0374279c6aa5e70cc75403a202a04a0487a40050bdf46a41885444409364b9860365d8dc0e0ce087d80044d21100dd2224c01205448d04c0a04c337036ae5106d581982d5e1f063856b9c2a2460e0a601425d0328134473567828c8dbaa15e869444c7910244094286041fbb2cce50220570529602148048329232c89c040000c1a876ce18e46f012c96415a8b90860",
			"miner": "0x7ae2f5b9e386cd1b50a4550696d957cb4900f03a",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759591",
			"parentHash": "0x2af8376a302e60d766a74c4b4bbc98be08611865f3545da840062eabac511aff",
			"receiptsRoot": "0x91d41e3697f96ed7b05f10c0463ac53b4e24ad815c263374f5772ece54c74492",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x4c933c90f8d6e1ffecb5f3dfa920f42800e5a933e89270afe4a2399e89d945e3",
			"timestamp": "0x60ac7392",
			"totalDifficulty": "0xea4b82",
			"transactionsRoot": "0x3d9f0347d455bbf966c644add2ec3ba72dce962ce0fee509c876abf333a8752d"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b791a889653e7fae96d9088f4f51e29fabab8946c97b3808c2701fcfa4fec15e310c5ca3dc8ce47b68a6375c43f7f13880b007929274c7c8353aa0849358dd5dfe00",
			"gasLimit": "0x3938700",
			"gasUsed": "0x1292023",
			"hash": "0x4ace1c429b3bf4f75654b2747610e3df4321a3b115f46cf4c3e4cdc07e16384b",
			"logsBloom": "0x0ef226282e8c5270b86ba86cd3942c9d6c084350d6c2c5204132cc15a1f9b991a048d04766127304435bb0c66e9acd033840811cb24e21ba067c2e7034a1bacd2cea6c841829024e8177d87b28a725a038f3282019c9abf6a66db800c4fa0e0d637b10358a0330d0749d47470208b8211a116274c78f1f600435143740302acb181c5015881354c05af5551c28b16d53102544c50623d828a58415e162720c7a8301862f974f587aae46c30e13a42c1a0c078b10d3e0dc6888eab232a6c1d1cc315a1d67c29369a34e584042c1d333d0fe56943f012444b01c41c82b3a7dacc680925ead003943280148710c0d28ddc20a1c5d7860eeceea6620202a2c18e162",
			"miner": "0x8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec73",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759592",
			"parentHash": "0x686bc4a6f643ff9de728a2386a2db77894faa255dc41b8e1f6e9cff4cb27e685",
			"receiptsRoot": "0x627077d9b0b683d8855106eb7f4e6c97776cac9e4c43b1296f65a3955e224bbf",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xe3013ffbcd54608736b6660a9440a0e02cdff3093b81f24c6f319dffa02f8794",
			"timestamp": "0x60ac7395",
			"totalDifficulty": "0xea4b84",
			"transactionsRoot": "0xb55bcd0739f2419b124f182e42f542fdd5edf2e461a7f622e69e8d0d30b81381"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b701765b0e4da320698b090670aaaddbee8a88960d68e9116a8a961f8489d091e53fcf58d47d8c52432252c5704eef934c6830145ef08b4f53c02c71a39d9ed4bf00",
			"gasLimit": "0x391b12a",
			"gasUsed": "0x16f1ddc",
			"hash": "0x9cfdde281bcb5ce7a945b7e7b65937d57144bd7c4f2afc78861b8278f962f1a9",
			"logsBloom": "0x0df90f3ffea6141387740c60fda57ccdefa56f2a8fc5de6b076e9dffb23951f2d53ef427a0e21725670a1efe2baf4b8adff637395e3f132aae760e6bbce7b0f401f33e9e993094ce1d56d36b2cb733e72eb8ce4f9b5c78df99b7bcbde196cf66dfd9d6ffff2e823f17df824bfc2ea9a6ae3562f4d7cbffe184ff167fe6346bcb9f68218549bf5d99fbc5e71594abafdf6bfd57f58b358c2839b6046db7bec89bde6bad5f77dae16a4f4d4f3c172eb7b9df0d23f777c43e7917e3bdfc6ca9f0877c16f6b692c5f92e0aaebcc7cc43b7ea78f5f8d3851ca6387d35e7ff8a3efec539d77c240468ca1c6ba313fcbe956fa458bc177af1cf4fef7b2a34541d8b0cfa",
			"miner": "0x9bb832254baf4e8b4cc26bd2b52b31389b56e98b",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759593",
			"parentHash": "0x4ace1c429b3bf4f75654b2747610e3df4321a3b115f46cf4c3e4cdc07e16384b",
			"receiptsRoot": "0x5f19964cce2277394b244076c855aa4be02731628ecc5661bb19ced5ba47a759",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x87243b67114f19232756fba8e0dd2ac9a0e0b1c3e5c719bfb678545aa69553ef",
			"timestamp": "0x60ac7398",
			"totalDifficulty": "0xea4b86",
			"transactionsRoot": "0x4c3c1d213b8fd8a940264e5b7f33081405c99f9f8c13a79140efe2a181cff43b"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b7efe068383e6ff15b5935f2c7e30ad5a53f131d172887a871c491e1444f6966164c4886535fb16165589bcd55e12ca3400a02471c9f5210da0e9e32fe1b20a01a00",
			"gasLimit": "0x3904626",
			"gasUsed": "0x1960a85",
			"hash": "0x881be32a4b68f7ecc8408f029126767b27dede57a62f769b2d8f7ef71113c64d",
			"logsBloom": "0x5c676edd1e7c5a3127f8c17af63126eb68422b42c62e65b41c689f6cc58b0ff587bada5e1e80735d777ddde69b8ea772b8e7e767f2eaa42b257a9c54732771ce11d5eabfe492e273c548e80d806703357eb76ca25f4c6eeb9fc7f53da37d4e3e0bc9f075cf2e58082fb99bcc24b8a863fe7136578eaeb7fe1fbd2033742124eb3cccb114538b341c7cce5f4da3d66aeb606477971042cc5a75c5307f662a39dcea68c02f7a4ee4d94b3ceb0ffa4eff292a2e1c7c59a55e3c2b629ff13e27f8f63adba42a3bc88da3d05a42ea9cc6578daf74ecdf413c61d2e039519b78e0af56bdf2d5ef626e479e63f07fe894a80b1a1c3af9b970ef58f473b2f85d798bdeff",
			"miner": "0x9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759594",
			"parentHash": "0x9cfdde281bcb5ce7a945b7e7b65937d57144bd7c4f2afc78861b8278f962f1a9",
			"receiptsRoot": "0xd714a1aeadca6d956d61c4d3341256d6751c2c9ab627e73a313fd7fbd1f81f05",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x736f5dd240ebd69038b16ab87cb346bf6e0ecbbb5f108793a5e5ba5cf767b5c4",
			"timestamp": "0x60ac739b",
			"totalDifficulty": "0xea4b88",
			"transactionsRoot": "0xa7b26890bfd73389e92b1d90d1bd8a352b8993213f94d6318219deb956ee6c51"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b7b8970ca7f3216b6f47a0ddf1164656f3d37beb4ae8c4fbda579033777bc33253045829c87353268b567640c7ceddb87c6f0ed1477ca29ea34cf8e62e3cf8305200",
			"gasLimit": "0x38f16f0",
			"gasUsed": "0x128e65a",
			"hash": "0x083de17c79eb4e9b671ebea582b386b26e37e191ffb878700acf82d0a8060990",
			"logsBloom": "0xcc612e08064c2050034c21c5d82571a9f104423ecf01c60002c6c5e2658425100510d0642c12b3dc39409938a0b817218e720bac528844c029010c55333da14010ba8150503082f22188639a5197e2ac2e912b3110d40a42c807b30a9956a84e4b8dd9272f13c94054482442e422a820582513dd842e9fa886bf42316c0230633878be2615bbe62028d0404410024c9a28b47605a03b9238f58c145946211ad2c30001404468b6ca3127171092661570e50e42c615e619b08f1383a4c05bc0844e33202f22f8583bd0310ac783069524f494263200f00210480a603a100fe68c10df054b00a0c03e0f2111d8cd0217065dbc0718e5de466658e12ccca7a34870",
			"miner": "0xa6f79b60359f141df90a0c745125b131caaffd12",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759595",
			"parentHash": "0x881be32a4b68f7ecc8408f029126767b27dede57a62f769b2d8f7ef71113c64d",
			"receiptsRoot": "0x02279165675ef03901bada335794d15561a3086088f7051c22cc67e8884a3fd3",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x91000dc3ee541fcbfbf9e50adbbd661573662c7a35cc77898ca26a51eb00311a",
			"timestamp": "0x60ac739e",
			"totalDifficulty": "0xea4b8a",
			"transactionsRoot": "0x1f1db2afc29bb24f4f179817bb21fd920097a35396a7b41f7d2a7151ac358f09"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b747e9be4caa3872a963f1e4131b9bba7b576d3f4429bddb7737219111753ce03054bb3ae9ad5a5981011e222a4b387236ca936b60cce8435c8f5da20b647329fd00",
			"gasLimit": "0x392a605",
			"gasUsed": "0x1542dc9",
			"hash": "0x6eba69cc82c3119fe62b59628ae72cafe4175b0cd7619a0ac5b71df4f9ce0883",
			"logsBloom": "0x8bf25afb7ed81a50daf8a9ec918183b2fc006600ce86c5f0599acdc79430711043ded543a442163c5d59d7ecffb83c1018c288c9821fd53d6fdecae8ba2f30be98f2dc130d5cd3df210baa29a1a4b5bef893c8bafb4508569895a0ac83df97326e18d17eaa0a10d0125846506ac0f969de3b4637c5af765bce7165f60e0eaca5189682bbc9df4210f59747489898099062f6fcd74abfd9b93d8c15fb434c9cb1eecb081b5dfeb1687f864b9647fd7f2cbe6b4975df80a8b8c95a9e76667148757d8e1d26d53e430f989cc5c8a853f329ed4058bfa0a601335ec7927a2af9e678d3d45ff98228c9e259f7f1ea3f52fc4a1d2c942a7376ce68e628dec80c90b3ea",
			"miner": "0xb8f7166496996a7da21cf1f1b04d9b3e26a3d077",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759596",
			"parentHash": "0x083de17c79eb4e9b671ebea582b386b26e37e191ffb878700acf82d0a8060990",
			"receiptsRoot": "0x6cbd7fbee301afc02cb1dd87bcc070270b2daeb41976361d3d3929a6f4d7e972",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x40b4f6168b0b7c36b95762ef6f1146339123431a0543e899525039bf033c811a",
			"timestamp": "0x60ac73a1",
			"totalDifficulty": "0xea4b8c",
			"transactionsRoot": "0x3283014eb5d06d8ab3530d750f42e41b82de6b3f55a7ddbb09dac74ae7c1f3c3"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b719723d29f7d061fab02990c800ac6d389996d9122adfb4aca5a04991ce662b2503623114073a52deb87a7105274086bf57e844f88924795ed5633da344252fd300",
			"gasLimit": "0x3938700",
			"gasUsed": "0x11d4503",
			"hash": "0x5679a2023347a64daa76f125ae9cea4e97e544f6cde29ea86dbf5634a6825778",
			"logsBloom": "0x5ff58a3dfe0e713e8b4d8c54d84880fb572cba398695c444785fdec7a424ddd48511523757b9af84df105a240409611e8a5217cc9250021af5040471b125244498faae278938f95f01c542aaa0c44c306f926f2970534c27e4a5f32382b784187b3956a49b0ee12970484b712620a810b29540d580ab37e904fb72b244a200621fea6810a9aa1d08bda05c680aba68cfa434f5034026ec0c3483147ccb7a6efacf90972e46eac4485b14612abf0a57b51e6e01e21bc1d8ab21621969300cedd5592fb5ae207c410623021aa30927798af1e4ac33e407ccb4203348321369af379497050e1563466fdba805080e0835052e7c8d6ce9af5473bb9a22a6e28f14ec",
			"miner": "0xbe807dddb074639cd9fa61b47676c064fc50d62c",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759597",
			"parentHash": "0x6eba69cc82c3119fe62b59628ae72cafe4175b0cd7619a0ac5b71df4f9ce0883",
			"receiptsRoot": "0x21baad2d37eadd8c0f0c83c4108eab23caf12898ce93075308491ecbf5ae1827",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xb1c2730f2b8d4e2db3b1882a4a3535b40eccdb732193e5b111b135c3d7e66693",
			"timestamp": "0x60ac73a4",
			"totalDifficulty": "0xea4b8e",
			"transactionsRoot": "0xe1069870eb63ec670bc9b85a5686d7a168623967d3881f29be150d0515f16ea4"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72701c6e8c84e316e4666f356e16f65bde5e7449c7de58937d8515b0a28178b861abfca923a533869b8a05f452f097d209f614553bf5165e9d0c4346ae8b4d47700",
			"gasLimit": "0x3938700",
			"gasUsed": "0xf112c9",
			"hash": "0xc81cf1dcd5063c2f63884ffcffdbda41e59bcd5bae48b2a4bda179b72114d341",
			"logsBloom": "0x0f313a2b5e1c52318a8a33508433d3ceefa94b2e9f1f42d069528da2e03805bacb2ad33633f5330445d954f44a99d7313e5d8d859e9a212b2411ad7fa02533c40df2ae406396ba6a21106679a0a300bd2c98092611643c52f5b4a160d7b64f4c7db83aff8fabe1c5430873c0a322f8e5cc59e2778f8bf7f09df159f40c6974e252ec0d7f4e1646d0f884e510601808d8a2e6458d5663cce931ae26d7e611e8f39a10862f76cb94cb79245b3e0f6eb62a0e05097859cc9af04b61de38816140a430e21c6386241227a098239399d372857d5528ba04e508320251405b5027f75512d054cb6624d92a2b68116923ad8f21393c2528e4eadc60206269d49a888ae1",
			"miner": "0xce2fd7544e0b2cc94692d4a704debef7bcb61328",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759598",
			"parentHash": "0x5679a2023347a64daa76f125ae9cea4e97e544f6cde29ea86dbf5634a6825778",
			"receiptsRoot": "0xecd39a15083ea7579eb519681139ce8cee80bf715c92cb14fe3dfae520bc5623",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xc80f94f0a82431833bca14dda51ce044f295c51af09858e79b2042a921329e8c",
			"timestamp": "0x60ac73a7",
			"totalDifficulty": "0xea4b90",
			"transactionsRoot": "0x573883fc0ef9c5801e861305f5fb232a5b242a221ce4379860a9620adb84d9fc"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b7edbebc7411171a042634750ea85596f260205c95bbdae2676e4e0e6874f613581e171c0ea142e12eb5e361f84acc9c89e37337fb2475f108157d160e5335763101",
			"gasLimit": "0x3938700",
			"gasUsed": "0x13deda4",
			"hash": "0xa8c0db3c4c73c66009e8eab935096b3c4027fbf036e27f849cf6858621a16a4a",
			"logsBloom": "0x64bb86391978f01d827d0950929d98f1ac08aa149687c13954fa6e46a0202850809ed33b440413c45f1a57e58e29410ab847b5f2062800fb13900af63a2794cc3cd34df6f01164dfa38ba24a80e47978e29129683af5c8129cffbb67c6f69d44581f9aacef0fe0c95a18445b506ba8752235c259cc87364a94b98531542004fa38c2b865a9b2538536cc85d008822884417e742540339918f195467e1aa0ad12afadc43d445a90d8e40ec77dd31ead3eae0c199cd9c03de40d661e2d2cb0cb10b98e7db67140458280021110006331e7e83400b1c0648135185104b7180ead360afbd5bd50a5653ec1e1195d4860160868981798c5cfd7773fa8474cbd8880f4",
			"miner": "0xe2d3a739effcd3a99387d015e260eefac72ebea1",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x759599",
			"parentHash": "0xc81cf1dcd5063c2f63884ffcffdbda41e59bcd5bae48b2a4bda179b72114d341",
			"receiptsRoot": "0x71588d92ee27d509d1e6870e4c108395472d11cddb44b9aed47bf5248a78a787",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x9700f54de3fe8393aa76fcc32a9f6ddc9cb9deb21200f220769ba9e6aaa855ff",
			"timestamp": "0x60ac73aa",
			"totalDifficulty": "0xea4b92",
			"transactionsRoot": "0x958be68fefd51a3e35133cd9711a5bde2e8c3b9599de17c58ef0893dc67193ec"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b7732a46744f6bbba56b13739737d70abbb03eca14dd199115afc1851dee74e60259d2f9927baea69d1ad824763fa28f272b5bb91d45059f86fa61fa1b4d8a5c6901",
			"gasLimit": "0x38ff37a",
			"gasUsed": "0x193d181",
			"hash": "0x8633735c2f43178b9248b0f99d510e4107a80907f5e5eab3a01fb792e59398ec",
			"logsBloom": "0x7cbd4faffffff7bccbfcf8629521c8ee58a82fd3de3de3e957c58d5794d28995a36cfdb72797f340f95890e27bdf1f771a570fdfd3daf46b2b16f367762f367726fa77d7d91743fe1b5f56e9a0b9b237ff938f385f4f4c56d5b5b28082f22f196e5bdbfdaf1afc10f65ad3df126af80bbb75e37ddf8aff7806b5c43f0e5850cac6ed33ffcc2bf58938f4f608aabefb9fa0aceddf677ad9fb7daf767f6610edd9cbc04c4fdc54a3f9ff7ccb15db7ef6723f0dcbe81fe494f8bb6a8fbf6481e4f5709f7faf6adca21f63b9c26a197ab60f7bc5c5b1916446321a116aaa3deeec66d4765dffa22165ae09687d5a5844831a76bdc5b857dfcee3a2f9a45eafaa87e5",
			"miner": "0xe9ae3261a475a27bb1028f140bc2a7c843318afd",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x75959a",
			"parentHash": "0xa8c0db3c4c73c66009e8eab935096b3c4027fbf036e27f849cf6858621a16a4a",
			"receiptsRoot": "0x4c1fd767550a855ffb24756fcde53408dd989dbff961115bf80cc0cdb610b81b",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xb978d927bcb58e884b86b1144a85bc2ba336a827ce1bd77c9995cb3a376a6d56",
			"timestamp": "0x60ac73ad",
			"totalDifficulty": "0xea4b94",
			"transactionsRoot": "0x11736117a52862926a62053b793cfb8a0e02ca668b9c3299134aaa17fde9bee5"
		}"#,
	].iter().map(|json| serde_json::from_str(json).unwrap()).collect::<Vec<BSCHeader>>();

		assert_ok!(BSC::verify_and_update_authority_set_signed(
			Origin::signed(1),
			headers_7706000_to_7706010,
		));
	});

	// for BSC testnet
	ExtBuilder::default().testnet().build().execute_with(|| {
		let testnet_headers_9516600_to_9516606 = [
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e75780000001600553d1284214b9b9c85549ab3d2b972df0deef66ac2c935552c16704d214347f29fa77f77da6d75d7c7523679479c2402e921db00923e014cd439c606c5967a1a4ad9cc746a70ee58568466f7996dd0ace4e896c5d20b2a975c050e4220be276ace4892f4b41a980a75ecd1309ea12fa2ed87a8744fbfc9b863d5a2959d3f95eae5dc7d70144ce1b73b403b7eb6e0b71b214cb885500844365e95cd9942c7276e7fd8c89c669357d161d57b0b255c94ea96e179999919e625dd7ad2f7b88723857946a41af646c589c336e1e7e55c0f0a308c989ebccef3276256a33372647b4f9b61ab82666dd67af8e7404995bd96d87b056f46203d012ed3e4521788f19c4c6d434a23e3b8df112e1a01",
			"gasLimit": "0x1c7f9be",
			"gasUsed": "0x5a8896",
			"hash": "0x34b15d2bdd589884741d5f2b0e4be29a5dcdc469c0cc27ac7e31f807efaeacb9",
			"logsBloom": "0x00a04000000000000022100080001080000040002000420000800000000010008000410000001800010000400041400c0008000301150000400000000022600001400000801002220001000808000221601300000100000000000c0040000000000100200a1200008000400000000800000000000200100040100111000400408800050000410040400202004000000008000444000811080000204400000080020000040000000041000000020000000000000033000000400011102200000000000102000001000000000000108e00000020400208001000200400000021010110000200000820410000004000020001200000080804480160600800080000",
			"miner": "0x1284214b9b9c85549ab3d2b972df0deef66ac2c9",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x913638",
			"parentHash": "0x017203a062da40a61596fff8df385e1803a37d912e9443c8ca274946edbe12f2",
			"receiptsRoot": "0xa38d1c3c3deea5647a2a397f51aa52c70af96492dd945033b1ea190f4661e1ea",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xd6350b59463b0f85e66ce7baf8f2d11c4861377ecefe31b79db3613f4ef22a52",
			"timestamp": "0x60bdbb95",
			"totalDifficulty": "0x1211b49",
			"transactionsRoot": "0x6035819fcd17bc49a5f3035bd056d54258bcaaedff556ce4915c49f506c8fea7"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010a02846765746888676f312e31352e35856c696e75780000001600553d2e38fac35164660fa0ee02276627713ffd348ac3d84da664c5e1a84fc92c96a30109c7c0b56ff5f0d509f354424ad5beb58cdf2e107699bb5261b63ca8bb1b1c01",
			"gasLimit": "0x1c9c1b6",
			"gasUsed": "0x2ef79",
			"hash": "0x3474ad7643d2f7a5fcfcd4c4cf58ebf5bebd278fc6e54c58ed2b2ac3638271de",
			"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002010000000000000000000000000000000000020000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000800000000000000080000000000000000000000002010000000000000000000000000000000000000000000000",
			"miner": "0x35552c16704d214347f29fa77f77da6d75d7c752",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x913639",
			"parentHash": "0x34b15d2bdd589884741d5f2b0e4be29a5dcdc469c0cc27ac7e31f807efaeacb9",
			"receiptsRoot": "0x76ba0a3fd5f4730528a08b7353b659d066c46bf602e0f6952a261c4202733661",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x2b78dda7c0b808ee9d5b8151bfd3a60b1a58269c8f8b463284f5a4690f43650b",
			"timestamp": "0x60bdbb98",
			"totalDifficulty": "0x1211b4b",
			"transactionsRoot": "0x055d4a0f0fdf0ae45fed0e98df2bf76b3c6c52a86cde638e6387bff40fb656e5"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010006846765746888676f312e31352e35856c696e75780000001600553d6d3988c21ec42f01b7c92a55ba1388d19587eb3740fec47a7f70c69021049d3978b2e055212959e1694a1636fb95e6302a6c404c380928e88eaa12e2b7f0055700",
			"gasLimit": "0x1c9c380",
			"gasUsed": "0xb991",
			"hash": "0x1cee01e6f8ce42c1b2539e3d31e4a4f4687b991e44efc792dd5178cbb7c13c54",
			"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002010000000000000000000000000000000000020000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000480000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000",
			"miner": "0x3679479c2402e921db00923e014cd439c606c596",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x91363a",
			"parentHash": "0x3474ad7643d2f7a5fcfcd4c4cf58ebf5bebd278fc6e54c58ed2b2ac3638271de",
			"receiptsRoot": "0x1493fee3f10464b3726beed5e508a14394ee0fa3bf4fce1e1f09361280e66778",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xffa5ea329d920b70effd115b44a465a77d6efbb130bc08152e010afd6c823984",
			"timestamp": "0x60bdbb9b",
			"totalDifficulty": "0x1211b4d",
			"transactionsRoot": "0xe426d705ad680875a68e12f591ea16793cbde303f38676ef511601bf5a1114ba"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010007846765746888676f312e31352e38856c696e75780000001600553d51a38238db7900a9a570de3d58abf15e2b5fcf40ce3e60a45ed1707088148be61dc9053a83d6eae2f7a92779cc8554ce96af5e2eb532207c64fd6eb0eb037aba00",
			"gasLimit": "0x1c9c380",
			"gasUsed": "0x6e327",
			"hash": "0xe03a7e316914333134299eb3b74f8853d7c0de416de839a0fa48347f868f644a",
			"logsBloom": "0x0100000000100000000040000001000002000000040000000000001000000020100000000000000000080000000108000a000000200000000000000000240000000000000000000000100008000000002018000001040000000000000002000000200020020200000000000020400820000008000000100000000010000000000000000000018000400800000000001000001400010800000000000000800000020000000000000000000000000000000000000000000080000000002000100020000002000000800280000000040000000000000008000022000000002060000110000000000000010000000000000040000000400000000000010000000000",
			"miner": "0x7a1a4ad9cc746a70ee58568466f7996dd0ace4e8",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x91363b",
			"parentHash": "0x1cee01e6f8ce42c1b2539e3d31e4a4f4687b991e44efc792dd5178cbb7c13c54",
			"receiptsRoot": "0x79f6fc41f28df8960cc1758f4cd6f7c30202b3411d58084248fceb2105f3cd03",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x352ede79848c26875970389970ea265feae15514472c4336e37647e8f3f03258",
			"timestamp": "0x60bdbb9e",
			"totalDifficulty": "0x1211b4f",
			"transactionsRoot": "0x34e5386aaed532c1b9a2be24b2e3a16ded8a6fed18eb7c0924d5ab2455b4ddd5"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010006846765746888676f312e31352e35856c696e75780000001600553df1e86c16a7f5c32d87b4717d1ca0a2df81da61f4f0316decb6f93afc0b50d5ad6471c8382332991f1877aedfda846758aa323d50dc8f7b7465a044bbd44e3fd501",
			"gasLimit": "0x1c9c380",
			"gasUsed": "0x947c3",
			"hash": "0xdec8d3e6f941abb1342cae0b31fcd29df3582e86518691995cb4fe6da094a26c",
			"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000002010000000000000000000000000000000000020000200000000000000020000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000",
			"miner": "0x96c5d20b2a975c050e4220be276ace4892f4b41a",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x91363c",
			"parentHash": "0xe03a7e316914333134299eb3b74f8853d7c0de416de839a0fa48347f868f644a",
			"receiptsRoot": "0x752e53876a0a9737065ff7ee62527f757ccbcefd9c9b866fa7edc6311844fd0a",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0x3154975e092d9a613dc695cf97019bcf697455f839c5aa880189b1695e31987e",
			"timestamp": "0x60bdbba1",
			"totalDifficulty": "0x1211b51",
			"transactionsRoot": "0x2748038d0f7b4e8d8898c005c36dc1bf94f570c9f345587bbcc6d3ddf00d395b"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010a02846765746888676f312e31352e35856c696e75780000001600553da8f7afce6ac0d9d4906b34ff2f6bb4c46f9b7d26917a87ff2581078fb030286149d59244743d1d7b2f94495853abf8e235085cf331eb92e1b4348c44f24bc87301",
			"gasLimit": "0x1c9c380",
			"gasUsed": "0x2ee8a0",
			"hash": "0x8145d94bf46170c92f1f8fafc303b4d6bde95b1b03f884b2ac289be52a2a0bad",
			"logsBloom": "0x0000000000080400000000200000000000000000800040400000000000000800000140010000000000001000000010000000000000000000000000000000040000108000000000000000000800000000201000000000000000100000000000000000042002021000004000000000080000040000000000000010001010000000000000a000000000000000004001000810000c00000000000000000000000000000000000008000000000000400002000000000000000000400000000000040000000002000000000800000000000000040000000000000000000000000020000000000000000000010000000080600000000000000000200100000000000000",
			"miner": "0x980a75ecd1309ea12fa2ed87a8744fbfc9b863d5",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x91363d",
			"parentHash": "0xdec8d3e6f941abb1342cae0b31fcd29df3582e86518691995cb4fe6da094a26c",
			"receiptsRoot": "0xcae8041526afbf5b3a48999d2b1899f88ccddadb282bc478b8f1bc0ee0391263",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xbf3dd879dea5eb15f906f12e2b4f6146f1838f39cac6a9aa82476d98843a144c",
			"timestamp": "0x60bdbba4",
			"totalDifficulty": "0x1211b53",
			"transactionsRoot": "0x9b4b2eac1f62e7075f1dbe3ebf0008e882980beb1ff4bacf4d81d11454f41106"
		}"#,
		r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010a02846765746888676f312e31352e35856c696e75780000001600553d56e3e9141dfa2815b0c00f6291d628ed6d0191d5b9eb1a39e2e8dc70aefb9ba657c5275de292123824766bde04b764b07ad247f2dc037eb83e51e99d604b9f3000",
			"gasLimit": "0x1c9c380",
			"gasUsed": "0x19a78",
			"hash": "0xb0fd7a7713b1682065971552a9febe40cbfe004415d9c9cdb04036e521348979",
			"logsBloom": "0x00000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200200000000000000800000000000000000002010000000000000000000000000000000000020000200000000000000000000000000100000000000000000000000000000000020000000400040000000000000000400000800000000000000000000020000000400000000000000000000000000000000000000000000002000000000000008000000000000000000000000000000000000000000000000000000000010002000000000010000000000000000000000000000000000000000000000",
			"miner": "0xa2959d3f95eae5dc7d70144ce1b73b403b7eb6e0",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x91363e",
			"parentHash": "0x8145d94bf46170c92f1f8fafc303b4d6bde95b1b03f884b2ac289be52a2a0bad",
			"receiptsRoot": "0x38017b9ace6c8ac5a8e91fe47a1b15431811cb850f289a350a3e629b47e0137d",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xd122b9e938c151f4c28ea6d1613e06aca234208bf27e22fc6ac737e7df089626",
			"timestamp": "0x60bdbba7",
			"totalDifficulty": "0x1211b55",
			"transactionsRoot": "0xd3200ece531adcd5fc7e5b8342647507e1fc123da5fb706dddc6b5f75ffa2c4d"
		}"#,
	].iter().map(|json| serde_json::from_str(json).unwrap()).collect::<Vec<BSCHeader>>();

		assert_ok!(BSC::verify_and_update_authority_set_signed(
			Origin::signed(1),
			testnet_headers_9516600_to_9516606,
		));
	})
}
