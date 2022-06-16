// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// TODO: support more currency
#[macro_export]
macro_rules! impl_account_data {
	(
		$(#[$attr:meta])*
		$(pub)? struct $sname:ident<Balance$(, $($gtype:tt)*)?>
		for
			$ring_instance:ident,
			$kton_instance:ident
		where
			Balance = $btype:ty
			$(, $($gtypebound:tt)*)?
		{
			$($(pub)? $fname:ident: $ftype:ty),*
		}
	) => {
		use darwinia_balances::{BalanceInfo, FrozenBalance, Reasons};

		$(#[$attr])*
		#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
		pub struct $sname<Balance$(, $($gtype)*)?>
		$(
		where
			$($gtypebound)*
		)?
		{
			pub free: Balance,
			pub reserved: Balance,
			pub free_kton: Balance,
			pub reserved_kton: Balance,
			$(pub $fname: $ftype),*
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
		}

		impl BalanceInfo<$btype, $kton_instance> for AccountData<$btype> {
			fn free(&self) -> $btype { self.free_kton }
			fn set_free(&mut self, new_free_kton: $btype) { self.free_kton = new_free_kton; }

			fn reserved(&self) -> $btype { self.reserved_kton }
			fn set_reserved(&mut self, new_reserved_kton: $btype) { self.reserved_kton = new_reserved_kton; }

			fn total(&self) -> $btype { self.free_kton.saturating_add(self.reserved_kton) }
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
				if !::std::path::Path::new(path).is_file() && ::std::env::var(env_name).is_err() {
					Default::default()
				} else {
					serde_json::from_reader(
						::std::fs::File::open(std::env::var(env_name).unwrap_or(path.to_owned()))
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
