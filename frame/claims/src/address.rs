// --- crates ---
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// --- darwinia ---
use crate::AddressT;
use array_bytes::{fixed_hex_bytes_unchecked, hex_string_unchecked};

macro_rules! impl_address {
	($name:ident, $sname:expr, $prefix:expr) => {
		#[doc = "An "]
		#[doc = $sname]
		#[doc = " address (i.e. 20 bytes, used to represent an "]
		#[doc = $sname]
		#[doc = ".\n\nThis gets serialized to the "]
		#[doc = $prefix]
		#[doc = "-prefixed hex representation."]
		#[derive(Debug, Default)]
		pub struct $name(pub AddressT);

		impl Serialize for $name {
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				let hex: String = hex_string_unchecked(&self.0, $prefix).into_iter().collect();
				serializer.serialize_str(&hex)
			}
		}

		impl<'de> Deserialize<'de> for $name {
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let base_string = String::deserialize(deserializer)?;
				let offset = if base_string.starts_with($prefix) { 2 } else { 0 };
				let s = &base_string[offset..];
				if s.len() != 40 {
					Err(serde::de::Error::custom(
						concat!("Bad length of ", $sname, " address (should be 42 including '", $prefix, "')"),
					))?;
				}

				Ok($name(fixed_hex_bytes_unchecked!(s, 20)))
			}
		}
	};
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Account<Address> {
	pub address: Address,
	pub backed_ring: u128,
}

darwinia_support::impl_genesis! {
	struct ClaimsList {
		dot: Vec<Account<EthereumAddress>>,
		eth: Vec<Account<EthereumAddress>>,
		tron: Vec<Account<TronAddress>>
	}
}

impl_address!(EthereumAddress, "Ethereum", "0x");
impl_address!(TronAddress, "Tron", "41");
