use crate::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free_ring: Balance,
	pub free_kton: Balance,
	pub reserved_ring: Balance,
	pub reserved_kton: Balance,
}

impl pallet_support::balance::BalanceInfo<Balance, KtonInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_kton
	}

	fn reserved(&self) -> Balance {
		self.reserved_kton
	}

	fn mutate_free(&mut self, new_free: Balance) {
		self.free_kton = new_free;
	}

	fn mutate_reserved(&mut self, new_reserved: Balance) {
		self.reserved_kton = new_reserved;
	}

	fn usable(
		&self,
		reasons: pallet_support::balance::lock::LockReasons,
		frozen_balance: pallet_support::balance::FrozenBalance<Balance>,
	) -> Balance {
		self.free_kton
			.saturating_sub(pallet_support::balance::FrozenBalance::frozen_for(
				reasons,
				frozen_balance,
			))
	}

	fn total(&self) -> Balance {
		self.free_kton.saturating_add(self.reserved_kton)
	}
}

impl pallet_support::balance::BalanceInfo<Balance, RingInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free_ring
	}

	fn reserved(&self) -> Balance {
		self.reserved_ring
	}

	fn mutate_free(&mut self, new_free: Balance) {
		self.free_ring = new_free;
	}

	fn mutate_reserved(&mut self, new_reserved: Balance) {
		self.reserved_ring = new_reserved;
	}

	fn usable(
		&self,
		reasons: pallet_support::balance::lock::LockReasons,
		frozen_balance: pallet_support::balance::FrozenBalance<Balance>,
	) -> Balance {
		self.free_ring
			.saturating_sub(pallet_support::balance::FrozenBalance::frozen_for(
				reasons,
				frozen_balance,
			))
	}

	fn total(&self) -> Balance {
		self.free_ring.saturating_add(self.reserved_ring)
	}
}
