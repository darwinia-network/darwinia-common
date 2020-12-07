#[macro_export]
macro_rules! impl_test_account_data {
	() => {
		pub type RingInstance = darwinia_balances::Instance0;
		pub type KtonInstance = darwinia_balances::Instance1;

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
