use crate::*;
use pallet_support::balance::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	pub free: Balance,
	pub reserved: Balance,
	pub free_kton: Balance,
	pub reserved_kton: Balance,
}

impl BalanceInfo<Balance, RingInstance> for AccountData<Balance> {
	fn free(&self) -> Balance {
		self.free
	}
	fn set_free(&mut self, new_free: Balance) {
		self.free = new_free;
	}

	fn reserved(&self) -> Balance {
		self.reserved
	}
	fn set_reserved(&mut self, new_reserved: Balance) {
		self.reserved = new_reserved;
	}

	fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}

	fn usable(
		&self,
		reasons: lock::LockReasons,
		frozen_balance: FrozenBalance<Balance>,
	) -> Balance {
		self.free
			.saturating_sub(frozen_balance.frozen_for(reasons))
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
			.saturating_sub(frozen_balance.frozen_for(reasons))
	}
}
