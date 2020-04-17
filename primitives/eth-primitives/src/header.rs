// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethbloom::Bloom;
use keccak_hash::{keccak, KECCAK_EMPTY_LIST_RLP, KECCAK_NULL_RLP};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
// --- substrate ---
use sp_runtime::RuntimeDebug;
use sp_std::{prelude::*, str::FromStr};
// --- darwinia ---
use crate::*;
use darwinia_support::{fixed_hex_bytes_unchecked, hex_bytes_unchecked};

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
enum Seal {
	/// The seal/signature is included.
	With,
	/// The seal/signature is not included.
	Without,
}

#[derive(Clone, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthHeader {
	pub parent_hash: H256,
	pub timestamp: u64,
	pub number: EthBlockNumber,
	pub author: EthAddress,
	pub transactions_root: H256,
	pub uncles_hash: H256,
	pub extra_data: Bytes,
	pub state_root: H256,
	pub receipts_root: H256,
	pub log_bloom: Bloom,
	pub gas_used: U256,
	pub gas_limit: U256,
	pub difficulty: U256,
	pub seal: Vec<Bytes>,
	pub hash: Option<H256>,
}

impl EthHeader {
	pub fn from_str_unchecked(s: &str) -> Self {
		fn parse_value_unchecked(s: &str) -> &str {
			s.splitn(2, ':')
				.skip(1)
				.next()
				.unwrap_or_default()
				.trim()
				.trim_matches('"')
		}

		fn str_to_u64(s: &str) -> u64 {
			if s.starts_with("0x") {
				u64::from_str_radix(&s[2..], 16).unwrap_or_default()
			} else {
				s.parse().unwrap_or_default()
			}
		}

		let mut s = s
			.trim()
			.trim_start_matches('{')
			.trim_end_matches('}')
			.split(',');
		let mut nested_array = 0u32;
		let mut eth_header = Self::default();
		let mut mix_hash = H256::default();
		let mut nonce = H64::default();
		while let Some(s) = s.next() {
			if s.is_empty() {
				continue;
			}

			if s[s.find(':').unwrap_or_default() + 1..]
				.trim_start()
				.starts_with('[')
				&& !s.ends_with(']')
			{
				nested_array = nested_array.saturating_add(1);
			} else if s.ends_with(']') {
				nested_array = nested_array.saturating_sub(1);
			}

			if nested_array != 0 {
				continue;
			}

			let s = s.trim();
			if s.starts_with("\"difficulty") {
				eth_header.difficulty = str_to_u64(parse_value_unchecked(s)).into();
			} else if s.starts_with("\"extraData") {
				eth_header.extra_data = hex_bytes_unchecked(parse_value_unchecked(s));
			} else if s.starts_with("\"gasLimit") {
				eth_header.gas_limit = str_to_u64(parse_value_unchecked(s)).into();
			} else if s.starts_with("\"gasUsed") {
				eth_header.gas_used = str_to_u64(parse_value_unchecked(s)).into();
			} else if s.starts_with("\"hash") {
				eth_header.hash =
					Some(fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into());
			} else if s.starts_with("\"logsBloom") {
				let s = parse_value_unchecked(s);
				let s = if s.starts_with("0x") { &s[2..] } else { s };
				eth_header.log_bloom = Bloom::from_str(s).unwrap_or_default();
			} else if s.starts_with("\"miner") {
				eth_header.author = fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 20).into();
			} else if s.starts_with("\"mixHash") {
				mix_hash = fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			} else if s.starts_with("\"nonce") {
				nonce = fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 8).into();
			} else if s.starts_with("\"number") {
				eth_header.number = str_to_u64(parse_value_unchecked(s));
			} else if s.starts_with("\"parentHash") {
				eth_header.parent_hash =
					fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			} else if s.starts_with("\"receiptsRoot") {
				eth_header.receipts_root =
					fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			} else if s.starts_with("\"sha3Uncles") {
				eth_header.uncles_hash =
					fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			} else if s.starts_with("\"stateRoot") {
				eth_header.state_root =
					fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			} else if s.starts_with("\"timestamp") {
				eth_header.timestamp = str_to_u64(parse_value_unchecked(s));
			} else if s.starts_with("\"transactionsRoot") {
				eth_header.transactions_root =
					fixed_hex_bytes_unchecked!(parse_value_unchecked(s), 32).into();
			}
		}
		eth_header.seal = vec![rlp::encode(&mix_hash), rlp::encode(&nonce)];

		eth_header
	}
}

impl Default for EthHeader {
	fn default() -> Self {
		EthHeader {
			parent_hash: H256::zero(),
			timestamp: 0,
			number: 0,
			author: EthAddress::zero(),
			transactions_root: KECCAK_NULL_RLP,
			uncles_hash: KECCAK_EMPTY_LIST_RLP,
			extra_data: vec![],
			state_root: KECCAK_NULL_RLP,
			receipts_root: KECCAK_NULL_RLP,
			log_bloom: Bloom::default(),
			gas_used: U256::default(),
			gas_limit: U256::default(),
			difficulty: U256::default(),
			seal: vec![],
			hash: None,
		}
	}
}

impl PartialEq for EthHeader {
	fn eq(&self, c: &EthHeader) -> bool {
		if let (&Some(ref h1), &Some(ref h2)) = (&self.hash, &c.hash) {
			if h1 == h2 {
				return true;
			}
		}

		self.parent_hash == c.parent_hash
			&& self.timestamp == c.timestamp
			&& self.number == c.number
			&& self.author == c.author
			&& self.transactions_root == c.transactions_root
			&& self.uncles_hash == c.uncles_hash
			&& self.extra_data == c.extra_data
			&& self.state_root == c.state_root
			&& self.receipts_root == c.receipts_root
			&& self.log_bloom == c.log_bloom
			&& self.gas_used == c.gas_used
			&& self.gas_limit == c.gas_limit
			&& self.difficulty == c.difficulty
			&& self.seal == c.seal
	}
}

impl Decodable for EthHeader {
	fn decode(r: &Rlp) -> Result<Self, DecoderError> {
		let mut blockheader = EthHeader {
			parent_hash: r.val_at(0)?,
			uncles_hash: r.val_at(1)?,
			author: r.val_at(2)?,
			state_root: r.val_at(3)?,
			transactions_root: r.val_at(4)?,
			receipts_root: r.val_at(5)?,
			log_bloom: r.val_at(6)?,
			difficulty: r.val_at(7)?,
			number: r.val_at(8)?,
			gas_limit: r.val_at(9)?,
			gas_used: r.val_at(10)?,
			timestamp: r.val_at(11)?,
			extra_data: r.val_at(12)?,
			seal: vec![],
			hash: keccak(r.as_raw()).into(),
		};

		for i in 13..r.item_count()? {
			blockheader.seal.push(r.at(i)?.as_raw().to_vec())
		}

		Ok(blockheader)
	}
}

impl Encodable for EthHeader {
	fn rlp_append(&self, s: &mut RlpStream) {
		self.stream_rlp(s, Seal::With);
	}
}

/// Alter value of given field, reset memoised hash if changed.
fn change_field<T>(hash: &mut Option<H256>, field: &mut T, value: T)
where
	T: PartialEq<T>,
{
	if field != &value {
		*field = value;
		*hash = None;
	}
}

impl EthHeader {
	/// Create a new, default-valued, header.
	pub fn new() -> Self {
		Self::default()
	}

	/// Get the parent_hash field of the header.
	pub fn parent_hash(&self) -> &H256 {
		&self.parent_hash
	}

	/// Get the timestamp field of the header.
	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	/// Get the number field of the header.
	pub fn number(&self) -> EthBlockNumber {
		self.number
	}

	/// Get the author field of the header.
	pub fn author(&self) -> &EthAddress {
		&self.author
	}

	/// Get the extra data field of the header.
	pub fn extra_data(&self) -> &Bytes {
		&self.extra_data
	}

	/// Get the state root field of the header.
	pub fn state_root(&self) -> &H256 {
		&self.state_root
	}

	/// Get the receipts root field of the header.
	pub fn receipts_root(&self) -> &H256 {
		&self.receipts_root
	}

	/// Get the log bloom field of the header.
	pub fn log_bloom(&self) -> &Bloom {
		&self.log_bloom
	}

	/// Get the transactions root field of the header.
	pub fn transactions_root(&self) -> &H256 {
		&self.transactions_root
	}

	/// Get the uncles hash field of the header.
	pub fn uncles_hash(&self) -> &H256 {
		&self.uncles_hash
	}

	/// Get the gas used field of the header.
	pub fn gas_used(&self) -> &U256 {
		&self.gas_used
	}

	/// Get the gas limit field of the header.
	pub fn gas_limit(&self) -> &U256 {
		&self.gas_limit
	}

	/// Get the difficulty field of the header.
	pub fn difficulty(&self) -> &U256 {
		&self.difficulty
	}

	/// Get the seal field of the header.
	pub fn seal(&self) -> &[Bytes] {
		&self.seal
	}

	/// Set the seal field of the header.
	pub fn set_seal(&mut self, a: Vec<Bytes>) {
		change_field(&mut self.hash, &mut self.seal, a)
	}

	/// Set the difficulty field of the header.
	pub fn set_difficulty(&mut self, a: U256) {
		change_field(&mut self.hash, &mut self.difficulty, a);
	}

	/// Get & memoize the hash of this header (keccak of the RLP with seal).
	pub fn compute_hash(&mut self) -> H256 {
		let hash = self.hash();
		self.hash = Some(hash);
		hash
	}

	pub fn re_compute_hash(&self) -> H256 {
		keccak_hash::keccak(self.rlp(Seal::With))
	}

	/// Get the hash of this header (keccak of the RLP with seal).
	pub fn hash(&self) -> H256 {
		self.hash
			.unwrap_or_else(|| keccak_hash::keccak(self.rlp(Seal::With)))
	}

	/// Get the hash of the header excluding the seal
	pub fn bare_hash(&self) -> H256 {
		keccak_hash::keccak(self.rlp(Seal::Without))
	}

	/// Encode the header, getting a type-safe wrapper around the RLP.
	pub fn encoded(&self) -> encoded::Header {
		encoded::Header::new(self.rlp(Seal::With))
	}

	/// Get the RLP representation of this Header.
	fn rlp(&self, with_seal: Seal) -> Bytes {
		let mut s = RlpStream::new();
		self.stream_rlp(&mut s, with_seal);
		s.out()
	}

	/// Place this header into an RLP stream `s`, optionally `with_seal`.
	fn stream_rlp(&self, s: &mut RlpStream, with_seal: Seal) {
		if let Seal::With = with_seal {
			s.begin_list(13 + self.seal.len());
		} else {
			s.begin_list(13);
		}

		s.append(&self.parent_hash);
		s.append(&self.uncles_hash);
		s.append(&self.author);
		s.append(&self.state_root);
		s.append(&self.transactions_root);
		s.append(&self.receipts_root);
		s.append(&self.log_bloom);
		s.append(&self.difficulty);
		s.append(&self.number);
		s.append(&self.gas_limit);
		s.append(&self.gas_used);
		s.append(&self.timestamp);
		s.append(&self.extra_data);

		if let Seal::With = with_seal {
			for b in &self.seal {
				s.append_raw(b, 1);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	// --- darwinia ---
	use super::*;
	use error::BlockError;
	use pow::EthashPartial;

	#[inline]
	fn sequential_header() -> (EthHeader, EthHeader) {
		(
			EthHeader::from_str_unchecked(
				r#"
				{
					"difficulty": "0x92ac28cbc4930",
					"extraData": "0x5050594520686976656f6e2d6574682d6672",
					"gasLimit": "0x989631",
					"gasUsed": "0x986d77",
					"hash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
					"logsBloom": "0x0c7b091bc8ec02401ad12491004e3014e8806390031950181c118580ac61c9a00409022c418162002710a991108a11ca5383d4921d1da46346edc3eb8068481118b005c0b20700414c13916c54011a0922904aa6e255406a33494c84a1426410541819070e04852042410b30030d4c88a5103082284c7d9bd42090322ae883e004224e18db4d858a0805d043e44a855400945311cb253001412002ea041a08e30394fc601440310920af2192dc4194a03302191cf2290ac0c12000815324eb96a08000aad914034c1c8eb0cb39422e272808b7a4911989c306381502868820b4b95076fc004b14dd48a0411024218051204d902b80d004c36510400ccb123084",
					"miner": "0x4c549990a7ef3fea8784406c1eecc98bf4211fa5",
					"mixHash": "0x543bc0769f7d5df30e7633f4a01552c2cee7baace8a6da37fddaa19e49e81209",
					"nonce": "0xa5d3d0ccc8bb8a29",
					"number": "0x8947a9",
					"parentHash": "0x0b2d720b8d3b6601e4207ef926b0c228735aa1d58301a23d58f9cb51ac2288d8",
					"receiptsRoot": "0x5968afe6026e673df3b9745d925a5648282d2195a46c22771fec48210daf8e23",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"size": "0x93f7",
					"stateRoot": "0x4ba0fb3e6f4c1af32a799df667d304bcdb7f8154e6f86831f92f5a354c2baf70",
					"timestamp": "0x5ddb67a0",
					"totalDifficulty": "0x2c10c70159db491d5d8",
					"transactions": [omitted],
					"transactionsRoot": "0x07d44fadb4aca78c81698710211c5399c1408bb3f0aa3a687d091d230fcaddc6",
					"uncles": [omitted]
				}
			"#,
			),
			EthHeader::from_str_unchecked(
				r#"
				{
					"difficulty": "0x92c07e50de0b9",
					"extraData": "0x7575706f6f6c2e636e2d3163613037623939",
					"gasLimit": "0x98700d",
					"gasUsed": "0x98254e",
					"hash": "0xb972df738904edb8adff9734eebdcb1d3b58fdfc68a48918720a4a247170f15e",
					"logsBloom": "0x0c0110a00144a0082057622381231d842b8977a98d1029841000a1c21641d91946594605e902a5432000159ad24a0300428d8212bf4d1c81c0f8478402a4a818010011437c07a112080e9a4a14822311a6840436f26585c84cc0d50693c148bf9830cf3e0a08970788a4424824b009080d52372056460dec808041b68ea04050bf116c041f25a3329d281068740ca911c0d4cd7541a1539005521694951c286567942d0024852080268d29850000954188f25151d80e4900002122c01ad53b7396acd34209c24110b81b9278642024603cd45387812b0696d93992829090619cf0b065a201082280812020000430601100cb08a3808204571c0e564d828648fb",
					"miner": "0xd224ca0c819e8e97ba0136b3b95ceff503b79f53",
					"mixHash": "0x0ea8027f96c18f474e9bc74ff71d29aacd3f485d5825be0a8dde529eb82a47ed",
					"nonce": "0x55859dc00728f99a",
					"number": "0x8947aa",
					"parentHash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
					"receiptsRoot": "0x3fbd99e253ff45045eec1e0011ac1b45fa0bccd641a356727defee3b166dd3bf",
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"size": "0x8a17",
					"stateRoot": "0x5dfc6357dda61a7f927292509afacd51453ff158342eb9628ccb419fbe91c638",
					"timestamp": "0x5ddb67a3",
					"totalDifficulty": "0x2c10c7941a5999fb691",
					"transactions": [omitted],
					"transactionsRoot": "0xefebac0e71cc2de04cf2f509bb038a82bbe92a659e010061b49b5387323b5ea6",
					"uncles": [omitted]
				}
				"#,
			),
		)
	}

	#[inline]
	fn ropsten_sequential_header() -> (EthHeader, EthHeader) {
		(
			EthHeader::from_str_unchecked(
				r#"
			{
				"author": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
				"difficulty": "0xf4009f4b",
				"extraData": "0xd983010906846765746889676f312e31312e3133856c696e7578",
				"gasLimit": "0x7a1200",
				"gasUsed": "0x769975",
				"hash": "0x1dafbf6a9825241ea5dfa7c3a54781c0784428f2ef3b588748521f83209d3caa",
				"logsBloom": "0x0420000400000018000400400402044000088100000088000000010000040800202000002000a0000000000200004000800100000200000000000020003400000000000004002000000000080102004400000000010400008001000000000020000000009200100000000000004408040100000010000010022002130002000600048200000000000000004000002410000008000000000008021800100000000704010008080000200081000000004002000000009010c000010082000040400104020200000000040180000000000a803000000000002212000000000061000010000001010000400020000000002000020008008100040000005200000000",
				"miner": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
				"mixHash": "0xc4b28f4b671b2e675634f596840d3115ce3df0ab38b6608a69371da16a3455aa",
				"nonce": "0x7afbefa403b138fa",
				"number": "0x69226b",
				"parentHash": "0x8a18726cacb45b078bfe6491510cfa2dd578a70be2a217f416253cf3e94adbd2",
				"receiptsRoot": "0x9c9eb20b6f9176864630f84aa11f33969a355efa85b2eb1e386a5b1ea3599089",
				"sealFields": [
					"0xa0c4b28f4b671b2e675634f596840d3115ce3df0ab38b6608a69371da16a3455aa",
					"0x887afbefa403b138fa"
				],
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x83f4",
				"stateRoot": "0xde1df18f7da776a86119d17373d252d3591b5a4270e14113701d27c852d25313",
				"timestamp": "0x5de5246c",
				"totalDifficulty": "0x66728874bd82ce",
				"transactions": [omitted],
				"transactionsRoot": "0xe3ab46e9eeb65fea6b0b1ffd07587f3ee7741b66f16a0b63a3b0c01900387833",
				"uncles": [omitted]
			}
			"#,
			),
			EthHeader::from_str_unchecked(
				r#"
				{
					"author": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
					"difficulty": "0xf3c49f25",
					"extraData": "0xd983010906846765746889676f312e31312e3133856c696e7578",
					"gasLimit": "0x7a1200",
					"gasUsed": "0x702566",
					"hash": "0x21fe7ebfb3639254a0867995f3d490e186576b42aeea8c60f8e3360c256f7974",
					"logsBloom": "0x8211a0050000250240000000010200402002800012890000600004000208230500042a400000000001000040c00080001001100000002000001004004012000010006200800900a03002510844010014a0000000010408600444200000200080000410001a00140004008000150108108000003010126a0110828010810000000200010000800011001000062040221422249420c1040a940002000000400840080000810000800000400000010408000002001018002200020040000000a00000804002800008000000000080800020082002000000002810054100500020000288240880290000510020000204c0304000000000000820088c800200000000",
					"miner": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
					"mixHash": "0x5a85e328a8bb041a386ffb25db029b7f0df4665a8a55b331b30a576761404fa6",
					"nonce": "0x650ea83006bb108d",
					"number": "0x69226c",
					"parentHash": "0x1dafbf6a9825241ea5dfa7c3a54781c0784428f2ef3b588748521f83209d3caa",
					"receiptsRoot": "0xb2f020ce6615246a711bed61f2f485833943adb734d8e1cddd93d7ae8a641451",
					"sealFields": [
						"0xa05a85e328a8bb041a386ffb25db029b7f0df4665a8a55b331b30a576761404fa6",
						"0x88650ea83006bb108d"
					],
					"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
					"size": "0x75e4",
					"stateRoot": "0xee6ad25ad26e79004f15b8d423a9952859983ad740924fd13165d6e20953ff3e",
					"timestamp": "0x5de52488",
					"totalDifficulty": "0x667289688221f3",
					"transactions": [omitted],
					"transactionsRoot": "0xcd2672df775af7bcb2b93a478666d500dee3d78e6970c71071dc79642db24719",
					"uncles": [omitted]
				}
				"#,
			),
		)
	}

	#[test]
	fn test_mainet_header_bare_hash() {
		let header = EthHeader::from_str_unchecked(
			r#"
			{
				"difficulty": "0x92ac28cbc4930",
				"extraData": "0x5050594520686976656f6e2d6574682d6672",
				"gasLimit": "0x989631",
				"gasUsed": "0x986d77",
				"hash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
				"logsBloom": "0x0c7b091bc8ec02401ad12491004e3014e8806390031950181c118580ac61c9a00409022c418162002710a991108a11ca5383d4921d1da46346edc3eb8068481118b005c0b20700414c13916c54011a0922904aa6e255406a33494c84a1426410541819070e04852042410b30030d4c88a5103082284c7d9bd42090322ae883e004224e18db4d858a0805d043e44a855400945311cb253001412002ea041a08e30394fc601440310920af2192dc4194a03302191cf2290ac0c12000815324eb96a08000aad914034c1c8eb0cb39422e272808b7a4911989c306381502868820b4b95076fc004b14dd48a0411024218051204d902b80d004c36510400ccb123084",
				"miner": "0x4c549990a7ef3fea8784406c1eecc98bf4211fa5",
				"mixHash": "0x543bc0769f7d5df30e7633f4a01552c2cee7baace8a6da37fddaa19e49e81209",
				"nonce": "0xa5d3d0ccc8bb8a29",
				"number": "0x8947a9",
				"parentHash": "0x0b2d720b8d3b6601e4207ef926b0c228735aa1d58301a23d58f9cb51ac2288d8",
				"receiptsRoot": "0x5968afe6026e673df3b9745d925a5648282d2195a46c22771fec48210daf8e23",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x93f7",
				"stateRoot": "0x4ba0fb3e6f4c1af32a799df667d304bcdb7f8154e6f86831f92f5a354c2baf70",
				"timestamp": "0x5ddb67a0",
				"totalDifficulty": "0x2c10c70159db491d5d8",
				"transactions": [omitted],
				"transactionsRoot": "0x07d44fadb4aca78c81698710211c5399c1408bb3f0aa3a687d091d230fcaddc6",
				"uncles": [omitted]
			}
			"#,
		);

		assert_eq!(
			header.hash(),
			fixed_hex_bytes_unchecked!(
				"0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
				32
			)
			.into()
		);

		let partial_header_hash = header.bare_hash();
		assert_eq!(
			partial_header_hash,
			fixed_hex_bytes_unchecked!(
				"0x3c2e6623b1de8862a927eeeef2b6b25dea6e1d9dad88dca3c239be3959dc384a",
				32
			)
			.into()
		);
	}

	#[test]
	fn test_ropsten_header_bare_hash() {
		let header = EthHeader::from_str_unchecked(
			r#"
			{
				"author": "0x1ad857f27200aec56ebb68283f91e6ac1086ad62",
				"difficulty": "0x6648e9e",
				"extraData": "0xd783010503846765746887676f312e372e33856c696e7578",
				"gasLimit": "0x47d629",
				"gasUsed": "0x182a8",
				"hash": "0xa83130084c3570d9e0432bbfd656b0fe6088d8837967ef552974de5e8dc1fad5",
				"logsBloom": "0x00000100000000100000000000000000000000000000000000000000000000000000008000000000000000000000000004000000000000000000000000000000000000000000000400400000000000000000000000000000000000000010000000000000000000000000000000000000200000000000010000000000000000000000000000000000000000000008000000000000000000000000800000000000000000000000000000000000000000000200000000000000000000000000000000000040000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000002000000000000000000000",
				"miner": "0x1ad857f27200aec56ebb68283f91e6ac1086ad62",
				"mixHash": "0x341e3bcf01c921963933253e0cf937020db69206f633e31e0d1c959cdd1188f5",
				"nonce": "0x475ddd90b151f305",
				"number": "0x11170",
				"parentHash": "0xe7a8c03a03f7c055599def00f21686d3b9179d272c8110162f012c191d303dad",
				"receiptsRoot": "0xfbbc5695aac7a42699da58878f0a8bb8c096ed95a9b087989c0903114650ca70",
				"sealFields": [
					"0xa0341e3bcf01c921963933253e0cf937020db69206f633e31e0d1c959cdd1188f5",
					"0x88475ddd90b151f305"
				],
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x35d",
				"stateRoot": "0x76565e67622936b6b9eac50f3a9ad940270f1c6d1d9f203fc6af4e0eb67b20fa",
				"timestamp": "0x583f2778",
				"totalDifficulty": "0x69708a12010",
				"transactions": [omitted],
				"transactionsRoot": "0x35ecd6e29d0b8d161bd7863cfa3198e979b451fa637834b96b0da3d8d5d081cf",
				"uncles": [omitted]
			}
			"#,
		);

		let partial_header_hash = header.bare_hash();
		assert_eq!(
			partial_header_hash,
			fixed_hex_bytes_unchecked!(
				"0xbb698ea6e304a7a88a6cd8238f0e766b4f7bf70dc0869bd2e4a76a8e93fffc80",
				32
			)
			.into()
		);
	}

	#[test]
	fn can_do_proof_of_work_verification_fail() {
		let mut header: EthHeader = EthHeader::default();
		header.set_seal(vec![rlp::encode(&H256::zero()), rlp::encode(&H64::zero())]);
		header.set_difficulty(
			U256::from_str("ffffffffffffffffffffffffffffffffffffffffffffaaaaaaaaaaaaaaaaaaaa")
				.unwrap(),
		);

		let ethash_params = EthashPartial::expanse();
		let verify_result = ethash_params.verify_block_basic(&header);

		match verify_result {
			Err(BlockError::InvalidProofOfWork(_)) => {}
			Err(_) => {
				panic!(
					"should be invalid proof of work error (got {:?})",
					verify_result
				);
			}
			_ => {
				panic!("Should be error, got Ok");
			}
		}
	}

	#[test]
	fn can_verify_basic_difficulty() {
		let header = sequential_header().0;
		let ethash_params = EthashPartial::expanse();
		assert_eq!(ethash_params.verify_block_basic(&header), Ok(()));
	}

	#[test]
	fn can_calculate_difficulty_ropsten() {
		let (header1, header2) = ropsten_sequential_header();
		let expected = U256::from_str("f3c49f25").unwrap();
		let ethash_params = EthashPartial::ropsten_testnet();
		//		ethash_params.set_difficulty_bomb_delays(0xc3500, 5000000);
		assert_eq!(
			ethash_params.calculate_difficulty(&header2, &header1),
			expected
		);
	}

	#[test]
	fn can_calculate_difficulty_production() {
		let (header1, header2) = sequential_header();
		let expected = U256::from_str("92c07e50de0b9").unwrap();
		let ethash_params = EthashPartial::production();
		assert_eq!(
			ethash_params.calculate_difficulty(&header2, &header1),
			expected
		);
	}

	#[test]
	fn can_verify_basic_difficulty_production() {
		let header = sequential_header().0;
		let ethash_params = EthashPartial::production();
		assert_eq!(ethash_params.verify_block_basic(&header), Ok(()));
	}
}
