// TODO: support more currency
#[macro_export]
macro_rules! impl_account_data {
	(
		$(#[$attr:meta])*
		$(pub)? struct $sname:ident<Balance$(, $gtype:ident),*>
		for
			$ring_instance:ident,
			$kton_instance:ident
		where
			Balance = $btype:ty
			$(, $gtype_:ident: $gtypebound:ty),*
		{
			$($($(pub)? $fname:ident: $ftype:ty),+)?
		}
	) => {
		use darwinia_support::balance::BalanceInfo;

		$(#[$attr])*
		#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
		pub struct $sname<Balance$(, $gtype),*> {
			pub free: Balance,
			pub reserved: Balance,
			pub free_kton: Balance,
			pub reserved_kton: Balance
			$(, $(pub $fname: $ftype),+)?
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

#[macro_export]
macro_rules! impl_genesis {
	(
		$(#[$attr:meta])*
		$(pub)? struct $sname:ident {
			$($(pub)? $fname:ident: $ftype:ty),+
		}
	) => {
		$(#[$attr])*
		#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
		pub struct $sname {
			$(pub $fname: $ftype),+
		}

		impl $sname {
			pub fn from_file(path: &str, env_name: &str) -> Self {
				if !std::path::Path::new(path).is_file() && std::env::var(env_name).is_err() {
					Default::default()
				} else {
					serde_json::from_reader(
						std::fs::File::open(std::env::var(env_name).unwrap_or(path.to_owned()))
							.unwrap(),
					)
					.unwrap()
				}
			}

			pub fn from_str(data: &str) -> Self {
				serde_json::from_str(data).unwrap()
			}
		}
	};
}
