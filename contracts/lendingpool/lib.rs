#![cfg_attr(not(feature = "std"), no_std)]

mod types;
use ink_lang as ink;

#[ink::contract]
mod lendingpool {
    use crate::types::*;
    use ierc20::IERC20;
    use price::Price;
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
        users: StorageHashMap<AccountId,u8>, //accountid -> 1/0
        borrow_status:StorageHashMap<(AccountId, AccountId), Balance>,
    }

    impl Lendingpool {  
        #[ink(constructor)]
        pub fn new(
            stoken: AccountId, debt_token: AccountId,
            oracle_price_address: AccountId, 
            ltv: u128, liquidity_threshold: u128, 
            liquidity_bonus: u128,
            optimal_utilization_rate:u128, 
            rate_slope1: u128, rate_slope2:u128,
        ) -> Self {
            Self {
                reserve: ReserveData::new(
                    stoken,
                    debt_token,
                    oracle_price_address,
                    ltv,
                    liquidity_threshold,
                    liquidity_bonus
                ),
                users_data: StorageHashMap::new(),
                delegate_allowance: StorageHashMap::new(),
                users_kyc_data: StorageHashMap::new(),
                interest_setting: InterestRateData::new(
                    optimal_utilization_rate,
                    rate_slope1,
                    rate_slope2,
                ),
                users: Default::default(),
                borrow_status: Default::default(),
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
            self.update_liquidity_index();
            self.update_interest_rates(amount, 0);
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let entry = self.users_data.entry(receiver);
            let user_reserve_data = entry.or_insert(Default::default());
            user_reserve_data.last_update_timestamp = Self::env().block_timestamp();
            assert!(stoken.mint(receiver, amount).is_ok());  
            self.users.insert(receiver,1);  //active user       
            self.env().emit_event(Deposit {
                user: sender,
                on_behalf_of: receiver,
                amount,
            });
        }

        fn get_normalized_income(&self) -> u128 {
            let timestamp = self.reserve.last_updated_timestamp; 
            if timestamp == self.env().block_timestamp() {
                return self.reserve.liquidity_index
            }
            let cumulated = self.caculate_linear_interest(timestamp) * &self.reserve.liquidity_index;
            cumulated
        }

        fn get_normalized_debt(&self) -> u128 {
            let timestamp = self.reserve.last_updated_timestamp; 
            if timestamp == self.env().block_timestamp() {
                return ONE;
            }
            let stable_borrow_rate = self.reserve.borrow_rate;
            let cumulated = self.calculate_compounded_interest(stable_borrow_rate,timestamp) * ONE;
            cumulated
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
        
        fn update_liquidity_index(&mut self){
            let current_liquidity_rate = self.reserve.liquidity_rate;
            if current_liquidity_rate > 0 {
                let cumulated_liquidity_interest = self.caculate_linear_interest(self.reserve.last_updated_timestamp);
                self.reserve.liquidity_index = self.reserve.liquidity_index * cumulated_liquidity_interest;
                self.reserve.last_updated_timestamp = self.env().block_timestamp();
            }
        }

        fn update_interest_rates(&mut self, liquidity_added: u128, liquidity_taken: u128) {
            let debttoken: IERC20 =  FromAccountId::from_account_id(self.reserve.debt_token_address);
            let total_debt = debttoken.total_supply();
            let (new_liquidity_rate, new_borrow_rate) = calculate_interest_rates(&self.reserve, &mut self.interest_setting, liquidity_added, liquidity_taken, total_debt, self.reserve.borrow_rate);
            self.reserve.liquidity_rate = new_liquidity_rate;
            self.reserve.borrow_rate = new_borrow_rate;
        }

        /**
        * @dev Withdraws an `amount` of underlying asset from the reserve, burning the equivalent sTokens owned
        * E.g. User has 100 sDOT, calls withdraw() and receives 100 DOT, burning the 100 sDOT
        * @param amount The underlying amount to be withdrawn
        *   - Send the value in order to withdraw the whole sToken balance
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
            self.update_liquidity_index();
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
        * was given enough allowance by a credit delegator on the
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
            let mut dtoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            let credit_balance = self.delegate_allowance.get(&(receiver, sender)).copied().unwrap_or(0);
            //在单币种的模式下，只考虑A用户授权给B，B来借款，不然这条assert需要有条件判断
            assert!(
                amount <= credit_balance, 
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );       
            let interest = self.get_normalized_income() * stoken.balance_of(receiver) ;
            let debt_interest = self.get_normalized_debt()* dtoken.balance_of(receiver);
            let reserve_data = self.users_data.get_mut(&receiver).expect("user config does not exist");
            if interest > 0 {
                reserve_data.cumulated_liquidity_interest += interest;
                reserve_data.cumulated_borrow_interest += debt_interest;
            }        
            let _credit_balance = stoken.balance_of(receiver)  - dtoken.balance_of(receiver) + reserve_data.cumulated_liquidity_interest - reserve_data.cumulated_borrow_interest;
            assert!(
                amount <= _credit_balance, 
                "{}",
                VL_NOT_ENOUGH_AVAILABLE_USER_BALANCE
            );
            reserve_data.last_update_timestamp = Self::env().block_timestamp();
            //reserve_data.borrow_balance += amount;

            balance_decrease_allowed(&mut self.reserve, receiver, amount);

            self.delegate_allowance.insert((receiver, sender), credit_balance - amount);
            assert!(dtoken.mint(receiver, amount).is_ok());
            self.borrow_status.entry((sender,receiver)).and_modify(|old_value| *old_value+= amount).or_insert(amount);        
            self.env().transfer(sender, amount).expect("transfer failed");
            
            self.update_liquidity_index();
            self.update_interest_rates(0, amount);
            self.env().emit_event(Borrow {
                user: sender,
                on_behalf_of,
                amount,
            });
        }

        /**
        * @notice Repays a borrowed `amount` on a specific reserve, burning the equivalent debt tokens owned
        * - E.g. User repays 100 DOT, burning 100 debt tokens of the `onBehalfOf` address
        * - Send the value in order to repay the debt for `asset`
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
            }
            if amount <= reserve_data_sender.cumulated_borrow_interest {
                reserve_data_sender.cumulated_borrow_interest -= amount;
                
            } else {
                let rest = amount - reserve_data_sender.cumulated_borrow_interest;
                reserve_data_sender.cumulated_borrow_interest = 0;
                //reserve_data_sender.borrow_balance -= amount;
                dtoken.burn(recevier, rest).expect("debt token burn failed");
                //如果还钱少于借款利息 是看不到变化的
                self.borrow_status.entry((sender,recevier)).and_modify(|old_value| *old_value-= rest);
            }
            reserve_data_sender.last_update_timestamp = Self::env().block_timestamp();
            self.update_liquidity_index();
            self.update_interest_rates(amount,0);
            self.env().emit_event(Repay {
                receiver: on_behalf_of,
                repayer: sender,
                amount,
            });
        }
        
        #[ink(message)]
        pub fn get_interest_rate_data(&self) -> (u128, u128, u128, u128, u128) {
            (
                self.interest_setting.optimal_utilization_rate, 
                self.interest_setting.excess_utilization_rate, 
                self.interest_setting.rate_slope1, 
                self.interest_setting.rate_slope2, 
                self.interest_setting.utilization_rate
            )
        } 

        /**
         * @dev delgator can delegate some their own credits which get by deposit funds to delegatee
         * @param delegatee who can borrow without collateral
         * @param amount
         */ 
        #[ink(message)]
        pub fn delegate(&mut self, delegatee: AccountId, amount: Balance) {
            let delegator = self.env().caller();
            self.delegate_allowance.insert((delegator, delegatee), amount);
        }

        #[ink(message)]
        pub fn delegate_amount(&self, delegator: AccountId, delegatee: AccountId) -> Balance {
            self.delegate_allowance.get(&(delegator, delegatee)).copied().unwrap_or(0u128)
        }

        //谁给我delegate了
        //如果要开放这个函数
        //建议改成直接由sender作为delegatee参数，不要自己填参数，这样只有自己能查自己的，别人不能查
        #[ink(message)]
        pub fn delegate_from(&self) -> Vec<(AccountId, Balance)> {
            let delegatee = self.env().caller();
            let mut delegators = vec![];
            for v in self.delegate_allowance.iter() {
                if v.0 .1 == delegatee {
                    delegators.push((v.0 .0, *v.1))
                }
            }
            delegators
        }

        //我给谁delegate了
        //如果要开放这个函数
        //建议改成直接由sender作为delegator参数，不要自己填参数，这样只有自己能查自己的，别人不能查
        //查到的value是u128的，所以要做处理
        #[ink(message)]
        pub fn delegate_to(&self) -> Vec<(AccountId, Balance)> {
            let delegator = self.env().caller();
            let mut delegatees = vec![];
            for v in self.delegate_allowance.iter() {
                if v.0 .0 == delegator {
                    delegatees.push((v.0 .1, *v.1))
                }
            }
            delegatees
        }

        /**
        * @dev Function to liquidate a non-healthy position collateral-wise, with Health Factor below 1
        * - The caller (liquidator) covers `debt_to_cover` amount of debt of the user getting liquidated, and receives
        *   a proportionally amount of the `collateralAsset` plus a bonus to cover market risk
        * @param borrower The address of the borrower getting liquidated
        * @param debt_to_cover The debt amount of borrowed `asset` the liquidator wants to cover
        * @param receive_s_token `true` if the liquidators wants to receive the collateral sTokens, `false` if he wants
        * to receive the underlying collateral asset directly
        **/  
        #[ink(message)]
        pub fn liquidation_call(&mut self, borrower:AccountId, debt_to_cover:u128, receive_s_token:bool){
            let liquidator = self.env().caller();
            let mut stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            
            let mut oracle: Price = FromAccountId::from_account_id(self.reserve.oracle_price_address);
            oracle.update().expect("Failed to update price");
            let unit_price = oracle.get();
            let borrower_total_debt_in_usd = debttoken.balance_of(borrower) * unit_price; 
            let borrower_total_balance_in_usd = stoken.balance_of(borrower) * unit_price;
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
            let mut actual_debt_to_liquidate;
            if debt_to_cover > max_liquidatable_debt {
                actual_debt_to_liquidate = max_liquidatable_debt
            } else {
                actual_debt_to_liquidate = debt_to_cover
            }
            //最后一个参数
            let (max_collateral_to_liquidate, debt_amount_needed) = caculate_available_collateral_to_liquidate(&self.reserve, actual_debt_to_liquidate, stoken.balance_of(borrower));
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
            self.update_liquidity_index();
           debttoken.burn(borrower, actual_debt_to_liquidate).expect("debt token burn failed");
           self.update_interest_rates(actual_debt_to_liquidate,0);
           if receive_s_token{
               stoken.transfer_from(borrower, liquidator, max_collateral_to_liquidate).expect("transfer stoken failed");                   
           } else {
            self.update_liquidity_index();
            self.update_interest_rates(0,max_collateral_to_liquidate);
            stoken.burn(borrower, max_collateral_to_liquidate).expect("stoken burn failed");
            //transfer max_collateral_to_liquidate dot back to liqudator
            self.env().transfer(liquidator, max_collateral_to_liquidate).expect("transfer failed");
           }

           let borrower_data = self.users_data.get_mut(&borrower).expect("user config does not exist");
           //borrower_data.borrow_balance -= actual_debt_to_liquidate;
           borrower_data.last_update_timestamp = Self::env().block_timestamp();
           self.env().emit_event(Liquidation {
            liquidator,
            liquidatee: borrower,
            amount_to_recover:actual_debt_to_liquidate,
            received_amount: max_collateral_to_liquidate,
        });
        }
        
        #[ink(message)]
        pub fn is_user_reserve_healthy(&self, user: AccountId) -> u128{
            let debttoken: IERC20 =  FromAccountId::from_account_id(self.reserve.debt_token_address);
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut oracle: Price = FromAccountId::from_account_id(self.reserve.oracle_price_address);
            oracle.update().expect("Failed to update price");
            let unit_price = oracle.get();
            //if user not exist should return 0
            if !self.users_data.get(&user).is_some(){
                return 0;
            };
            let _total_collateral_in_usd = unit_price * stoken.balance_of(user);
            let _total_debt_in_usd = unit_price * debttoken.balance_of(user);
            let health_factor = calculate_health_factor_from_balance(_total_collateral_in_usd, _total_debt_in_usd, self.reserve.liquidity_threshold);
            health_factor
        }

        /**
        * Get reserve data * total market supply * available liquidity 
        * total lending * utilization rate 
        **/
        #[ink(message)]
        pub fn get_reserve_data_ui(&self) -> (u128, u128, u128, u128){
            let debttoken: IERC20 =  FromAccountId::from_account_id(self.reserve.debt_token_address);
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let total_stoken: Balance = stoken.total_supply();
            let total_dtoken: Balance = debttoken.total_supply();
            let available_liquidity = total_stoken - total_dtoken;
            //todo
            let utilization_rate = total_dtoken * 1000_000_000 / total_stoken  * 100 ;
            (total_stoken, available_liquidity, total_dtoken, utilization_rate)
        }

        /**
        * liquidity_rate * borrow_rate * ltv * liquidity_threshold
        * liquidity_bonus * decimals * last_updated_timestamp
        **/
        #[ink(message)]
        pub fn get_reserve_data(&self) -> (u128, u128, u128, u128, u128, u128, u64){
            return (
                self.reserve.liquidity_rate, self.reserve.borrow_rate,
                self.reserve.ltv, self.reserve.liquidity_threshold, 
                self.reserve.liquidity_bonus, self.reserve.decimals, 
                self.reserve.last_updated_timestamp
            )
        } 

        /**
        * Get user reserve data * total deposit * total borrow * deposit interest
        * borrow interest *current timestamp 
        **/        
        #[ink(message)]
        pub fn get_user_reserve_data_ui(&self, user: AccountId) -> (u128, u128, u128, u128, u64) {
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let dtoken: IERC20 = FromAccountId::from_account_id(self.reserve.debt_token_address);
            let user_stoken: Balance = stoken.balance_of(user);
            let user_dtoken: Balance = dtoken.balance_of(user);
            let interest = self.get_normalized_income() * user_stoken;
            let debt_interest = self.get_normalized_debt()* user_dtoken;
            let data = self.users_data.get(&user);
            match data {
                None => return (0, 0, 0, 0, 0),
                Some(some_data) => {
                    let cumulated_liquidity_interest = some_data.cumulated_liquidity_interest + interest;
                    let cumulated_borrow_interest = some_data.cumulated_borrow_interest + debt_interest;
                    let current_timestamp = Self::env().block_timestamp();
                    return (user_stoken, cumulated_liquidity_interest, user_dtoken, cumulated_borrow_interest, current_timestamp);
                },
            }
        }

        #[ink(message)]
        pub fn get_user_borrow_status(&self)-> Vec<(AccountId,Balance)>{
            let sender = self.env().caller();
            let mut result = Vec::new();
            for ((borrower,owner),value) in self.borrow_status.iter(){
                if *borrower == sender{
                    result.push((*owner,*value));
                }
            }
            result
        }

        //暂不开放
        pub fn set_reserve_configuration(&mut self, ltv: u128, liquidity_threshold: u128, liquidity_bonus: u128){
            self.reserve.ltv = ltv;
            self.reserve.liquidity_threshold = liquidity_threshold;
            self.reserve.liquidity_bonus = liquidity_bonus;
        }
        //暂不开放
        pub fn set_interest_rate_data(
            &mut self, optimal_utilization_rate:u128, 
            rate_slope1: u128, rate_slope2:u128)
            {
                self.interest_setting.optimal_utilization_rate = optimal_utilization_rate;
                self.interest_setting.rate_slope1 = rate_slope1;
                self.interest_setting.rate_slope2 = rate_slope2;                
        }
        //暂不开放
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
        //暂不开放
        pub fn get_kyc_data(&self, user: AccountId) -> Option<UserKycData> {
            self.users_kyc_data.get(&user).cloned()
        }
        //暂不开放
        pub fn get_the_unhelthy_reserves(&self)-> Option<Vec<AccountId>>{
            let debttoken: IERC20 =  FromAccountId::from_account_id(self.reserve.debt_token_address);
            let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
            let mut oracle: Price = FromAccountId::from_account_id(self.reserve.oracle_price_address);
            oracle.update().expect("Failed to update price");
            let unit_price = oracle.get();
            let mut result = Vec::new();
            for (user, status) in self.users.iter(){
                if *status != 1 {
                    //1表示这个用户有钱存在池子里 是我们的用户，0表示用户把所有钱都取走了，相当于不是我们的用户了
                    continue
                }
                let _total_collateral_in_usd = unit_price * stoken.balance_of(*user);
                let _total_debt_in_usd = unit_price * debttoken.balance_of(*user);
                if calculate_health_factor_from_balance(_total_collateral_in_usd, _total_debt_in_usd, self.reserve.liquidity_threshold) <HEALTH_FACTOR_LIQUIDATION_THRESHOLD {
                    result.push(*user)
                }
            }
            if result.is_empty(){
                None
            }else{
                Some(result)
            }
        } 

    }
}