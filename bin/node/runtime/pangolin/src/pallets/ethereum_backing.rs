// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_backing::Config;

frame_support::parameter_types! {
	pub const EthereumBackingPalletId: PalletId = PalletId(*b"da/ethbk");
	pub const EthereumBackingFeePalletId: PalletId = PalletId(*b"da/ethfe");
	pub const RingLockLimit: Balance = 10_000_000 * COIN;
	pub const KtonLockLimit: Balance = 1000 * COIN;
	pub const AdvancedFee: Balance = 50 * COIN;
	pub const SyncReward: Balance = 1000 * COIN;
}
impl Config for Runtime {
	type PalletId = EthereumBackingPalletId;
	type FeePalletId = EthereumBackingFeePalletId;
	type Event = Event;
	type RedeemAccountId = AccountId;
	type EthereumRelay = EthereumRelay;
	type OnDepositRedeem = Staking;
	type RingCurrency = Ring;
	type KtonCurrency = Kton;
	type RingLockLimit = RingLockLimit;
	type KtonLockLimit = KtonLockLimit;
	type AdvancedFee = AdvancedFee;
	type SyncReward = SyncReward;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
}
