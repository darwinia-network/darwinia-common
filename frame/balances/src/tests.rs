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

//! Macro for creating the tests for the module.

#[macro_export]
macro_rules! decl_tests {
	($test:ty, $ext_builder:ty, $existential_deposit:expr) => {
		pub const CALL: &<$test as frame_system::Config>::Call =
			&Call::Ring(darwinia_balances::Call::transfer(0, 0));

		const ID_1: LockIdentifier = *b"1       ";
		const ID_2: LockIdentifier = *b"2       ";

		/// create a transaction info struct from weight. Handy to avoid building the whole struct.
		pub fn info_from_weight(w: Weight) -> DispatchInfo {
			DispatchInfo {
				weight: w,
				..Default::default()
			}
		}

		fn events() -> Vec<Event> {
			let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

			System::reset_events();

			evt
		}

		fn last_event() -> Event {
			<frame_system::Pallet<Test>>::events().pop().expect("Event expected").event
		}

		#[test]
		fn basic_locking_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					assert_eq!(Ring::free_balance(1), 10);
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 9 }, WithdrawReasons::all());
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 5, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
				});
		}

		#[test]
		fn account_should_be_reaped() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					assert_eq!(Ring::free_balance(1), 10);
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 10, ExistenceRequirement::AllowDeath));
					// Check that the account is dead.
					assert!(!<frame_system::Account<Test>>::contains_key(&1));
				});
		}

		#[test]
		fn partial_locking_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 5 }, WithdrawReasons::all());
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
				});
		}

		#[test]
		fn lock_removal_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common {
							amount: Balance::max_value(),
						},
						WithdrawReasons::all(),
					);
					Ring::remove_lock(ID_1, &1);
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
				});
		}

		#[test]
		fn lock_replacement_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common {
							amount: Balance::max_value(),
						},
						WithdrawReasons::all(),
					);
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 5 }, WithdrawReasons::all());
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
				});
		}

		#[test]
		fn double_locking_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 5 }, WithdrawReasons::all());
					Ring::set_lock(ID_2, &1, LockFor::Common { amount: 5 }, WithdrawReasons::all());
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
				});
		}

		#[test]
		fn combination_locking_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common {
							amount: Balance::max_value(),
						},
						WithdrawReasons::empty(),
					);
					Ring::set_lock(ID_2, &1, LockFor::Common { amount: 0 }, WithdrawReasons::all());
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
				});
		}

		#[test]
		fn lock_value_extension_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 5 }, WithdrawReasons::all());
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					assert_ok!(Ring::extend_lock(ID_1, &1, 2, WithdrawReasons::all()));
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					assert_ok!(Ring::extend_lock(ID_1, &1, 8, WithdrawReasons::all()));
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 3, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
				});
		}

		#[test]
		fn lock_reasons_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					pallet_transaction_payment::NextFeeMultiplier::put(Multiplier::saturating_from_integer(1));
					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common { amount: 10 },
						WithdrawReasons::RESERVE,
					);
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					assert_noop!(
						<Ring as ReservableCurrency<_>>::reserve(&1, 1),
						RingError::LiquidityRestrictions
					);
					assert!(
						<ChargeTransactionPayment<$test> as SignedExtension>::pre_dispatch(
							ChargeTransactionPayment::from(1),
							&1,
							CALL,
							&info_from_weight(1),
							1,
						)
						.is_err()
					);
					assert_ok!(
						<ChargeTransactionPayment<$test> as SignedExtension>::pre_dispatch(
							ChargeTransactionPayment::from(0),
							&1,
							CALL,
							&info_from_weight(1),
							1,
						)
					);

					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common { amount: 10 },
						WithdrawReasons::TRANSACTION_PAYMENT,
					);
					assert_ok!(<Ring as Currency<_>>::transfer(&1, &2, 1, ExistenceRequirement::AllowDeath));
					assert_ok!(<Ring as ReservableCurrency<_>>::reserve(&1, 1));
					assert!(
						<ChargeTransactionPayment<$test> as SignedExtension>::pre_dispatch(
							ChargeTransactionPayment::from(1),
							&1,
							CALL,
							&info_from_weight(1),
							1,
						)
						.is_err()
					);
					assert!(
						<ChargeTransactionPayment<$test> as SignedExtension>::pre_dispatch(
							ChargeTransactionPayment::from(0),
							&1,
							CALL,
							&info_from_weight(1),
							1,
						)
						.is_err()
					);
				});
		}

		#[test]
		fn lock_block_number_extension_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(ID_1, &1, LockFor::Common { amount: 10 }, WithdrawReasons::all());
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					Ring::extend_lock(ID_1, &1, 10, WithdrawReasons::all()).unwrap();
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					System::set_block_number(2);
					Ring::extend_lock(ID_1, &1, 10, WithdrawReasons::all()).unwrap();
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 3, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
				});
		}

		#[test]
		fn lock_reasons_extension_should_work() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.monied(true)
				.build()
				.execute_with(|| {
					Ring::set_lock(
						ID_1,
						&1,
						LockFor::Common { amount: 10 },
						WithdrawReasons::TRANSFER,
					);
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					Ring::extend_lock(ID_1, &1, 10, WithdrawReasons::empty()).unwrap();
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
					Ring::extend_lock(ID_1, &1, 10, WithdrawReasons::RESERVE).unwrap();
					assert_noop!(
						<Ring as Currency<_>>::transfer(&1, &2, 6, ExistenceRequirement::AllowDeath),
						RingError::LiquidityRestrictions
					);
				});
		}

		#[test]
		fn default_indexing_on_new_accounts_should_not_work2() {
			<$ext_builder>::default()
				.existential_deposit(10)
				.monied(true)
				.build()
				.execute_with(|| {
					// account 5 should not exist
					// ext_deposit is 10, value is 9, not satisfies for ext_deposit
					assert_noop!(
						Ring::transfer(Some(1).into(), 5, 9),
						RingError::ExistentialDeposit,
					);
					assert_eq!(Ring::free_balance(1), 100);
				});
		}

		#[test]
		fn reserved_balance_should_prevent_reclaim_count() {
			<$ext_builder>::default()
				.existential_deposit(256 * 1)
				.monied(true)
				.build()
				.execute_with(|| {
					System::inc_account_nonce(&2);
					assert_eq!(Ring::total_balance(&2), 256 * 20);

					assert_ok!(Ring::reserve(&2, 256 * 19 + 1)); // account 2 becomes mostly reserved
					assert_eq!(Ring::free_balance(2), 255); // "free" account deleted."
					assert_eq!(Ring::total_balance(&2), 256 * 20); // reserve still exists.
					assert_eq!(System::account_nonce(&2), 1);

					// account 4 tries to take index 1 for account 5.
					assert_ok!(Ring::transfer(Some(4).into(), 5, 256 * 1 + 0x69));
					assert_eq!(Ring::total_balance(&5), 256 * 1 + 0x69);

					assert!(Ring::slash(&2, 256 * 19 + 2).1.is_zero()); // account 2 gets slashed
																// "reserve" account reduced to 255 (below ED) so account deleted
					assert_eq!(Ring::total_balance(&2), 0);
					assert_eq!(System::account_nonce(&2), 0); // nonce zero

					// account 4 tries to take index 1 again for account 6.
					assert_ok!(Ring::transfer(Some(4).into(), 6, 256 * 1 + 0x69));
					assert_eq!(Ring::total_balance(&6), 256 * 1 + 0x69);
				});
		}

		#[test]
		fn reward_should_work() {
			<$ext_builder>::default().monied(true).build().execute_with(|| {
				assert_eq!(Ring::total_balance(&1), 10);
				assert_ok!(Ring::deposit_into_existing(&1, 10).map(drop));
				assert_eq!(Ring::total_balance(&1), 20);
				assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 120);
			});
		}

		#[test]
		fn dust_account_removal_should_work() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.monied(true)
				.build()
				.execute_with(|| {
					System::inc_account_nonce(&2);
					assert_eq!(System::account_nonce(&2), 1);
					assert_eq!(Ring::total_balance(&2), 2000);
					// index 1 (account 2) becomes zombie
					assert_ok!(Ring::transfer(Some(2).into(), 5, 1901));
					assert_eq!(Ring::total_balance(&2), 0);
					assert_eq!(Ring::total_balance(&5), 1901);
					assert_eq!(System::account_nonce(&2), 0);
				});
		}

		#[test]
		fn balance_works() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 42);
				assert_eq!(Ring::free_balance(1), 42);
				assert_eq!(Ring::reserved_balance(1), 0);
				assert_eq!(Ring::total_balance(&1), 42);
				assert_eq!(Ring::free_balance(2), 0);
				assert_eq!(Ring::reserved_balance(2), 0);
				assert_eq!(Ring::total_balance(&2), 0);
			});
		}

		#[test]
		fn balance_transfer_works() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::transfer(Some(1).into(), 2, 69));
				assert_eq!(Ring::total_balance(&1), 42);
				assert_eq!(Ring::total_balance(&2), 69);
			});
		}

		#[test]
		fn force_transfer_works() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_noop!(Ring::force_transfer(Some(2).into(), 1, 2, 69), BadOrigin,);
				assert_ok!(Ring::force_transfer(RawOrigin::Root.into(), 1, 2, 69));
				assert_eq!(Ring::total_balance(&1), 42);
				assert_eq!(Ring::total_balance(&2), 69);
			});
		}

		#[test]
		fn reserving_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);

				assert_eq!(Ring::total_balance(&1), 111);
				assert_eq!(Ring::free_balance(1), 111);
				assert_eq!(Ring::reserved_balance(1), 0);

				assert_ok!(Ring::reserve(&1, 69));

				assert_eq!(Ring::total_balance(&1), 111);
				assert_eq!(Ring::free_balance(1), 42);
				assert_eq!(Ring::reserved_balance(1), 69);
			});
		}

		#[test]
		fn balance_transfer_when_reserved_should_not_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 69));
				assert_noop!(
					Ring::transfer(Some(1).into(), 2, 69),
					RingError::InsufficientBalance,
				);
			});
		}

		#[test]
		fn deducting_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 69));
				assert_eq!(Ring::free_balance(1), 42);
			});
		}

		#[test]
		fn refunding_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 42);
				assert_ok!(Ring::mutate_account(&1, |a| a.reserved = 69));
				Ring::unreserve(&1, 69);
				assert_eq!(Ring::free_balance(1), 111);
				assert_eq!(Ring::reserved_balance(1), 0);
			});
		}

		#[test]
		fn slashing_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 69));
				assert!(Ring::slash(&1, 69).1.is_zero());
				assert_eq!(Ring::free_balance(1), 0);
				assert_eq!(Ring::reserved_balance(1), 42);
				assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 42);
			});
		}

		#[test]
		fn slashing_incomplete_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 42);
				assert_ok!(Ring::reserve(&1, 21));
				assert_eq!(Ring::slash(&1, 69).1, 27);
				assert_eq!(Ring::free_balance(1), 0);
				assert_eq!(Ring::reserved_balance(1), 0);
				assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 0);
			});
		}

		#[test]
		fn unreserving_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 111));
				Ring::unreserve(&1, 42);
				assert_eq!(Ring::reserved_balance(1), 69);
				assert_eq!(Ring::free_balance(1), 42);
			});
		}

		#[test]
		fn slashing_reserved_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 111));
				assert_eq!(Ring::slash_reserved(&1, 42).1, 0);
				assert_eq!(Ring::reserved_balance(1), 69);
				assert_eq!(Ring::free_balance(1), 0);
				assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 69);
			});
		}

		#[test]
		fn slashing_incomplete_reserved_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 42));
				assert_eq!(Ring::slash_reserved(&1, 69).1, 27);
				assert_eq!(Ring::free_balance(1), 69);
				assert_eq!(Ring::reserved_balance(1), 0);
				assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 69);
			});
		}

		#[test]
		fn repatriating_reserved_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 110);
				let _ = Ring::deposit_creating(&2, 1);
				assert_ok!(Ring::reserve(&1, 110));
				assert_ok!(Ring::repatriate_reserved(&1, &2, 41, BalanceStatus::Free), 0);
				assert_eq!(Ring::reserved_balance(1), 69);
				assert_eq!(Ring::free_balance(1), 0);
				assert_eq!(Ring::reserved_balance(2), 0);
				assert_eq!(Ring::free_balance(2), 42);
			});
		}

		#[test]
		fn transferring_reserved_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 110);
				let _ = Ring::deposit_creating(&2, 1);
				assert_ok!(Ring::reserve(&1, 110));
				assert_ok!(Ring::repatriate_reserved(&1, &2, 41, BalanceStatus::Reserved), 0);
				assert_eq!(Ring::reserved_balance(1), 69);
				assert_eq!(Ring::free_balance(1), 0);
				assert_eq!(Ring::reserved_balance(2), 41);
				assert_eq!(Ring::free_balance(2), 1);
			});
		}

		#[test]
		fn transferring_reserved_balance_to_nonexistent_should_fail() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 111);
				assert_ok!(Ring::reserve(&1, 111));
				assert_noop!(
					Ring::repatriate_reserved(&1, &2, 42, BalanceStatus::Free),
					RingError::DeadAccount
				);
			});
		}

		#[test]
		fn transferring_incomplete_reserved_balance_should_work() {
			<$ext_builder>::default().build().execute_with(|| {
				let _ = Ring::deposit_creating(&1, 110);
				let _ = Ring::deposit_creating(&2, 1);
				assert_ok!(Ring::reserve(&1, 41));
				assert_ok!(Ring::repatriate_reserved(&1, &2, 69, BalanceStatus::Free), 28);
				assert_eq!(Ring::reserved_balance(1), 0);
				assert_eq!(Ring::free_balance(1), 69);
				assert_eq!(Ring::reserved_balance(2), 0);
				assert_eq!(Ring::free_balance(2), 42);
			});
		}

		#[test]
		fn transferring_too_high_value_should_not_panic() {
			<$ext_builder>::default().build().execute_with(|| {
				Ring::make_free_balance_be(&1, Balance::max_value());
				Ring::make_free_balance_be(&2, 1);

				assert_err!(
					Ring::transfer(Some(1).into(), 2, Balance::max_value()),
					ArithmeticError::Overflow,
				);

				assert_eq!(Ring::free_balance(1), Balance::max_value());
				assert_eq!(Ring::free_balance(2), 1);
			});
		}

		#[test]
		fn account_create_on_free_too_low_with_other() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					let _ = Ring::deposit_creating(&1, 100);
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 100);

					// No-op.
					let _ = Ring::deposit_creating(&2, 50);
					assert_eq!(Ring::free_balance(2), 0);
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 100);
				})
		}

		#[test]
		fn account_create_on_free_too_low() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					// No-op.
					let _ = Ring::deposit_creating(&2, 50);
					assert_eq!(Ring::free_balance(2), 0);
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 0);
				})
		}

		#[test]
		fn account_removal_on_free_too_low() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 0);

					// Setup two accounts with free balance above the existential threshold.
					let _ = Ring::deposit_creating(&1, 110);
					let _ = Ring::deposit_creating(&2, 110);

					assert_eq!(Ring::free_balance(1), 110);
					assert_eq!(Ring::free_balance(2), 110);
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 220);

					// Transfer funds from account 1 of such amount that after this transfer
					// the balance of account 1 will be below the existential threshold.
					// This should lead to the removal of all balance of this account.
					assert_ok!(Ring::transfer(Some(1).into(), 2, 20));

					// Verify free balance removal of account 1.
					assert_eq!(Ring::free_balance(1), 0);
					assert_eq!(Ring::free_balance(2), 130);

					// Verify that TotalIssuance tracks balance removal when free balance is too low.
					assert_eq!(<TotalIssuance<$test, RingInstance>>::get(), 130);
				});
		}

		#[test]
		fn burn_must_work() {
			<$ext_builder>::default().monied(true).build().execute_with(|| {
				let init_total_issuance = Ring::total_issuance();
				let imbalance = Ring::burn(10);
				assert_eq!(Ring::total_issuance(), init_total_issuance - 10);
				drop(imbalance);
				assert_eq!(Ring::total_issuance(), init_total_issuance);
			});
		}

		#[test]
		fn transfer_keep_alive_works() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.build()
				.execute_with(|| {
					let _ = Ring::deposit_creating(&1, 100);
					assert_noop!(
						Ring::transfer_keep_alive(Some(1).into(), 2, 100),
						RingError::KeepAlive
					);
					assert_eq!(Ring::total_balance(&1), 100);
					assert_eq!(Ring::total_balance(&2), 0);
				});
		}

		#[test]
		#[should_panic = "the balance of any account should always be at least the existential deposit."]
		fn cannot_set_genesis_value_below_ed() {
			($existential_deposit).with(|v| *v.borrow_mut() = 11);
			let mut t = frame_system::GenesisConfig::default()
				.build_storage::<$test>()
				.unwrap();
			let _ = darwinia_balances::GenesisConfig::<$test, RingInstance> {
				balances: vec![(1, 10)],
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		#[test]
		#[should_panic = "duplicate balances in genesis."]
		fn cannot_set_genesis_value_twice() {
			let mut t = frame_system::GenesisConfig::default().build_storage::<$test>().unwrap();
			let _ = darwinia_balances::GenesisConfig::<$test, RingInstance> {
				balances: vec![(1, 10), (2, 20), (1, 15)],
			}.assimilate_storage(&mut t).unwrap();
		}

		#[test]
		fn dust_moves_between_free_and_reserved() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					// Set balance to free and reserved at the existential deposit
					assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));
					// Check balance
					assert_eq!(Ring::free_balance(1), 100);
					assert_eq!(Ring::reserved_balance(1), 0);

					// Reserve some free balance
					assert_ok!(Ring::reserve(&1, 50));
					// Check balance, the account should be ok.
					assert_eq!(Ring::free_balance(1), 50);
					assert_eq!(Ring::reserved_balance(1), 50);

					// Reserve the rest of the free balance
					assert_ok!(Ring::reserve(&1, 50));
					// Check balance, the account should be ok.
					assert_eq!(Ring::free_balance(1), 0);
					assert_eq!(Ring::reserved_balance(1), 100);

					// Unreserve everything
					Ring::unreserve(&1, 100);
					// Check balance, all 100 should move to free_balance
					assert_eq!(Ring::free_balance(1), 100);
					assert_eq!(Ring::reserved_balance(1), 0);
				});
		}

		#[test]
		fn account_deleted_when_just_dust() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					// Set balance to free and reserved at the existential deposit
					assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 50, 50));
					// Check balance
					assert_eq!(Ring::free_balance(1), 50);
					assert_eq!(Ring::reserved_balance(1), 50);

					// Reserve some free balance
					let res = Ring::slash(&1, 1);
					assert_eq!(res, (NegativeImbalance::new(1), 0));
					// The account should be dead.
					assert_eq!(Ring::free_balance(1), 0);
					assert_eq!(Ring::reserved_balance(1), 0);
				});
		}

		#[test]
		fn emit_events_with_reserve_and_unreserve() {
			<$ext_builder>::default()
				.build()
				.execute_with(|| {
					let _ = Ring::deposit_creating(&1, 100);

					System::set_block_number(2);
					assert_ok!(Ring::reserve(&1, 10));

					assert_eq!(
						last_event(),
						Event::darwinia_balances_Instance1(darwinia_balances::Event::Reserved(1, 10)),
					);

					System::set_block_number(3);
					assert!(Ring::unreserve(&1, 5).is_zero());

					assert_eq!(
						last_event(),
						Event::darwinia_balances_Instance1(darwinia_balances::Event::Unreserved(1, 5)),
					);

					System::set_block_number(4);
					assert_eq!(Ring::unreserve(&1, 6), 1);

					// should only unreserve 5
					assert_eq!(
						last_event(),
						Event::darwinia_balances_Instance1(darwinia_balances::Event::Unreserved(1, 5)),
					);
				});
		}

		#[test]
		fn emit_events_with_existential_deposit() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

					assert_eq!(
						events(),
						[
							Event::frame_system(frame_system::Event::NewAccount(1)),
							Event::darwinia_balances_Instance1(darwinia_balances::Event::Endowed(1, 100)),
							Event::darwinia_balances_Instance1(darwinia_balances::Event::BalanceSet(1, 100, 0)),
						]
					);

					let res = Ring::slash(&1, 1);
					assert_eq!(res, (NegativeImbalance::new(1), 0));

					assert_eq!(
						events(),
						[
							Event::frame_system(frame_system::Event::KilledAccount(1)),
							Event::darwinia_balances_Instance1(darwinia_balances::Event::DustLost(1, 99))
						]
					);
				});
		}

		#[test]
		fn emit_events_with_no_existential_deposit_suicide() {
			<$ext_builder>::default()
				.existential_deposit(1)
				.build()
				.execute_with(|| {
					assert_ok!(Ring::set_balance(RawOrigin::Root.into(), 1, 100, 0));

					assert_eq!(
						events(),
						[
							Event::frame_system(frame_system::Event::NewAccount(1)),
							Event::darwinia_balances_Instance1(darwinia_balances::Event::Endowed(1, 100)),
							Event::darwinia_balances_Instance1(darwinia_balances::Event::BalanceSet(1, 100, 0)),
						]
					);

					let res = Ring::slash(&1, 100);
					assert_eq!(res, (NegativeImbalance::new(100), 0));

					assert_eq!(
						events(),
						[
							Event::frame_system(frame_system::Event::KilledAccount(1))
						]
					);
				});
		}

		#[test]
		fn slash_loop_works() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					/* User has no reference counter, so they can die in these scenarios */

					// SCENARIO: Slash would not kill account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					// Slashed completed in full
					assert_eq!(Ring::slash(&1, 900), (NegativeImbalance::new(900), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Slash will kill account because not enough balance left.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					// Slashed completed in full
					assert_eq!(Ring::slash(&1, 950), (NegativeImbalance::new(950), 0));
					// Account is killed
					assert!(!System::account_exists(&1));

					// SCENARIO: Over-slash will kill account, and report missing slash amount.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					// Slashed full free_balance, and reports 300 not slashed
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1000), 300));
					// Account is dead
					assert!(!System::account_exists(&1));

					// SCENARIO: Over-slash can take from reserved, but keep alive.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 400));
					// Slashed full free_balance and 300 of reserved balance
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1300), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash can take from reserved, and kill.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 350));
					// Slashed full free_balance and 300 of reserved balance
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1300), 0));
					// Account is dead because 50 reserved balance is not enough to keep alive
					assert!(!System::account_exists(&1));

					// SCENARIO: Over-slash can take as much as possible from reserved, kill, and report missing amount.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 250));
					// Slashed full free_balance and 300 of reserved balance
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1250), 50));
					// Account is super dead
					assert!(!System::account_exists(&1));

					/* User will now have a reference counter on them, keeping them alive in these scenarios */

					// SCENARIO: Slash would not kill account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					assert_ok!(System::inc_consumers(&1)); // <-- Reference counter added here is enough for all tests
					// Slashed completed in full
					assert_eq!(Ring::slash(&1, 900), (NegativeImbalance::new(900), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Slash will take as much as possible without killing account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					// Slashed completed in full
					assert_eq!(Ring::slash(&1, 950), (NegativeImbalance::new(900), 50));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash will not kill account, and report missing slash amount.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 0));
					// Slashed full free_balance minus ED, and reports 400 not slashed
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(900), 400));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash can take from reserved, but keep alive.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 400));
					// Slashed full free_balance and 300 of reserved balance
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1300), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash can take from reserved, but keep alive.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 350));
					// Slashed full free_balance and 250 of reserved balance to leave ED
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1250), 50));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash can take as much as possible from reserved and report missing amount.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 1_000, 250));
					// Slashed full free_balance and 300 of reserved balance
					assert_eq!(Ring::slash(&1, 1_300), (NegativeImbalance::new(1150), 150));
					// Account is still alive
					assert!(System::account_exists(&1));

					// Slash on non-existent account is okay.
					assert_eq!(Ring::slash(&12345, 1_300), (NegativeImbalance::new(0), 1300));
				});
		}

		#[test]
		fn slash_reserved_loop_works() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					/* User has no reference counter, so they can die in these scenarios */

					// SCENARIO: Slash would not kill account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					// Slashed completed in full
					assert_eq!(Ring::slash_reserved(&1, 900), (NegativeImbalance::new(900), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Slash would kill account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					// Slashed completed in full
					assert_eq!(Ring::slash_reserved(&1, 1_000), (NegativeImbalance::new(1_000), 0));
					// Account is dead
					assert!(!System::account_exists(&1));

					// SCENARIO: Over-slash would kill account, and reports left over slash.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					// Slashed completed in full
					assert_eq!(Ring::slash_reserved(&1, 1_300), (NegativeImbalance::new(1_000), 300));
					// Account is dead
					assert!(!System::account_exists(&1));

					// SCENARIO: Over-slash does not take from free balance.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 300, 1_000));
					// Slashed completed in full
					assert_eq!(Ring::slash_reserved(&1, 1_300), (NegativeImbalance::new(1_000), 300));
					// Account is alive because of free balance
					assert!(System::account_exists(&1));

					/* User has a reference counter, so they cannot die */

					// SCENARIO: Slash would not kill account.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					assert_ok!(System::inc_consumers(&1)); // <-- Reference counter added here is enough for all tests
					// Slashed completed in full
					assert_eq!(Ring::slash_reserved(&1, 900), (NegativeImbalance::new(900), 0));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Slash as much as possible without killing.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					// Slashed as much as possible
					assert_eq!(Ring::slash_reserved(&1, 1_000), (NegativeImbalance::new(950), 50));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash reports correctly, where reserved is needed to keep alive.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 50, 1_000));
					// Slashed as much as possible
					assert_eq!(Ring::slash_reserved(&1, 1_300), (NegativeImbalance::new(950), 350));
					// Account is still alive
					assert!(System::account_exists(&1));

					// SCENARIO: Over-slash reports correctly, where full reserved is removed.
					assert_ok!(Ring::set_balance(Origin::root(), 1, 200, 1_000));
					// Slashed as much as possible
					assert_eq!(Ring::slash_reserved(&1, 1_300), (NegativeImbalance::new(1_000), 300));
					// Account is still alive
					assert!(System::account_exists(&1));

					// Slash on non-existent account is okay.
					assert_eq!(Ring::slash_reserved(&12345, 1_300), (NegativeImbalance::new(0), 1300));
				});
		}

		#[test]
		fn operations_on_dead_account_should_not_change_state() {
			// These functions all use `mutate_account` which may introduce a storage change when
			// the account never existed to begin with, and shouldn't exist in the end.
			<$ext_builder>::default()
				.existential_deposit(0)
				.build()
				.execute_with(|| {
					assert!(!<frame_system::Account<Test>>::contains_key(&1337));

					// Unreserve
					assert_storage_noop!(assert_eq!(Ring::unreserve(&1337, 42), 42));
					// Reserve
					assert_noop!(Ring::reserve(&1337, 42), RingError::InsufficientBalance);
					// Slash Reserve
					assert_storage_noop!(assert_eq!(Ring::slash_reserved(&1337, 42).1, 42));
					// Repatriate Reserve
					assert_noop!(Ring::repatriate_reserved(&1337, &1338, 42, BalanceStatus::Free), RingError::DeadAccount);
					// Slash
					assert_storage_noop!(assert_eq!(Ring::slash(&1337, 42).1, 42));
				});
		}

		#[test]
		fn transfer_keep_alive_all_free_succeed() {
			<$ext_builder>::default()
				.existential_deposit(100)
				.build()
				.execute_with(|| {
					assert_ok!(Ring::set_balance(Origin::root(), 1, 100, 100));
					assert_ok!(Ring::transfer_keep_alive(Some(1).into(), 2, 100));
					assert_eq!(Ring::total_balance(&1), 100);
					assert_eq!(Ring::total_balance(&2), 100);
				});
		}
	};
}
