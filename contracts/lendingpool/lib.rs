#![cfg_attr(not(feature = "std"), no_std)]

mod types;
use ink_lang as ink;

#[ink::contract]
mod lendingpool {
    use crate::types::*;
    use ierc20::IERC20;
    use ink_prelude::string::String;
    use ink_env::call::FromAccountId;
    use ink_prelude::{vec, vec::Vec};
    use ink_storage::collections::HashMap as StorageHashMap;

    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(event)]
    pub struct Withdraw {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
    
    #[ink(event)]
    pub struct Borrow {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
   
    #[ink(event)]
    pub struct Repay {
        #[ink(topic)]
        receiver: AccountId,
        #[ink(topic)]
        repayer: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
    
    #[ink(event)]
    pub struct Delegate {
        #[ink(topic)]
        delegator: AccountId,
        #[ink(topic)]
        delegatee: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(storage)]
    pub struct Lendingpool {
        reserve: ReserveData,
        users_data: StorageHashMap<AccountId, UserReserveData>,
        delegate_allowance: StorageHashMap<(AccountId, AccountId), Balance>,
        users_kyc_data: StorageHashMap<AccountId, UserKycData>,
        interest_setting: InterestRateData,
        max_borrow_size_percent:u128 //根据stable的，可要可不要！
    }

    impl Lendingpool {  
        #[ink(constructor)]
        pub fn new(
            stoken: AccountId, debt_token: AccountId, 
            ltv: u128, liquidity_threshold: u128, 
            liquidity_bonus: u128, reserve_factor: u128,
            optimal_utilization_rate:u128, base_borrow_rate: u128, 
            rate_slope1: u128, rate_slope2:u128,
        ) -> Self {
            Self {//洋
                reserve: ReserveData {
                    liquidity_rate:10,
                    borrow_rate: 18,
                    stoken_address: stoken,
                    debt_token_address: debt_token,
                    ltv: ltv,
                    liquidity_threshold: liquidity_threshold,//是随时变的？
                    liquidity_bonus: liquidity_bonus,
                    decimals: 12,
                    reserve_factor: reserve_factor,
                    liquidity_index:1, //这个需加12位0？
                    borrow_index:1,//这个需加12位0？
                    last_updated_timestamp:Default::default(),
                },
                users_data: StorageHashMap::new(),
                delegate_allowance: StorageHashMap::new(),
                users_kyc_data: StorageHashMap::new(),
                interest_setting: InterestRateData {
                    optimal_utilization_rate:optimal_utilization_rate,
                    excess_utilization_rate:1 - optimal_utilization_rate,
                    base_borrow_rate: base_borrow_rate,
                    rate_slope1: rate_slope1,
                    rate_slope2: rate_slope2,
                    utilization_rate: Default::default(),
                },
                max_borrow_size_percent:2500,//利率借款时的额度不能超过资产剩余可借额度的25%
            }
        }
       
        #[ink(message, payable)]
        pub fn deposit(&mut self, on_behalf_of: Option<AccountId>) {
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = on_behalf_of {
                receiver = behalf;
            }
            let amount = self.env().transferred_balance();
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);

            let entry = self.users_data.entry(receiver);
            let reserve_data = entry.or_insert(Default::default());
            // user balance should always be stoken - debttoken
            let user_balance = stoken.balance_of(receiver) - debttoken.balance_of(receiver);

            if reserve_data.last_update_timestamp != 0 {
                let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;
                let interest = user_balance * interval as u128 * self.reserve.liquidity_rate
                    / (100 * 365 * 24 * 3600 * 1000);
                if interest > 0 {
                    reserve_data.cumulated_liquidity_interest += interest;
                    reserve_data.last_update_timestamp = Self::env().block_timestamp();
                }
            } else {
                reserve_data.last_update_timestamp = Self::env().block_timestamp();
            }

            assert!(stoken.mint(receiver, amount).is_ok());
            //把user加到userconfig上: u:UserConfig
            // let user_data = UserData{
            //     principal_borrow_balance: ,
            //     last_variable_borrow_cumulative_index: , 
            //     origination_fee: ,
            //     borrow_rate: ,
            //     last_update_timestamp: ,
            // }
            // u.user_config.insert(sender, user_data)

            self.env().emit_event(Deposit {
                user: sender,
                on_behalf_of: receiver,
                amount,
            });
        }

        // #[ink(message)]
        // pub fn get_user_reserve_data(&self, user: AccountId) -> Option<UserReserveData> {
        //     self.users_data.get(&user).cloned()
        // }

        // #[ink(message)]
        // pub fn get_base_borrow_rate(&self) -> u128 {
        //     self.base_borrow_rate
        // }
    
        // #[ink(message)]
        // pub fn get_rate_slope1(&self) -> u128 {
        //     self.rate_slope1
        // }

        // #[ink(message)]
        // pub fn get_rate_slope2(&self) -> u128 {
        //     self.rate_slope2
        // }

        // #[ink(message)]
        // pub fn get_max_borrow_rate(&self) -> u128 {
        //     self.base_borrow_rate + self.rate_slope1 + self.rate_slope2
        // }
        
        pub fn get_normalized_income(&self, vars: ReserveData) -> u128 {
            let timestamp = vars.last_updated_timestamp; 
            if timestamp == self.env().block_timestamp() {
                return vars.liquidity_index
            }
            let cumulated = self.caculate_linear_interest(&vars, timestamp) * &vars.liquidity_index;
            cumulated
        }
        //david
        fn caculate_linear_interest(&self, vars: &ReserveData, last_updated_timestamp: u64) -> u128 {
            let time_difference = self.env().block_timestamp() - last_updated_timestamp;
            //let interest = vars.liquidity_rate * time_difference.into() / ONE_YEAR + 1; //need to be replaced by one
            let interest =0;
            interest
        }
        //david
        fn calculate_compounded_interest(&self, rate:u128, last_update_timestamp:u64) -> u128{
            let time_difference = self.env().block_timestamp() - last_update_timestamp;
            if time_difference == 0 {
                return 1
            } 
            let time_difference_minus_one = time_difference - 1;
            let time_difference_minus_two = if time_difference > 2{
                time_difference - 2
            } else {
                0
            };
            let rate_per_second = rate / ONE_YEAR;
            let base_power_two = rate_per_second * rate_per_second;
            let base_power_three = base_power_two * rate_per_second;
            //let second_term = time_difference * time_difference_minus_one * base_power_two / 2;
            // let third_term = time_difference * time_difference_minus_one * time_difference_minus_two * base_power_three / 6;
            // let interest = rate_per_second * time_difference + 1 + second_term + third_term;
            let interest:u128 = 0;
            interest
        }
        //ting
        fn update_indexes(&self, vars: &mut ReserveData,scaled_debt:u128, timestamp:u64, liquidity_index:u128, borrow_index:u128) -> (u128, u128){
            let current_liquidity_rate = vars.liquidity_rate;
            let mut new_liquidity_index = liquidity_index;        
            let mut new_borrow_index = borrow_index;
            if current_liquidity_rate > 0 {
                let cumulated_liquidity_interest = self.caculate_linear_interest(&vars, timestamp);
                new_liquidity_index *= cumulated_liquidity_interest;//这个算法不对？应该是+
                //todo new_liquidity_index overflow
                vars.liquidity_index = new_liquidity_index;
                //要确认可用，因为是被variable_debt用的！
                if scaled_debt != 0{
                    let cumulated_borrow_interest = self.calculate_compounded_interest(vars.borrow_rate, timestamp);
                    new_borrow_index = cumulated_borrow_interest * borrow_index;
                    //todo new_borrow_index overflow
                    vars.borrow_index = new_borrow_index; 
                }                         
            }
            vars.last_updated_timestamp = self.env().block_timestamp();
            (new_liquidity_index, new_borrow_index)
        }
        //ting
        fn update_state(&self, vars:&mut ReserveData){
            let debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            let total_debt = debttoken.total_supply();
            let previous_borrow_index = vars.borrow_index;
            let previous_liquidity_index = vars.liquidity_index;
            let last_updated_timestamp = vars.last_updated_timestamp;
            let (new_liquidity_index, new_borrow_index) = 
            self.update_indexes(vars, total_debt, last_updated_timestamp, previous_liquidity_index, previous_borrow_index);
            //mint_to_treasury ting    mint s_token to the treasury account
        }

        // #[ink(message)]
        // pub fn get_scaled_balance(&self, user: AccountId) -> Balance {
        //     let reserve_data = self
        //         .users_data
        //         .get(&user)
        //         .cloned()
        //         .unwrap_or(Default::default());
        //     let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
        //     let debttoken: IERC20 =
        //         FromAccountId::from_account_id(self.reserve.debt_token_address);
        //     // user balance should always be stoken - debttoken
        //     let mut user_balance = stoken
        //         .balance_of(user)
        //         .saturating_sub(debttoken.balance_of(user));
        //     if reserve_data.last_update_timestamp != 0 {
        //         let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;
        //         let interest = user_balance * interval as u128 * self.reserve.liquidity_rate
        //             / (100 * 365 * 24 * 3600 * 1000);
        //         user_balance += interest;
        //     }
        //     user_balance
        // }

        #[ink(message)]
        pub fn withdraw(&mut self, amount: Balance, to: Option<AccountId>) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = to {
                receiver = behalf;
            }

            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let debttoken: IERC20 =
                FromAccountId::from_account_id(self.reserve.debt_token_address);
            //user balance should always be stoken - debttoken
            let user_balance = stoken.balance_of(sender) - debttoken.balance_of(sender);
            //todo user_balance要大于0，这样算是healthfactor=1
            let reserve_data = self
                .users_data
                .get_mut(&sender)
                .expect("user config does not exist");
            let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;

            let interest = user_balance * interval as u128 * self.reserve.liquidity_rate
                / (100 * 365 * 24 * 3600 * 1000);
            if interest > 0 {
                reserve_data.cumulated_liquidity_interest += interest;
                reserve_data.last_update_timestamp = Self::env().block_timestamp();
            }

            let cur_user_balance = (stoken.balance_of(receiver)  - debttoken.balance_of(receiver)* 100/75) + reserve_data.cumulated_liquidity_interest ;
            assert!(
                amount <= cur_user_balance,
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );

            if amount <= reserve_data.cumulated_liquidity_interest {
                reserve_data.cumulated_liquidity_interest -= amount;
            } else {
                let rest = amount - reserve_data.cumulated_liquidity_interest;
                reserve_data.cumulated_liquidity_interest = 0;
                stoken.burn(sender, rest).expect("sToken burn failed");
            }
            self.env()
                .transfer(receiver, amount)
                .expect("transfer failed");

            self.env().emit_event(Withdraw {
                user: sender,
                to: receiver,
                amount,
            });
        }

        #[ink(message)]
        pub fn borrow(&mut self, amount: Balance, on_behalf_of: AccountId) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            let sender = self.env().caller();
            let receiver = on_behalf_of;

            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut dtoken: IERC20 =
                FromAccountId::from_account_id(self.reserve.debt_token_address);

            // credit delegation allowances check
            let credit_balance = self
                .delegate_allowance
                .get(&(receiver, sender))
                .copied()
                .unwrap_or(0);
            assert!(
                amount <= credit_balance,
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );

            // stoken - debetoken
            let liquidation_threshold =
                stoken.balance_of(receiver)  - dtoken.balance_of(receiver)* 100/75 ;
            assert!(
                amount <= liquidation_threshold,
                "{}",
                LP_NOT_ENOUGH_LIQUIDITY_TO_BORROW
            );

            let reserve_data = self
                .users_data
                .get_mut(&receiver)
                .expect("user config does not exist");
            let interval = Self::env().block_timestamp() - reserve_data.last_update_timestamp;

            // borrow update depositor interest
            let user_balance = stoken.balance_of(receiver) - dtoken.balance_of(receiver) ;
            let interest = user_balance * interval as u128 * self.reserve.liquidity_rate
                / (100 * 365 * 24 * 3600 * 1000);
            reserve_data.cumulated_liquidity_interest += interest;
            reserve_data.last_update_timestamp = Self::env().block_timestamp();

            // update borrow info
            let entry_sender = self.users_data.entry(sender);
            let reserve_data_sender = entry_sender.or_insert(Default::default());
            let interval =
                Self::env().block_timestamp() - reserve_data_sender.last_update_timestamp;
            reserve_data_sender.cumulated_borrow_interest += reserve_data_sender
                .borrow_balance
                * interval as u128
                * self.reserve.borrow_rate
                / (100 * 365 * 24 * 3600 * 1000);
            reserve_data_sender.borrow_balance += amount;
            reserve_data_sender.last_update_timestamp = Self::env().block_timestamp();

            // update delegate amount
            self.delegate_allowance
                .insert((receiver, sender), credit_balance - amount);
            // dtoken
            //     .transfer_from(receiver, sender, credit_balance - amount)
            //     .expect("transfer failed");

            // mint debt token to receiver
            assert!(dtoken.mint(receiver, amount).is_ok());

            // transfer reserve asset to sender
            self.env()
                .transfer(sender, amount)
                .expect("transfer failed");

            self.env().emit_event(Borrow {
                user: sender,
                on_behalf_of,
                amount,
            });
        }


        #[ink(message, payable)]
        pub fn repay(&mut self, on_behalf_of: AccountId) {
            let sender = self.env().caller();
            let recevier = on_behalf_of;

            // get repay amount
            let amount = self.env().transferred_balance();
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            let mut dtoken: IERC20 =
                FromAccountId::from_account_id(self.reserve.debt_token_address);

            // update interest
            let reserve_data_sender = self
                .users_data
                .get_mut(&sender)
                .expect("you have not borrow any dot");
            let interval =
                Self::env().block_timestamp() - reserve_data_sender.last_update_timestamp;
            reserve_data_sender.cumulated_borrow_interest += reserve_data_sender
                .borrow_balance
                * interval as u128
                * self.reserve.borrow_rate
                / (100 * 365 * 24 * 3600 * 1000);
            reserve_data_sender.borrow_balance -= amount;
            reserve_data_sender.last_update_timestamp = Self::env().block_timestamp();

            if amount <= reserve_data_sender.cumulated_borrow_interest {
                reserve_data_sender.cumulated_borrow_interest -= amount
            } else {
                let rest = amount - reserve_data_sender.cumulated_borrow_interest;
                reserve_data_sender.cumulated_borrow_interest = 0;
                dtoken.burn(recevier, rest).expect("debt token burn failed");
            }

            self.env().emit_event(Repay {
                receiver: on_behalf_of,
                repayer: sender,
                amount,
            });
        }

        #[ink(message)]
        pub fn delegate(&mut self, delegatee: AccountId, amount: Balance) {
            let delegator = self.env().caller();
            self.delegate_allowance
                .insert((delegator, delegatee), amount);
        }

        #[ink(message)]
        pub fn delegate_amount(&self, delegator: AccountId, delegatee: AccountId) -> Balance {
            self.delegate_allowance
                .get(&(delegator, delegatee))
                .copied()
                .unwrap_or(0u128)
        }

        #[ink(message)]
        pub fn delegate_of(&self, delegatee: AccountId) -> Vec<(AccountId, Balance)> {
            let mut delegates = vec![];
            for v in self.delegate_allowance.iter() {
                if v.0 .1 == delegatee {
                    delegates.push((v.0 .0, *v.1))
                }
            }
            delegates
        }

        #[ink(message)]
        pub fn set_kyc_data(&mut self, name: Option<String>, email: Option<String>) {
            let user = self.env().caller();
            let entry = self.users_kyc_data.entry(user);
            let kyc_data = entry.or_insert(Default::default());
            if let Some(user_name) = name {
                kyc_data.name = user_name;
            }
            if let Some(user_email) = email {
                kyc_data.email = user_email;
            }         
        }

        #[ink(message)]
        pub fn get_kyc_data(&self, user: AccountId) -> Option<UserKycData> {
            self.users_kyc_data.get(&user).cloned()
        }
    }
}