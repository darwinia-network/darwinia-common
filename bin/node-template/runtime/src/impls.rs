use crate::*;
use pallet_support::balance::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free_ring: Balance,
	pub free_kton: Balance,
	pub reserved_ring: Balance,
	pub reserved_kton: Balance,
}

impl BalanceInfo<Balance, RingInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_ring
	}
	fn set_free(&mut self, new_free: Balance) {
		self.free_ring = new_free;
	}

	fn reserved(&self) -> Balance {
		self.reserved_ring
	}
	fn set_reserved(&mut self, new_reserved: Balance) {
		self.reserved_ring = new_reserved;
	}

	fn total(&self) -> Balance {
		self.free_ring.saturating_add(self.reserved_ring)
	}

	fn usable(
		&self,
		reasons: lock::LockReasons,
		frozen_balance: FrozenBalance<Balance>,
	) -> Balance {
		self.free_ring
			.saturating_sub(FrozenBalance::frozen_for(reasons, frozen_balance))
	}
}

impl BalanceInfo<Balance, KtonInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_kton
	}
	fn set_free(&mut self, new_free: Balance) {
		self.free_kton = new_free;
	}

	fn reserved(&self) -> Balance {
		self.reserved_kton
	}
	fn set_reserved(&mut self, new_reserved: Balance) {
		self.reserved_kton = new_reserved;
	}

	fn total(&self) -> Balance {
		self.free_kton.saturating_add(self.reserved_kton)
	}

	fn usable(
		&self,
		reasons: lock::LockReasons,
		frozen_balance: FrozenBalance<Balance>,
	) -> Balance {
		self.free_kton
			.saturating_sub(FrozenBalance::frozen_for(reasons, frozen_balance))
	}
}
