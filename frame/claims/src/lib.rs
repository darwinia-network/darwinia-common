//! Module to process claims from Ethereum addresses.

#![cfg_attr(not(feature = "std"), no_std)]

mod address;
mod types {
	use crate::*;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
	// TODO: support *KTON*
	// pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;

	type AccountId<T> = <T as system::Trait>::AccountId;
	type RingCurrency<T> = <T as Trait>::RingCurrency;
	// TODO: support *KTON*
	// type KtonCurrency<T> = <T as Trait>::KtonCurrency;
}

pub use address::{EthereumAddress, TronAddress};

use codec::{Decode, Encode};
use frame_support::{
	traits::{Currency, Get},
	weights::SimpleDispatchInfo,
	{decl_error, decl_event, decl_module, decl_storage},
};
use frame_system::{self as system, ensure_none, ensure_root};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::{
	traits::{CheckedSub, SaturatedConversion, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionLongevity, TransactionValidity, ValidTransaction,
	},
	RuntimeDebug,
};
use sp_std::prelude::*;

use address::AddressT;
use types::*;

#[repr(u8)]
enum ValidityError {
	/// The signature is invalid.
	InvalidSignature = 0,
	/// The signer has no claim.
	SignerHasNoClaim = 1,
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum OtherSignature {
	Eth(EcdsaSignature),
	Tron(EcdsaSignature),
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum OtherAddress {
	Eth(EthereumAddress),
	Tron(TronAddress),
}

#[cfg_attr(feature = "std", derive(Debug, Default, Serialize, Deserialize))]
pub struct ClaimsList {
	pub dot: Vec<Account<EthereumAddress>>,
	pub eth: Vec<Account<EthereumAddress>>,
	pub tron: Vec<Account<TronAddress>>,
}

#[cfg_attr(feature = "std", derive(Debug, Default, Serialize, Deserialize))]
pub struct Account<Address> {
	pub address: Address,
	pub backed_ring: u128,
}

#[derive(Clone, Encode, Decode)]
pub struct EcdsaSignature(pub [u8; 65]);

impl PartialEq for EcdsaSignature {
	fn eq(&self, other: &Self) -> bool {
		&self.0[..] == &other.0[..]
	}
}

impl sp_std::fmt::Debug for EcdsaSignature {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
		write!(f, "EcdsaSignature({:?})", &self.0[..])
	}
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Prefix: Get<&'static [u8]>;

	/// The *RING* currency.
	type RingCurrency: Currency<Self::AccountId>;
	// TODO: support *KTON*
	// /// The *KTON* currency.
	// type KtonCurrency: Currency<Self::AccountId>;
}

decl_event!(
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
		RingBalance = RingBalance<T>,
	{
		/// Someone claimed some *RING*s.
		Claimed(AccountId, AddressT, RingBalance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Invalid Ethereum signature.
		InvalidSignature,
		/// Ethereum address has no claim.
		SignerHasNoClaim,
		/// There's not enough in the pot to pay out some unvested amount. Generally implies a logic
		/// error.
		PotUnderflow,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Claims {
		ClaimsFromEth
			get(claims_from_eth)
			: map hasher(identity) EthereumAddress => Option<RingBalance<T>>;
		ClaimsFromTron
			get(claims_from_tron)
			: map hasher(identity) TronAddress => Option<RingBalance<T>>;

		Total get(total): RingBalance<T>;
	}
	add_extra_genesis {
		config(claims_list): ClaimsList;
		build(|config| {
			let mut total = <RingBalance<T>>::zero();

			// Eth Address
			for Account { address, backed_ring } in &config.claims_list.dot {
				// DOT:RING = 1:50
				let backed_ring = (*backed_ring).saturated_into();
				<ClaimsFromEth<T>>::insert(address, backed_ring);
				total += backed_ring;
			}
			for Account { address, backed_ring } in &config.claims_list.eth {
				let backed_ring = (*backed_ring).saturated_into();
				<ClaimsFromEth<T>>::insert(address, backed_ring);
				total += backed_ring;
			}

			// Tron Address
			for Account { address, backed_ring } in &config.claims_list.tron {
				let backed_ring = (*backed_ring).saturated_into();
				<ClaimsFromTron<T>>::insert(address, backed_ring);
				total += backed_ring;
			}

			<Total<T>>::put(total);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		/// The Prefix that is used in signed Ethereum messages for this network
		const Prefix: &[u8] = T::Prefix::get();

		/// Deposit one of this module's events by using the default implementation.
		fn deposit_event() = default;

		/// Make a claim.
		#[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
		fn claim(origin, dest: T::AccountId, signature: OtherSignature) {
			ensure_none(origin)?;

			let data = dest.using_encoded(to_ascii_hex);

			match signature {
				OtherSignature::Eth(signature) => {
					let signer = Self::eth_recover(&signature, &data)
						.ok_or(<Error<T>>::InvalidSignature)?;
					let balance_due = <ClaimsFromEth<T>>::get(&signer)
						.ok_or(<Error<T>>::SignerHasNoClaim)?;
					let new_total = Self::total()
						.checked_sub(&balance_due)
						.ok_or(<Error<T>>::PotUnderflow)?;

					T::RingCurrency::deposit_creating(&dest, balance_due);
					<ClaimsFromEth<T>>::remove(&signer);
					<Total<T>>::put(new_total);

					Self::deposit_event(RawEvent::Claimed(dest, signer.0, balance_due));
				}
				OtherSignature::Tron(signature) => {
					let signer = Self::tron_recover(&signature, &data)
						.ok_or(<Error<T>>::InvalidSignature)?;
					let balance_due = <ClaimsFromTron<T>>::get(&signer)
						.ok_or(<Error<T>>::SignerHasNoClaim)?;
					let new_total = Self::total()
						.checked_sub(&balance_due)
						.ok_or(<Error<T>>::PotUnderflow)?;

					T::RingCurrency::deposit_creating(&dest, balance_due);
					<ClaimsFromTron<T>>::remove(&signer);
					<Total<T>>::put(new_total);

					Self::deposit_event(RawEvent::Claimed(dest, signer.0, balance_due));
				}
			}
		}

		/// Add a new claim, if you are root.
		#[weight = SimpleDispatchInfo::FixedNormal(30_000)]
		fn mint_claim(origin, who: OtherAddress, value: RingBalance<T>) {
			ensure_root(origin)?;

			match who {
				OtherAddress::Eth(who) => {
					<Total<T>>::mutate(|t| *t += value);
					<ClaimsFromEth<T>>::insert(who, value);
				}
				OtherAddress::Tron(who) => {
					<Total<T>>::mutate(|t| *t += value);
					<ClaimsFromTron<T>>::insert(who, value);
				}
			}
		}
	}
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice the length.
fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
	let mut r = Vec::with_capacity(data.len() * 2);
	let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
	for &b in data.iter() {
		push_nibble(b / 16);
		push_nibble(b % 16);
	}
	r
}

impl<T: Trait> Module<T> {
	// Constructs the message that RPC's `personal_sign` and `sign` would sign.
	fn eth_signable_message(what: &[u8], signed_message: &[u8]) -> Vec<u8> {
		let prefix = T::Prefix::get();
		let mut l = prefix.len() + what.len();
		let mut rev = Vec::new();
		while l > 0 {
			rev.push(b'0' + (l % 10) as u8);
			l /= 10;
		}
		let mut v = signed_message.to_vec();
		v.extend(rev.into_iter().rev());
		v.extend_from_slice(&prefix[..]);
		v.extend_from_slice(what);
		v
	}

	// Constructs the message that RPC's `personal_sign` and `sign` would sign.
	// Tron have different signing specs: https://github.com/tronprotocol/tips/issues/104
	fn tron_signable_message(what: &[u8], signed_message: &[u8]) -> Vec<u8> {
		let prefix = T::Prefix::get();
		let mut l = 32;
		let mut rev = Vec::new();
		while l > 0 {
			rev.push(b'0' + (l % 10) as u8);
			l /= 10;
		}
		let mut v = signed_message.to_vec();
		v.extend(rev.into_iter().rev());
		v.extend_from_slice(&prefix[..]);
		v.extend_from_slice(what);
		v
	}

	// Attempts to recover the Ethereum address from a message signature signed by using
	// the Ethereum RPC's `personal_sign` and `eth_sign`.
	fn eth_recover(s: &EcdsaSignature, what: &[u8]) -> Option<EthereumAddress> {
		let msg = keccak_256(&Self::eth_signable_message(
			what,
			b"\x19Ethereum Signed Message:\n",
		));
		let mut res = EthereumAddress::default();
		res.0
			.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
		Some(res)
	}

	// Attempts to recover the Tron address from a message signature signed by using
	// the Tron RPC's `personal_sign` and `tron_sign`.
	fn tron_recover(s: &EcdsaSignature, what: &[u8]) -> Option<TronAddress> {
		let msg = keccak_256(&Self::tron_signable_message(
			what,
			b"\x19TRON Signed Message:\n",
		));
		let mut res = TronAddress::default();
		res.0
			.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
		Some(res)
	}
}

#[allow(deprecated)] // Allow `ValidateUnsigned`
impl<T: Trait> sp_runtime::traits::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		const PRIORITY: u64 = 100;

		match call {
			Call::claim(account, signature) => {
				let data = account.using_encoded(to_ascii_hex);

				match signature {
					OtherSignature::Eth(signature) => {
						let maybe_signer = Self::eth_recover(&signature, &data);
						let signer = if let Some(s) = maybe_signer {
							s
						} else {
							return InvalidTransaction::Custom(
								ValidityError::InvalidSignature as _,
							)
							.into();
						};

						if !<ClaimsFromEth<T>>::contains_key(&signer) {
							return Err(InvalidTransaction::Custom(
								ValidityError::SignerHasNoClaim as _,
							)
							.into());
						}

						Ok(ValidTransaction {
							priority: PRIORITY,
							requires: vec![],
							provides: vec![("claims", signer).encode()],
							longevity: TransactionLongevity::max_value(),
							propagate: true,
						})
					}
					OtherSignature::Tron(signature) => {
						let maybe_signer = Self::tron_recover(&signature, &data);
						let signer = if let Some(s) = maybe_signer {
							s
						} else {
							return InvalidTransaction::Custom(
								ValidityError::InvalidSignature as _,
							)
							.into();
						};

						if !<ClaimsFromTron<T>>::contains_key(&signer) {
							return Err(InvalidTransaction::Custom(
								ValidityError::SignerHasNoClaim as _,
							)
							.into());
						}

						Ok(ValidTransaction {
							priority: PRIORITY,
							requires: vec![],
							provides: vec![("claims", signer).encode()],
							longevity: TransactionLongevity::max_value(),
							propagate: true,
						})
					}
				}
			}
			_ => Err(InvalidTransaction::Call.into()),
		}
	}
}

#[cfg(test)]
mod tests {
	// --- third-party ---
	use codec::Encode;
	use frame_support::{assert_err, assert_noop, assert_ok, impl_outer_origin, parameter_types};
	use hex_literal::hex;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{BlakeTwo256, IdentityLookup},
		Perbill,
	};
	use tiny_keccak::keccak256;

	// --- custom ---
	use crate::*;

	// --- substrate ---
	type System = frame_system::Module<Test>;

	// --- darwinia ---
	type Ring = pallet_ring::Module<Test>;

	// --- current ---
	type Claims = Module<Test>;

	const ETHEREUM_SIGNED_MESSAGE: &'static [u8] = b"\x19Ethereum Signed Message:\n";
	const TRON_SIGNED_MESSAGE: &'static [u8] = b"\x19TRON Signed Message:\n";

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	parameter_types! {
		pub const BlockHashCount: u32 = 250;
		pub const MaximumBlockWeight: u32 = 4 * 1024 * 1024;
		pub const MaximumBlockLength: u32 = 4 * 1024 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl frame_system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		// The testing primitives are very useful for avoiding having to work with signatures
		// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
		type AccountData = pallet_support::balance::AccountData<u64>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type MigrateAccount = ();
	}

	parameter_types! {
		pub const ExistentialDeposit: u64 = 1;
		pub const CreationFee: u64 = 0;
	}
	impl pallet_ring::Trait for Test {
		type Balance = u64;
		type DustRemoval = ();
		type Event = ();
		type ExistentialDeposit = ExistentialDeposit;
		type AccountStore = System;
		type TryDropKton = ();
	}

	parameter_types! {
		pub const Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
	}
	impl Trait for Test {
		type Event = ();
		type Prefix = Prefix;
		type RingCurrency = Ring;
	}

	fn alice() -> secp256k1::SecretKey {
		secp256k1::SecretKey::parse(&keccak256(b"Alice")).unwrap()
	}
	fn bob() -> secp256k1::SecretKey {
		secp256k1::SecretKey::parse(&keccak256(b"Bob")).unwrap()
	}
	fn carol() -> secp256k1::SecretKey {
		secp256k1::SecretKey::parse(&keccak256(b"Carol")).unwrap()
	}
	fn public(secret: &secp256k1::SecretKey) -> secp256k1::PublicKey {
		secp256k1::PublicKey::from_secret_key(secret)
	}
	fn eth(secret: &secp256k1::SecretKey) -> EthereumAddress {
		let mut res = EthereumAddress::default();
		res.0
			.copy_from_slice(&keccak256(&public(secret).serialize()[1..65])[12..]);
		res
	}
	fn tron(secret: &secp256k1::SecretKey) -> TronAddress {
		let mut res = TronAddress::default();
		res.0
			.copy_from_slice(&keccak256(&public(secret).serialize()[1..65])[12..]);
		res
	}
	fn eth_sig(
		secret: &secp256k1::SecretKey,
		what: &[u8],
		signed_message: &[u8],
	) -> EcdsaSignature {
		let msg = keccak256(&Claims::eth_signable_message(
			&to_ascii_hex(what)[..],
			signed_message,
		));
		let (sig, recovery_id) = secp256k1::sign(&secp256k1::Message::parse(&msg), secret);
		let mut r = [0u8; 65];
		r[0..64].copy_from_slice(&sig.serialize()[..]);
		r[64] = recovery_id.serialize();
		EcdsaSignature(r)
	}

	fn tron_sig(
		secret: &secp256k1::SecretKey,
		what: &[u8],
		signed_message: &[u8],
	) -> EcdsaSignature {
		let msg = keccak256(&Claims::tron_signable_message(
			&to_ascii_hex(what)[..],
			signed_message,
		));
		let (sig, recovery_id) = secp256k1::sign(&secp256k1::Message::parse(&msg), secret);
		let mut r = [0u8; 65];
		r[0..64].copy_from_slice(&sig.serialize()[..]);
		r[64] = recovery_id.serialize();
		EcdsaSignature(r)
	}

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		// We use default for brevity, but you can configure as desired if needed.
		pallet_ring::GenesisConfig::<Test>::default()
			.assimilate_storage(&mut t)
			.unwrap();
		GenesisConfig {
			claims_list: ClaimsList {
				dot: vec![Account {
					address: eth(&alice()),
					backed_ring: 100,
				}],
				eth: vec![Account {
					address: eth(&bob()),
					backed_ring: 200,
				}],
				tron: vec![Account {
					address: tron(&carol()),
					backed_ring: 300,
				}],
			},
		}
		.assimilate_storage::<Test>(&mut t)
		.unwrap();
		t.into()
	}

	#[test]
	fn basic_setup_works() {
		new_test_ext().execute_with(|| {
			assert_eq!(Claims::total(), 600);

			assert_eq!(Claims::claims_from_eth(&eth(&alice())), Some(100));
			assert_eq!(Claims::claims_from_tron(&tron(&alice())), None);

			assert_eq!(Claims::claims_from_eth(&eth(&bob())), Some(200));
			assert_eq!(Claims::claims_from_tron(&tron(&bob())), None);

			assert_eq!(Claims::claims_from_eth(&eth(&carol())), None);
			assert_eq!(Claims::claims_from_tron(&tron(&carol())), Some(300));
		});
	}

	#[test]
	fn serde_works() {
		let x = EthereumAddress(hex!["0123456789abcdef0123456789abcdef01234567"]);
		let y = serde_json::to_string(&x).unwrap();
		assert_eq!(y, "\"0x0123456789abcdef0123456789abcdef01234567\"");
		let z: EthereumAddress = serde_json::from_str(&y).unwrap();
		assert_eq!(x, z);

		let x = TronAddress(hex!["0123456789abcdef0123456789abcdef01234567"]);
		let y = serde_json::to_string(&x).unwrap();
		assert_eq!(y, "\"410123456789abcdef0123456789abcdef01234567\"");
		let z: TronAddress = serde_json::from_str(&y).unwrap();
		assert_eq!(x, z);
	}

	#[test]
	fn claiming_works() {
		new_test_ext().execute_with(|| {
			assert_eq!(Ring::free_balance(1), 0);
			assert_ok!(Claims::claim(
				Origin::NONE,
				1,
				OtherSignature::Eth(eth_sig(&alice(), &1u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
			));
			assert_eq!(Ring::free_balance(&1), 100);
			assert_eq!(Claims::total(), 500);

			assert_eq!(Ring::free_balance(2), 0);
			assert_ok!(Claims::claim(
				Origin::NONE,
				2,
				OtherSignature::Eth(eth_sig(&bob(), &2u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
			));
			assert_eq!(Ring::free_balance(&2), 200);
			assert_eq!(Claims::total(), 300);

			assert_eq!(Ring::free_balance(3), 0);
			assert_ok!(Claims::claim(
				Origin::NONE,
				3,
				OtherSignature::Tron(tron_sig(&carol(), &3u64.encode(), TRON_SIGNED_MESSAGE)),
			));
			assert_eq!(Ring::free_balance(&3), 300);
			assert_eq!(Claims::total(), 0);
		});
	}

	#[test]
	fn add_claim_works() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				Claims::mint_claim(Origin::signed(42), OtherAddress::Eth(eth(&carol())), 200),
				sp_runtime::traits::BadOrigin,
			);
			assert_eq!(Ring::free_balance(42), 0);
			assert_noop!(
				Claims::claim(
					Origin::NONE,
					69,
					OtherSignature::Eth(eth_sig(
						&carol(),
						&69u64.encode(),
						ETHEREUM_SIGNED_MESSAGE
					)),
				),
				<Error<Test>>::SignerHasNoClaim,
			);
			assert_ok!(Claims::mint_claim(
				Origin::ROOT,
				OtherAddress::Eth(eth(&carol())),
				200
			));
			assert_eq!(Claims::total(), 800);
			assert_ok!(Claims::claim(
				Origin::NONE,
				69,
				OtherSignature::Eth(eth_sig(&carol(), &69u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
			));
			assert_eq!(Ring::free_balance(&69), 200);
			assert_eq!(Claims::total(), 600);
		});
	}

	#[test]
	fn origin_signed_claiming_fail() {
		new_test_ext().execute_with(|| {
			assert_eq!(Ring::free_balance(42), 0);
			assert_err!(
				Claims::claim(
					Origin::signed(42),
					42,
					OtherSignature::Eth(eth_sig(
						&alice(),
						&42u64.encode(),
						ETHEREUM_SIGNED_MESSAGE
					)),
				),
				sp_runtime::traits::BadOrigin,
			);
		});
	}

	#[test]
	fn double_claiming_doesnt_work() {
		new_test_ext().execute_with(|| {
			assert_eq!(Ring::free_balance(42), 0);
			assert_ok!(Claims::claim(
				Origin::NONE,
				42,
				OtherSignature::Eth(eth_sig(&alice(), &42u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
			));
			assert_noop!(
				Claims::claim(
					Origin::NONE,
					42,
					OtherSignature::Eth(eth_sig(
						&alice(),
						&42u64.encode(),
						ETHEREUM_SIGNED_MESSAGE
					)),
				),
				<Error<Test>>::SignerHasNoClaim,
			);
		});
	}

	#[test]
	fn non_sender_sig_doesnt_work() {
		new_test_ext().execute_with(|| {
			assert_eq!(Ring::free_balance(42), 0);
			assert_noop!(
				Claims::claim(
					Origin::NONE,
					42,
					OtherSignature::Eth(eth_sig(
						&alice(),
						&69u64.encode(),
						ETHEREUM_SIGNED_MESSAGE
					)),
				),
				<Error<Test>>::SignerHasNoClaim,
			);
		});
	}

	#[test]
	fn non_claimant_doesnt_work() {
		new_test_ext().execute_with(|| {
			assert_eq!(Ring::free_balance(42), 0);
			assert_noop!(
				Claims::claim(
					Origin::NONE,
					42,
					OtherSignature::Eth(eth_sig(
						&carol(),
						&69u64.encode(),
						ETHEREUM_SIGNED_MESSAGE
					)),
				),
				<Error<Test>>::SignerHasNoClaim,
			);
		});
	}

	#[test]
	fn real_eth_sig_works() {
		new_test_ext().execute_with(|| {
				// "Pay RUSTs to the TEST account:2a00000000000000"
				let sig = hex!["444023e89b67e67c0562ed0305d252a5dd12b2af5ac51d6d3cb69a0b486bc4b3191401802dc29d26d586221f7256cd3329fe82174bdf659baea149a40e1c495d1c"];
				let sig = EcdsaSignature(sig);
				let who = 42u64.using_encoded(to_ascii_hex);
				let signer = Claims::eth_recover(&sig, &who).unwrap();
				assert_eq!(signer.0, hex!["6d31165d5d932d571f3b44695653b46dcc327e84"]);
			});
	}

	#[test]
	fn real_tron_sig_works() {
		new_test_ext().execute_with(|| {
			// "Pay RUSTs to the TEST account:0c0529c66a44e1861e5e1502b4a87009f23c792518a7a2091363f5a0e38abd57"
			let sig = hex!["34c3d5afc7f8fa08f9d00a1ec4ac274c63ebce99460b556de85258c94f41ab2f52ad5188bd9fc51251cf5dcdd53751b1bd577828db3f2e8fe8ef77907d7f3f6a1b"];
			let sig = EcdsaSignature(sig);
			let who = hex!["0c0529c66a44e1861e5e1502b4a87009f23c792518a7a2091363f5a0e38abd57"].using_encoded(to_ascii_hex);
			let signer = Claims::tron_recover(&sig, &who).unwrap();
			assert_eq!(signer.0, hex!["11974bce18a43243ede78beec2fd8e0ba4fe17ae"]);
		});
	}

	#[test]
	fn validate_unsigned_works() {
		#![allow(deprecated)] // Allow `ValidateUnsigned`
		use sp_runtime::traits::ValidateUnsigned;

		new_test_ext().execute_with(|| {
			assert_eq!(
				Claims::validate_unsigned(&Call::claim(
					1,
					OtherSignature::Eth(eth_sig(&alice(), &1u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
				)),
				Ok(ValidTransaction {
					priority: 100,
					requires: vec![],
					provides: vec![("claims", eth(&alice())).encode()],
					longevity: TransactionLongevity::max_value(),
					propagate: true,
				})
			);
			assert_eq!(
				Claims::validate_unsigned(&Call::claim(
					0,
					OtherSignature::Eth(EcdsaSignature([0; 65]))
				)),
				InvalidTransaction::Custom(ValidityError::InvalidSignature as _).into(),
			);
			assert_eq!(
				Claims::validate_unsigned(&Call::claim(
					1,
					OtherSignature::Eth(eth_sig(&carol(), &1u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
				)),
				InvalidTransaction::Custom(ValidityError::SignerHasNoClaim as _).into(),
			);
			assert_eq!(
				Claims::validate_unsigned(&Call::claim(
					0,
					OtherSignature::Eth(eth_sig(&carol(), &1u64.encode(), ETHEREUM_SIGNED_MESSAGE)),
				)),
				InvalidTransaction::Custom(ValidityError::SignerHasNoClaim as _).into(),
			);
		});
	}
}
