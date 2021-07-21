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

    /**
     * @dev Emitted on Deposit()
     * @param user The address initiating the deposit
     * @param on_behalf_of The beneficiary of the deposit, receiving the sTokens
     * @param amount The amount deposited
     **/
    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    /**
     * @dev Emitted on Withdraw()
     * @param user The address initiating the withdrawal, owner of sTokens
     * @param to Address that will receive the underlying
     * @param amount The amount to be withdrawn
     **/
    #[ink(event)]
    pub struct Withdraw {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
    
    /**
     * @dev Emitted on Borrow() when debt needs to be opened
     * @param user The address of the user initiating the borrow(), receiving the funds on borrow()
     * @param on_behalf_of The address that will be getting the debt
     * @param amount The amount borrowed out
     **/    
    #[ink(event)]
    pub struct Borrow {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        on_behalf_of: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    /**
     * @dev Emitted on Repay()
     * @param receiver The beneficiary of the repayment, getting his debt reduced
     * @param repayer The address of the user initiating the repay(), providing the funds
     * @param amount The amount repaid
     **/    
    #[ink(event)]
    pub struct Repay {
        #[ink(topic)]
        receiver: AccountId,
        #[ink(topic)]
        repayer: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
    
    /**
     * @dev emitted on Delegate()
     * @param delegator  who have money and allow delegatee use it as collateral
     * @param delegatee who can borrow money from pool without collateral
     * @param amount the amount
     **/    
    #[ink(event)]
    pub struct Delegate {
        #[ink(topic)]
        delegator: AccountId,
        #[ink(topic)]
        delegatee: AccountId,
        #[ink(topic)]
        amount: Balance,
    }
   
    /**
     * @dev emitted on Liquidation() when a borrower is liquidated.
     * @param liquidator The address of the liquidator
     * @param liquidatee The address of the borrower getting liquidated
     * @param amount_to_recover The debt amount of borrowed `asset` the liquidator wants to cover
     * @param received_amount The amount of collateral received by the liiquidator     
     **/    
    #[ink(event)]
    pub struct Liquidation {
        #[ink(topic)]
        liquidator: AccountId,
        #[ink(topic)]
        liquidatee: AccountId,
        #[ink(topic)]
        amount_to_recover: Balance,
        #[ink(topic)]
        received_amount: Balance,
    }

    #[ink(storage)]
    pub struct Lendingpool {
        reserve: ReserveData,
        users_data: StorageHashMap<AccountId, UserReserveData>,
        delegate_allowance: StorageHashMap<(AccountId, AccountId), Balance>,
        users_kyc_data: StorageHashMap<AccountId, UserKycData>,
        interest_setting: InterestRateData,
    }

    impl Lendingpool {  
        #[ink(constructor)]
        pub fn new(
            stoken: AccountId, debt_token: AccountId, 
            ltv: u128, liquidity_threshold: u128, 
            liquidity_bonus: u128, reserve_factor: u128,
            optimal_utilization_rate:u128, 
            rate_slope1: u128, rate_slope2:u128,
        ) -> Self {
            Self {
                reserve: ReserveData::new(
                    stoken,
                    debt_token,
                    ltv,
                    liquidity_threshold,
                    liquidity_bonus
                ),
                users_data: StorageHashMap::new(),
                delegate_allowance: StorageHashMap::new(),
                users_kyc_data: StorageHashMap::new(),
                interest_setting: InterestRateData {
                    optimal_utilization_rate:optimal_utilization_rate,
                    excess_utilization_rate:1 - optimal_utilization_rate,
                    rate_slope1: rate_slope1,
                    rate_slope2: rate_slope2,
                    utilization_rate: Default::default(),
                },
            }
        }

       /**
        * @dev Deposits an `amount` of underlying asset into the reserve, receiving in return overlying sTokens.
        * - E.g. User deposits 100 DOT and gets in return 100 sDOT
        * @param onBehalfOf The address that will receive the sTokens, same as msg.sender if the user
        *   wants to receive them on his own wallet, or a different address if the beneficiary of sTokens
        *   is a different wallet
        **/      
        #[ink(message, payable)]
        pub fn deposit(&mut self, on_behalf_of: Option<AccountId>) {
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = on_behalf_of {
                receiver = behalf;
            }
            let amount = self.env().transferred_balance();
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);

            self.update_interest_rates(amount, 0);
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let entry = self.users_data.entry(receiver);
            let user_reserve_data = entry.or_insert(Default::default());
            user_reserve_data.last_update_timestamp = Self::env().block_timestamp();

            assert!(stoken.mint(receiver, amount).is_ok());         
            self.env().emit_event(Deposit {
                user: sender,
                on_behalf_of: receiver,
                amount,
            });
        }

        pub fn get_normalized_income(&self) -> u128 {
            let timestamp = self.reserve.last_updated_timestamp; 
            if timestamp == self.env().block_timestamp() {
                return self.reserve.liquidity_index
            }
            let cumulated = self.caculate_linear_interest(timestamp) * &self.reserve.liquidity_index;
            cumulated
        }

        pub fn get_normalized_debt(&self) -> u128{
            let timestamp = self.reserve.last_updated_timestamp; 
            if timestamp == self.env().block_timestamp() {
                return ONE;
            }
            let stable_borrow_rate = self.reserve.borrow_rate;
            let cumulated = self.calculate_compounded_interest(stable_borrow_rate,timestamp) * ONE;
            cumulated
        }

        pub fn update_interest_rates(&mut self, liquidity_added: u128, liquidity_taken: u128){
            let debttoken: IERC20 =  FromAccountId::from_account_id(self.reserve.debt_token_address);
            let total_debt = debttoken.total_supply();
            let (new_liquidity_rate, new_borrow_rate) = calculate_interest_rates(&self.reserve, &mut self.interest_setting, liquidity_added, liquidity_taken, total_debt, self.reserve.borrow_rate);
  
            self.reserve.liquidity_rate = new_liquidity_rate;
            self.reserve.borrow_rate = new_borrow_rate;
        }

        fn caculate_linear_interest(&self, last_updated_timestamp: u64) -> u128 {
            let time_difference = self.env().block_timestamp() - last_updated_timestamp;
            let interest:u128 = ONE * self.reserve.liquidity_rate * time_difference as u128 / ONE_YEAR + ONE;
            interest
        }

        fn calculate_compounded_interest(&self, rate:u128, last_update_timestamp:u64) -> u128{
            let time_difference = self.env().block_timestamp() - last_update_timestamp;
            let time_difference = time_difference as u128;
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
            let second_term = time_difference * time_difference_minus_one * base_power_two / 2;
            let third_term = time_difference * time_difference_minus_one * time_difference_minus_two * base_power_three / 6;
            let interest = rate_per_second * time_difference + 1 + second_term + third_term;
            interest
        }
        //double check
        fn update_indexes(&mut self, timestamp:u64, liquidity_index:u128) ->  u128{
            let current_liquidity_rate = self.reserve.liquidity_rate;
            let mut new_liquidity_index = liquidity_index;        
            if current_liquidity_rate > 0 {
                let cumulated_liquidity_interest = self.caculate_linear_interest(timestamp);
                new_liquidity_index *= cumulated_liquidity_interest;
                self.reserve.liquidity_index = new_liquidity_index;                       
            }
            self.reserve.last_updated_timestamp = self.env().block_timestamp();
            new_liquidity_index
        }

        // fn update_state(&mut self){
        //     let previous_liquidity_index = self.reserve.liquidity_index;
        //     let last_updated_timestamp = self.reserve.last_updated_timestamp;
        //     let new_liquidity_index = self.update_indexes(last_updated_timestamp, previous_liquidity_index);
        // }
        
       /**
        * @dev Withdraws an `amount` of underlying asset from the reserve, burning the equivalent sTokens owned
        * E.g. User has 100 sDOT, calls withdraw() and receives 100 DOT, burning the 100 sDOT
        * @param amount The underlying amount to be withdrawn
        *   - Send the value type(uint256).max in order to withdraw the whole sToken balance
        * @param to Address that will receive the underlying, same as msg.sender if the user
        *   wants to receive it on his own wallet, or a different address if the beneficiary is a
        *   different wallet
        **/        
        #[ink(message)]
        pub fn withdraw(&mut self, amount: Balance, to: Option<AccountId>) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let sender = self.env().caller();
            let mut receiver = sender;
            if let Some(behalf) = to {
                receiver = behalf;
            }
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            

            let interest = self.get_normalized_income() * stoken.balance_of(sender) ;
            let debt_interest = self.get_normalized_debt()* debttoken.balance_of(sender);
            let reserve_data = self.users_data.get_mut(&sender).expect("user config does not exist");
            
            if interest > 0 {
                reserve_data.cumulated_liquidity_interest += interest;
                reserve_data.cumulated_borrow_interest += debt_interest;
                //reserve_data.last_update_timestamp = Self::env().block_timestamp();
            }            
            let available_user_balance = stoken.balance_of(sender)  - debttoken.balance_of(sender) + reserve_data.cumulated_liquidity_interest - reserve_data.cumulated_borrow_interest;
            assert!(
                amount <= available_user_balance,
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );
            assert!(balance_decrease_allowed(&mut self.reserve, sender, amount),
                "{}",
                VL_TRANSFER_NOT_ALLOWED
            );
            if amount <= reserve_data.cumulated_liquidity_interest {
                reserve_data.cumulated_liquidity_interest -= amount;
            } else {
                let rest = amount - reserve_data.cumulated_liquidity_interest;
                reserve_data.cumulated_liquidity_interest = 0;
                stoken.burn(sender, rest).expect("sToken burn failed");
            }
            reserve_data.last_update_timestamp = Self::env().block_timestamp();
            // self.update_state();
            self.update_interest_rates(0, amount);

            self.env().transfer(receiver, amount).expect("transfer failed"); 
            
            self.env().emit_event(Withdraw {
                user: sender,
                to: receiver,
                amount,
            });
        }

       /**
        * @dev Allows users to borrow a specific `amount` of the reserve underlying asset, provided that the borrower
        * already deposited enough collateral, or he was given enough allowance by a credit delegator on the
        * corresponding debt token
        * - E.g. User borrows 100 DOT passing as `onBehalfOf` his own address, receiving the 100 DOT in his wallet
        *   and 100 debt tokens
        * @param amount The amount to be borrowed
        * @param onBehalfOf Address of the user who will receive the debt. Should be the address of the borrower itself
        * calling the function if he wants to borrow against his own collateral, or the address of the credit delegator
        * if he has been given credit delegation allowance
        **/        
        #[ink(message)]
        pub fn borrow(&mut self, amount: Balance, on_behalf_of: AccountId) {
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let sender = self.env().caller();
            let receiver = on_behalf_of;
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut dtoken: IERC20 =FromAccountId::from_account_id(self.reserve.debt_token_address);

            //let unit_price = self.env().extension().fetch_price();
            let unit_price = 16;//小数点！
            let amount_in_usd = unit_price * amount;

            //本来要加max_borrow_size_percent,考虑到初期这里太多限制，不加了
            let credit_balance = self.delegate_allowance.get(&(receiver, sender)).copied().unwrap_or(0);

            let interest = self.get_normalized_income() * stoken.balance_of(sender) ;
            let debt_interest = self.get_normalized_debt()* dtoken.balance_of(sender);
            let reserve_data = self.users_data.get_mut(&sender).expect("user config does not exist");

            if interest > 0 {//如果receiver和sender不一样要重加
                reserve_data.cumulated_liquidity_interest += interest;
                reserve_data.cumulated_borrow_interest += debt_interest;
                //reserve_data.last_update_timestamp = Self::env().block_timestamp();
            }        
            //let reserve_data = self.users_data.get_mut(&receiver).expect("user config does not exist");
            // let entry_sender = self.users_data.entry(sender);
            // let reserve_data_sender = entry_sender.or_insert(Default::default());

            let _credit_balance = stoken.balance_of(sender)  - dtoken.balance_of(sender) + reserve_data.cumulated_liquidity_interest - reserve_data.cumulated_borrow_interest;
            assert!(
                amount <= _credit_balance, 
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );
            //这里要用balance_decrease_allowed
            let health_factor_after_decrease = 10;
            assert!(
                health_factor_after_decrease >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD, 
                "{}",
                VL_HEALTH_FACTOR_LOWER_THAN_LIQUIDATION_THRESHOLD
            );

            self.delegate_allowance.insert((receiver, sender), credit_balance - amount);
            assert!(dtoken.mint(receiver, amount).is_ok());            
            self.env().transfer(sender, amount).expect("transfer failed");//没说明什么币？
            //要更新双方user_reserve_date,如果receiver和sender

            //self.update_state();
            self.update_interest_rates(0, amount);//这个要考虑是不是要两个，因为是双方！
            self.env().emit_event(Borrow {
                user: sender,
                on_behalf_of,
                amount,
            });
        }

       /**
        * @notice Repays a borrowed `amount` on a specific reserve, burning the equivalent debt tokens owned
        * - E.g. User repays 100 DOT, burning 100 debt tokens of the `onBehalfOf` address
        * - Send the value type(uint256).max in order to repay the whole debt for `asset` on the specific `debtMode`
        * @param onBehalfOf Address of the user who will get his debt reduced/removed. Should be the address of the
        * user calling the function if he wants to reduce/remove his own debt, or the address of any other
        * other borrower whose debt should be removed
        **/        
        #[ink(message, payable)]
        pub fn repay(&mut self, on_behalf_of: AccountId) {
            let sender = self.env().caller();
            let recevier = on_behalf_of;
            let amount = self.env().transferred_balance();
            assert_ne!(amount, 0, "{}", VL_INVALID_AMOUNT);
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut dtoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            
            let interest = self.get_normalized_income() * stoken.balance_of(sender) ;
            let debt_interest = self.get_normalized_debt()* dtoken.balance_of(sender);
            let reserve_data_sender = self.users_data.get_mut(&sender).expect("you have not borrow any dot");
            if interest > 0 {
                reserve_data_sender.cumulated_liquidity_interest += interest;
                reserve_data_sender.cumulated_borrow_interest += debt_interest;
                //reserve_data.last_update_timestamp = Self::env().block_timestamp();
            }
            if amount <= reserve_data_sender.cumulated_borrow_interest {
                reserve_data_sender.cumulated_borrow_interest -= amount
            } else {
                let rest = amount - reserve_data_sender.cumulated_borrow_interest;
                reserve_data_sender.cumulated_borrow_interest = 0;
                reserve_data_sender.borrow_balance -= amount;
                dtoken.burn(recevier, rest).expect("debt token burn failed");
            }
            reserve_data_sender.last_update_timestamp = Self::env().block_timestamp();
            // self.update_state();
            self.update_interest_rates(amount,0);
            self.env().emit_event(Repay {
                receiver: on_behalf_of,
                repayer: sender,
                amount,
            });
        }

        #[ink(message)]
        pub fn get_reserve_data(&self) -> (u128,u128,u128,u128) {
            (self.reserve.ltv, self.reserve.liquidity_threshold, self.reserve.liquidity_bonus, self.reserve.decimals)
        }

        #[ink(message)]
        pub fn get_user_reserve_data(&self, user: AccountId) -> UserReserveData {
            *self.users_data.get(&user).unwrap()
        }
        
        #[ink(message)]
        pub fn get_interest_rate_data(&self) -> (u128,u128,u128,u128,u128) {
            (self.interest_setting.optimal_utilization_rate, self.interest_setting.excess_utilization_rate, self.interest_setting.rate_slope1, self.interest_setting.rate_slope2, self.interest_setting.utilization_rate)
        }

        pub fn set_reserve_configuration(&mut self, ltv: u128, liquidity_threshold: u128, liquidity_bonus: u128, decimals: u128){
            self.reserve.ltv = ltv;
            self.reserve.liquidity_threshold = liquidity_threshold;
            self.reserve.liquidity_bonus = liquidity_bonus;
            self.reserve.decimals = decimals;
        }

        pub fn set_interest_rate_data(&mut self, optimal_utilization_rate:u128, rate_slope1: u128, rate_slope2:u128){
            self.interest_setting.optimal_utilization_rate = optimal_utilization_rate;
            self.interest_setting.rate_slope1 = rate_slope1;
            self.interest_setting.rate_slope2 = rate_slope2;      
        }        

        /**
         * @dev delgator can delegate some their own credits which get by deposit funds to delegatee
         * @param delegatee who can borrow without collateral
         * @param amount
         */        
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

       /**
        * @dev Function to liquidate a non-healthy position collateral-wise, with Health Factor below 1
        * - The caller (liquidator) covers `debt_to_cover` amount of debt of the user getting liquidated, and receives
        *   a proportionally amount of the `collateralAsset` plus a bonus to cover market risk
        * @param borrower The address of the borrower getting liquidated
        * @param debt_to_cover The debt amount of borrowed `asset` the liquidator wants to cover
        * @param receive_s_token `true` if the liquidators wants to receive the collateral aTokens, `false` if he wants
        * to receive the underlying collateral asset directly
        **/        
        #[ink(message)]
        pub fn liquidation_call(&mut self, borrower:AccountId, debt_to_cover:u128, receive_s_token:bool){
            let liquidator = self.env().caller();
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            
            //todo 直接算出来？直接算出来因为始时
            let borrower_total_debt_in_usd = 10; 
            //todo 直接算出来？直接算出来因为始时
            let borrower_total_balance = 5;
            let borrower_total_balance_in_usd = 10;
            let health_factor = calculate_health_factor_from_balance(borrower_total_balance_in_usd, borrower_total_debt_in_usd, self.reserve.liquidity_threshold);
            assert!(
                health_factor <= HEALTH_FACTOR_LIQUIDATION_THRESHOLD, 
                "{}",
                LPCM_HEALTH_FACTOR_NOT_BELOW_THRESHOLD
            );
            assert!(
                borrower_total_debt_in_usd > 0, 
                "{}",
                LPCM_SPECIFIED_CURRENCY_NOT_BORROWED_BY_USER
            );
            let max_liquidatable_debt = borrower_total_debt_in_usd * LIQUIDATION_CLOSE_FACTOR_PERCENT;
            let mut actual_debt_to_liquidate = 0;
            if debt_to_cover > max_liquidatable_debt {
                actual_debt_to_liquidate = max_liquidatable_debt
            } else {
                actual_debt_to_liquidate = debt_to_cover
            }
            let (max_collateral_to_liquidate, debt_amount_needed) = caculate_available_collateral_to_liquidate(&self.reserve, actual_debt_to_liquidate, borrower_total_balance);
            if debt_amount_needed < actual_debt_to_liquidate {
                actual_debt_to_liquidate = debt_amount_needed;
            }
            if !receive_s_token {
                let available_dot = self.env().balance(); 
                assert!(
                    available_dot > max_collateral_to_liquidate, 
                    "{}",
                    LPCM_NOT_ENOUGH_LIQUIDITY_TO_LIQUIDATE
                );
            } 
            //self.update_state();
           debttoken.burn(borrower, actual_debt_to_liquidate).expect("debt token burn failed");
           self.update_interest_rates(actual_debt_to_liquidate,0);
           if receive_s_token{
               stoken.transfer_from(borrower, liquidator, max_collateral_to_liquidate);                   
           } else {
            //self.update_state();
            self.update_interest_rates(0,max_collateral_to_liquidate);
            stoken.burn(borrower, max_collateral_to_liquidate).expect("stoken burn failed");
            //transfer  max_collateral_to_liquidate dot back to liqudator
           }
           //这里要加两个interest的更新
           let borrower_data = self.users_data.get_mut(&borrower).expect("user config does not exist");
           borrower_data.borrow_balance -= actual_debt_to_liquidate;
           borrower_data.last_update_timestamp = Self::env().block_timestamp();
           self.env().emit_event(Liquidation {
            liquidator,
            liquidatee: borrower,
            amount_to_recover:actual_debt_to_liquidate,
            received_amount: max_collateral_to_liquidate,
        });
        }
        //david
        #[ink(message)]
        pub fn is_user_reserve_healthy(&self, user: AccountId) -> bool{ true }
        #[ink(message)]
        pub fn get_the_unhelthy_reserves(&self){} //Option<AccountId>        
    }
}