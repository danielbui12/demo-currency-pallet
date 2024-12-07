//! A shell pallet built with [`frame`].
//!
//! To get started with this pallet, try implementing the guide in
//! <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>

#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::polkadot_sdk_frame as frame;

// Re-export all pallet parts, this is needed to properly import the pallet into the runtime.
pub use pallet::*;

#[frame::pallet(dev_mode)]
pub mod pallet {
    use crate::frame::prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
    pub type Balance = u128;

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

        fn min_amount() -> Balance;
    }

  	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        Mint { to: T::AccountId, amount: Balance },
        Transfer { from: T::AccountId, to: T::AccountId, amount: Balance },
    }

    #[pallet::error]
    pub enum Error<T> { 
        InsufficientBalance,
        NonExistentAccount,
        BelowMinAmount
    }

    #[pallet::storage]
    pub type TotalIssuance<T: Config> = StorageValue<_, Balance>;
    #[pallet::storage]
    pub type BalanceOf<T: Config> = StorageMap<Key = T::AccountId, Value = Balance>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// An unsafe mint that can be called by anyone. Not a great idea.
        pub fn mint_unsafe(
            origin: T::RuntimeOrigin,
            dest: T::AccountId,
            amount: Balance,
        ) -> DispatchResult {
            // ensure that this is a signed account, but we don't really check `_who`.
            let _who = ensure_signed(origin)?;

            ensure!(amount >= T::min_amount(), Error::<T>::BelowMinAmount);

            // update the `BalanceOf` map. Notice how all `<T: Config>` remains as `<T>`.
            BalanceOf::<T>::mutate(dest.clone(), |b| *b = Some(b.unwrap_or(0) + amount));
            // update total issuance.
            TotalIssuance::<T>::mutate(|t| *t = Some(t.unwrap_or(0) + amount));
        
			Self::deposit_event(Event::Mint { to: dest, amount: amount });

            Ok(())
        }

        /// Transfer `amount` from `origin` to `dest`.
        pub fn transfer(
            origin: T::RuntimeOrigin,
            dest: T::AccountId,
            amount: Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // ensure sender has enough balance, and if so, calculate what is left after `amount`.
            let sender_balance = BalanceOf::<T>::get(&sender).ok_or(Error::<T>::NonExistentAccount)?;
           	let remainder = sender_balance.checked_sub(amount).ok_or(Error::<T>::InsufficientBalance)?;

            // update sender and dest `BalanceOf`.
            BalanceOf::<T>::mutate(dest.clone(), |b| *b = Some(b.unwrap_or(0) + amount));
            BalanceOf::<T>::insert(&sender, remainder);

            Self::deposit_event(Event::Transfer { from: sender, to: dest, amount: amount });

            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    // bring in the testing prelude of frame
    use crate::frame::testing_prelude::*;
	// bring in all pallet items
	use super::pallet as pallet_currency;

    construct_runtime!(
        pub struct Runtime {
            System: frame_system,
            Currency: pallet_currency,
        }
    );


	#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
	impl frame_system::Config for Runtime {
		type Block = MockBlock<Runtime>;
		// within pallet we just said `<T as frame_system::Config>::AccountId`, now we
		// finally specified it.
		type AccountId = u64;
	}

    impl pallet_currency::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;

        fn min_amount() -> pallet_currency::Balance {
            1
        }
    }

    #[test]
    fn should_mint_unsafe() {
        TestState::new_empty().execute_with(|| {
            System::set_block_number(1);

            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(1), None);
            assert_eq!(pallet_currency::TotalIssuance::<Runtime>::get(), None);

            const DEST: <Runtime as frame_system::Config>::AccountId = 1;
            const AMOUNT: pallet_currency::Balance = 100;
            
            assert_ok!(pallet_currency::Pallet::<Runtime>::mint_unsafe(
                RuntimeOrigin::signed(1),
                DEST,
                AMOUNT
            ));

            // re-check the above
            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(1), Some(100));
            assert_eq!(pallet_currency::TotalIssuance::<Runtime>::get(), Some(100));

            let events = System::events();
            assert_eq!(events.len(), 1);
            System::assert_has_event(pallet_currency::Event::Mint { to: DEST, amount: AMOUNT }.into());
        });
    }

    #[test]
    fn should_not_mint_unsafe_below_min_amount()  {
        TestState::new_empty().execute_with(|| {
            assert_noop!(
                pallet_currency::Pallet::<Runtime>::mint_unsafe(RuntimeOrigin::signed(1), 2, 0),
                pallet_currency::Error::<Runtime>::BelowMinAmount
            );
        });
    }


    #[test]
    fn should_transfer()  {
        TestState::new_empty().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(pallet_currency::Pallet::<Runtime>::mint_unsafe(
                RuntimeOrigin::signed(1),
                1,
                100
            ));

            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(1), Some(100));
            assert_eq!(pallet_currency::TotalIssuance::<Runtime>::get(), Some(100));

            const FROM: <Runtime as frame_system::Config>::AccountId  = 1;
            const TO: <Runtime as frame_system::Config>::AccountId  = 2;
            const AMOUNT: pallet_currency::Balance = 50;

            assert_ok!(pallet_currency::Pallet::<Runtime>::transfer(
                RuntimeOrigin::signed(FROM),
                TO,
                AMOUNT
            ));
            System::assert_has_event(pallet_currency::Event::Transfer { from: FROM, to: TO, amount: AMOUNT }.into());

            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(2), Some(50));

        });
    }

    #[test]
    fn should_not_transfer_insufficient_balance()  {
        TestState::new_empty().execute_with(|| {
            assert_ok!(pallet_currency::Pallet::<Runtime>::mint_unsafe(
                RuntimeOrigin::signed(1),
                1,
                100
            ));

            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(1), Some(100));
            assert_eq!(pallet_currency::TotalIssuance::<Runtime>::get(), Some(100));

            assert_noop!(
                pallet_currency::Pallet::<Runtime>::transfer(RuntimeOrigin::signed(1), 2, 101),
                pallet_currency::Error::<Runtime>::InsufficientBalance
            );
        });
    }

    #[test]
    fn should_not_transfer_non_existent_account()  {
        TestState::new_empty().execute_with(|| {
            assert_eq!(pallet_currency::BalanceOf::<Runtime>::get(6), None);

            assert_noop!(
                pallet_currency::Pallet::<Runtime>::transfer(RuntimeOrigin::signed(6), 2, 101),
                pallet_currency::Error::<Runtime>::NonExistentAccount
            );
        });
    }
}