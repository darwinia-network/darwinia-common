// --- paritytech ---
use frame_support::PalletId;
use sp_runtime::Perbill;
// --- darwinia-network ---
use crate::*;
use darwinia_bridge_ethereum::Config;
use ethereum_primitives::EthereumNetwork;

frame_support::parameter_types! {
	pub const EthereumRelayPalletId: PalletId = PalletId(*b"da/ethrl");
	pub const EthereumRelayBridgeNetwork: EthereumNetwork = EthereumNetwork::Ropsten;
	pub const ConfirmPeriod: BlockNumber = 30;
	pub const ApproveThreshold: Perbill = Perbill::from_percent(60);
	pub const RejectThreshold: Perbill = Perbill::from_percent(1);
}

impl Config for Runtime {
	type ApproveOrigin = ApproveOrigin;
	type ApproveThreshold = ApproveThreshold;
	type BridgedNetwork = EthereumRelayBridgeNetwork;
	type Call = Call;
	type ConfirmPeriod = ConfirmPeriod;
	type Currency = Ring;
	type Event = Event;
	type PalletId = EthereumRelayPalletId;
	type RejectOrigin = EnsureRootOrHalfTechnicalComittee;
	type RejectThreshold = RejectThreshold;
	type RelayerGame = EthereumRelayerGame;
	type TechnicalMembership = TechnicalMembership;
	type WeightInfo = ();
}
