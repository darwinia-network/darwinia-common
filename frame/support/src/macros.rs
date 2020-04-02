// TODO: support more currency
#[macro_export]
macro_rules! impl_account_data {
	(
		$(#[$attr:meta])*
		pub struct $sname:ident<Balance$(, $gtype:ident),*>
		for
			$ring_instance:ident,
			$kton_instance:ident
		where
			Balance = $btype:ty
			$(, $gtype_:ident: $gtypebound:ty),*
		{
			$($($oname:ident: $otype:ty),+)?
		}
	) => {
		use darwinia_support::balance::BalanceInfo;

		$(#[$attr:meta])*
		#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
		pub struct $sname<Balance$(, $gtype),*> {
			free: Balance,
			reserved: Balance,
			free_kton: Balance,
			reserved_kton: Balance
			$(, $($oname: $otype),+)?
		}

		impl BalanceInfo<$btype, $ring_instance> for AccountData<$btype> {
			fn free(&self) -> $btype {
				self.free
			}
			fn set_free(&mut self, new_free: $btype) {
				self.free = new_free;
			}

			fn reserved(&self) -> $btype {
				self.reserved
			}
			fn set_reserved(&mut self, new_reserved: $btype) {
				self.reserved = new_reserved;
			}

			fn total(&self) -> $btype {
				self.free.saturating_add(self.reserved)
			}

			fn usable(
				&self,
				reasons: darwinia_support::balance::lock::LockReasons,
				frozen_balance: darwinia_support::balance::FrozenBalance<$btype>,
			) -> $btype {
				self.free.saturating_sub(frozen_balance.frozen_for(reasons))
			}
		}

		impl BalanceInfo<$btype, $kton_instance> for AccountData<$btype> {
			fn free(&self) -> $btype {
				self.free_kton
			}
			fn set_free(&mut self, new_free_kton: $btype) {
				self.free_kton = new_free_kton;
			}

			fn reserved(&self) -> $btype {
				self.reserved_kton
			}
			fn set_reserved(&mut self, new_reserved_kton: $btype) {
				self.reserved_kton = new_reserved_kton;
			}

			fn total(&self) -> $btype {
				self.free_kton.saturating_add(self.reserved_kton)
			}

			fn usable(
				&self,
				reasons: darwinia_support::balance::lock::LockReasons,
				frozen_balance: darwinia_support::balance::FrozenBalance<$btype>,
			) -> $btype {
				self.free_kton.saturating_sub(frozen_balance.frozen_for(reasons))
			}
		}
	};
}
