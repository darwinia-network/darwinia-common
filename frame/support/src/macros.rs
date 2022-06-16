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

			fn usable(&self, reasons: Reasons, frozen_balance: FrozenBalance<$btype>) -> $btype {
				self.free.saturating_sub(frozen_balance.frozen_for(reasons))
			}
		}

		impl BalanceInfo<$btype, $kton_instance> for AccountData<$btype> {
			fn free(&self) -> $btype { self.free_kton }
			fn set_free(&mut self, new_free_kton: $btype) { self.free_kton = new_free_kton; }

			fn reserved(&self) -> $btype { self.reserved_kton }
			fn set_reserved(&mut self, new_reserved_kton: $btype) { self.reserved_kton = new_reserved_kton; }

			fn total(&self) -> $btype { self.free_kton.saturating_add(self.reserved_kton) }

			fn usable(&self, reasons: Reasons, frozen_balance: FrozenBalance<$btype>) -> $btype {
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

// TODO: https://github.com/serde-rs/serde/issues/1634
// serde(bound(serialize = concat!(stringify!($ftype), ": core::fmt::Display")))
// serde(bound(deserialize = concat!(stringify!($ftype), ": std::str::FromStr")))
#[macro_export]
macro_rules! impl_runtime_dispatch_info {
	(
		$(pub)? struct $sname:ident$(<$($gtype:ident),+>)? {
			$($(pub)? $fname:ident: $ftype:ty),+
		}
	) => {
		#[cfg(feature = "std")]
		use serde::{Serialize, Serializer};

		#[cfg(not(feature = "std"))]
		#[derive(Default, Eq, PartialEq, Encode, Decode)]
		pub struct $sname$(<$($gtype),+>)? {
			$(
				pub $fname: $ftype
			),+
		}

		#[cfg(feature = "std")]
		#[derive(Debug, Default, Eq, PartialEq, Encode, Decode, Serialize)]
		#[serde(rename_all = "camelCase")]
		pub struct $sname$(<$($gtype),+>)?
		$(
		where
			$($gtype: core::fmt::Debug + core::fmt::Display),+
		)?
		{
			$(
				#[serde(serialize_with = "serialize_as_string")]
				pub $fname: $ftype
			),+
		}

		#[cfg(feature = "std")]
		fn serialize_as_string<S, T>(
			t: &T,
			serializer: S,
		) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
			T: core::fmt::Display
		{
			serializer.serialize_str(&t.to_string())
		}
	};
}

// TODO: https://github.com/serde-rs/serde/issues/1634
#[macro_export]
macro_rules! impl_rpc {
	(
		$(pub)? fn $fnname:ident($($params:tt)*) -> $respname:ident$(<$($gtype:ty),+>)? {
			$($fnbody:tt)*
		}
	) => {
		#[cfg(feature = "std")]
		pub fn $fnname($($params)*) -> $respname$(<$($gtype),+>)?
		$(
		where
			$($gtype: core::fmt::Display + std::str::FromStr),+
		)?
		{
			$($fnbody)*
		}

		#[cfg(not(feature = "std"))]
		pub fn $fnname($($params)*) -> $respname$(<$($gtype),+>)? {
			$($fnbody)*
		}
	};
}
