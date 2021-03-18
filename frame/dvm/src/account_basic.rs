use darwinia_evm::{Account as EVMAccount, AccountBasicMapping, AddressMapping};
use frame_support::traits::Currency;
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{UniqueSaturatedFrom, UniqueSaturatedInto},
	SaturatedConversion,
};

type RingInstance = darwinia_balances::Instance0;

pub struct DVMAccountBasicMapping<T>(sp_std::marker::PhantomData<T>);

impl<T: crate::Config + darwinia_balances::Config<RingInstance>> AccountBasicMapping
	for DVMAccountBasicMapping<T>
{
	/// Get the account basic in EVM format.
	fn account_basic(address: &H160) -> EVMAccount {
		let account_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(*address);
		let nonce = frame_system::Module::<T>::account_nonce(&account_id);
		let helper = U256::from(10)
			.checked_pow(U256::from(9))
			.unwrap_or(U256::from(0));

		// Get balance from <T as darwinia_evm::Config>::RingCurrency
		let balance: U256 = <T as darwinia_evm::Config>::RingCurrency::free_balance(&account_id)
			.saturated_into::<u128>()
			.into();

		// Get remaining balance from dvm
		let remaining_balance: U256 = crate::Module::<T>::remaining_balance(&account_id)
			.saturated_into::<u128>()
			.into();

		// Final balance = balance * 10^9 + remaining_balance
		let final_balance = U256::from(balance * helper)
			.checked_add(remaining_balance)
			.unwrap_or_default();

		EVMAccount {
			nonce: nonce.saturated_into::<u128>().into(),
			balance: final_balance,
		}
	}

	/// Mutate the basic account
	fn mutate_account_basic(address: &H160, new: EVMAccount) {
		let helper = U256::from(10)
			.checked_pow(U256::from(9))
			.unwrap_or(U256::MAX);
		let existential_deposit: u128 =
			<T as darwinia_evm::Config>::RingCurrency::minimum_balance()
				.saturated_into::<u128>()
				.into();
		let existential_deposit_dvm = U256::from(existential_deposit) * helper;

		let account_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(*address);
		let current = T::AccountBasicMapping::account_basic(address);
		let dvm_balance: U256 = crate::Module::<T>::remaining_balance(&account_id)
			.saturated_into::<u128>()
			.into();

		if current.nonce < new.nonce {
			// ASSUME: in one single EVM transaction, the nonce will not increase more than
			// `u128::max_value()`.
			for _ in 0..(new.nonce - current.nonce).low_u128() {
				frame_system::Module::<T>::inc_account_nonce(&account_id);
			}
		}

		let nb = new.balance;
		match current.balance {
			cb if cb > nb => {
				let diff = cb - nb;
				let (diff_balance, diff_remaining_balance) = diff.div_mod(helper);
				// If the dvm storage < diff remaining balance, we can not do sub operation directly.
				// Otherwise, slash <T as darwinia_evm::Config>::RingCurrency, dec dvm storage balance directly.
				if dvm_balance < diff_remaining_balance {
					let remaining_balance = dvm_balance
						.saturating_add(U256::from(1) * helper)
						.saturating_sub(diff_remaining_balance);

					<T as darwinia_evm::Config>::RingCurrency::slash(
						&account_id,
						(diff_balance + 1).low_u128().unique_saturated_into(),
					);
					let value = <T as darwinia_balances::Config<RingInstance>>::Balance::unique_saturated_from(
						remaining_balance.low_u128(),
					);
					crate::Module::<T>::set_remaining_balance(&account_id, value);
				} else {
					<T as darwinia_evm::Config>::RingCurrency::slash(
						&account_id,
						diff_balance.low_u128().unique_saturated_into(),
					);
					let value = <T as darwinia_balances::Config<RingInstance>>::Balance::unique_saturated_from(
					diff_remaining_balance.low_u128());
					crate::Module::<T>::dec_remaining_balance(&account_id, value);
				}
			}
			cb if cb < nb => {
				let diff = nb - cb;
				let (diff_balance, diff_remaining_balance) = diff.div_mod(helper);

				// If dvm storage balance + diff remaining balance > helper, we must update <T as darwinia_evm::Config>::RingCurrency balance.
				if dvm_balance + diff_remaining_balance >= helper {
					let remaining_balance = dvm_balance + diff_remaining_balance - helper;

					<T as darwinia_evm::Config>::RingCurrency::deposit_creating(
						&account_id,
						(diff_balance + 1).low_u128().unique_saturated_into(),
					);
					let value = <T as darwinia_balances::Config<RingInstance>>::Balance::unique_saturated_from(
						remaining_balance.low_u128(),
					);
					crate::Module::<T>::set_remaining_balance(&account_id, value);
				} else {
					<T as darwinia_evm::Config>::RingCurrency::deposit_creating(
						&account_id,
						diff_balance.low_u128().unique_saturated_into(),
					);
					let value = <T as darwinia_balances::Config<RingInstance>>::Balance::unique_saturated_from(
						diff_remaining_balance.low_u128(),
					);
					crate::Module::<T>::inc_remaining_balance(&account_id, value);
				}
			}
			_ => return,
		}
		let after_mutate = T::AccountBasicMapping::account_basic(address);
		if after_mutate.balance < existential_deposit_dvm {
			crate::Module::<T>::remove_remaining_balance(&account_id);
		}
	}
}
