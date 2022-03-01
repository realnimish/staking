#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod staking {
    use ink_storage::{
        Mapping,
        traits::{
            SpreadAllocate,
            PackedLayout,
            SpreadLayout,
        },
    };

    type Time = u64;

    #[derive(PackedLayout, SpreadLayout)]
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Lock {
        locked_amt: Balance,
        locked_on: Time,
        last_claimed: Option<Time>,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Staking {
        stakes: Mapping<AccountId, Lock>,
    }

    impl Staking {
        
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|_| {})
        }

        fn claimable_value(lock: &Lock, time: Option<Time>) -> Balance {
            if time.is_none() {
                return 0;
            }
            let time = time.unwrap();
            let mut period = lock.locked_on;
            let mut tokens = lock.locked_amt;
            let mut claim = tokens/2;
            let daily_unlock = tokens/10;
            const DAY: Time = 60*60*24;

            period += DAY;
            while time >= period && tokens > 0 {
                tokens -= daily_unlock;
                claim += daily_unlock;
                if tokens < daily_unlock {
                    claim += tokens;
                    tokens = 0;
                }
                period += DAY;
            }
            return claim;
        }

        #[ink(message)]
        pub fn get_lock_details(&self) -> Option<Lock> {
            let caller = self.env().caller();
            self.stakes.get(&caller)
        }

        #[ink(message)]
        pub fn get_locked_amt(&self) -> Balance {
            let lock = self.get_lock_details();
            let now = self.env().block_timestamp();
            match lock {
                None => 0,
                Some(lock) => lock.locked_amt - Self::claimable_value(&lock, Some(now))
            }
        }

        #[ink(message)]
        pub fn get_claimed_amt(&self) -> Balance {
            let lock = self.get_lock_details();
            match lock {
                None => 0,
                Some(lock) => Self::claimable_value(&lock, lock.last_claimed)
            }
        }

        #[ink(message)]
        pub fn get_pending_amt(&self) -> Balance {
            let lock = self.get_lock_details();
            let now = self.env().block_timestamp();
            match lock {
                None => 0,
                Some(lock) => Self::claimable_value(&lock, Some(now)) - Self::claimable_value(&lock, lock.last_claimed)
            }
        }

        #[ink(message)]
        pub fn insert(&mut self, locked_amt: Balance, locked_on: Time) {
            let caller = self.env().caller();
            let lock = Lock{locked_amt, locked_on, last_claimed:None};
            self.stakes.insert(&caller,&lock);
        }

        #[ink(message,payable)]
        pub fn lock_tokens(&mut self) {
            let locked_amt = self.env().transferred_value();

            assert!(locked_amt > 0, "Zero tokens sent");
            assert!(self.get_locked_amt() == 0, "Your previous lock period has not ended");
            assert!(self.get_pending_amt() == 0, "Claim your tokens from previous lock first");

            let caller = self.env().caller();
            let locked_on = self.env().block_timestamp();
            let last_claimed: Option<Time> = None;
            let lock = Lock{locked_amt,locked_on,last_claimed};
            self.stakes.insert(&caller,&lock);
        }

        #[ink(message)]
        pub fn claim_tokens(&mut self) {
            let value = self.get_pending_amt();
            if value == 0 {
                return;
            }
            if let Some(mut lock) = self.get_lock_details() {
                lock.last_claimed = Some(self.env().block_timestamp());
            }

            let caller = self.env().caller();
            self.env().transfer(caller,value).unwrap();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn new_works() {
            let staking = Staking::new();
            assert_eq!(staking.get_lock_details(), None);
        }

        #[ink::test]
        fn insert_works() {
            let mut staking = Staking::new();
            assert_eq!(staking.get_lock_details(), None);
            staking.insert(5,10);
            let output = Lock{locked_amt: 5,locked_on: 10, last_claimed: None};
            assert_eq!(staking.get_lock_details(), Some(output));
        }
    }
}
