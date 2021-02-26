// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

#[macro_export]
macro_rules! impl_test_account_data {
	() => {
		pub type RingInstance = darwinia_balances::Instance0;
		pub type RingError = darwinia_balances::Error<Test, RingInstance>;
		pub type RingConfig = darwinia_balances::GenesisConfig<Test, RingInstance>;
		pub type Ring = darwinia_balances::Module<Test, RingInstance>;
		pub type KtonInstance = darwinia_balances::Instance1;
		pub type KtonError = darwinia_balances::Error<Test, KtonInstance>;
		pub type KtonConfig = darwinia_balances::GenesisConfig<Test, KtonInstance>;
		pub type Kton = darwinia_balances::Module<Test, KtonInstance>;

		$crate::impl_account_data! {
			struct AccountData<Balance>
			for
				RingInstance,
				KtonInstance
			where
				Balance = Balance
			{}
		}
	};
}
