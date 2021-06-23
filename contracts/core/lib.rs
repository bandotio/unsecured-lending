#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod core {
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use ink_storage::collections::HashMap;
    pub const ONE_YEAR:u128 = 365;
    pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 = 1; 
    pub const REBALANCE_UP_USAGE_RATIO_THRESHOLD:u128 = 95; 

    #[derive(Debug,PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub enum InterestRateMode {
        NONE, 
        STABLE,
        VARIABLE,
    }

    #[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub struct UserData {
        principal_borrow_balance: u128,
        last_variable_borrow_cumulative_index: u128, 
        origination_fee: u128,
        stable_borrow_rate: u128,
        last_update_timestamp: u64,
        use_as_collateral: bool,
    }
    //to-do reserveconfiguration
    #[ink(storage)]
    pub struct ReserveData {
        liquidity_index: u128,
        variable_borrow_index: u128,
        current_liquidity_rate: u128,
        current_variable_borrow_rate: u128,
        current_stable_borrow_rate: u128,
        last_update_timestamp: u64, 
        borrowing_enabled: bool,
    }

    impl ReserveData {
        #[ink(constructor)]
        pub fn new() -> Self{
            unimplemented!()
        }
        #[ink(message)]
        pub fn get_normalized_income(&self) -> u128{
            let timestamp = self.last_update_timestamp; 
            if timestamp == Self::env().block_timestamp() {
                return self.liquidity_index
            }
            // to-do, replace *，
            let cumulated:u128 = self.caculate_linear_interest() * &self.liquidity_index;
            cumulated
        }

       fn caculate_linear_interest(&self) -> u128{
            let time_difference = Self::env().block_timestamp() - &self.last_update_timestamp;
            //let interest:u128 = &self.current_liquidity_rate * time_difference.into() / ONE_YEAR + 1; 
            let interest:u128 = 0;
            interest
        }

        #[ink(message)]
        pub fn get_normalized_debt(&self) -> u128{
            let timestamp = self.last_update_timestamp; 
            if timestamp == Self::env().block_timestamp() {
                return self.variable_borrow_index
            }
            let cumulated:u128 = calculate_compounded_interest(&self.variable_borrow_index, timestamp) * &self.variable_borrow_index;
            cumulated
        }

        #[ink(message)]
        pub fn update_indexes(&self,scaled_variable_debt:u128, timestamp:u64) -> (u128, u128){
            let mut new_liquidity_index = self.liquidity_index;
            let mut new_variable_borrow_index = self.variable_borrow_index;

            if self.current_liquidity_rate > 0 {
                let cumulated_liquidity_interest = self.caculate_linear_interest();
                new_liquidity_index *= cumulated_liquidity_interest;
                //todo 不要让new_liquidity_index overflow
                // todo self.liquidity_index = new_liquidity_index
                if scaled_variable_debt ==! 0 {
                    let cumulated_variable_borrow_interest = calculate_compounded_interest(&self.current_liquidity_rate, timestamp);
                    new_variable_borrow_index *= cumulated_variable_borrow_interest;
                    //todo 不要overflow
                    //todo self.variable_borrow_index = new_variable_borrow_index
                }
            }
            //todo  self.last_update_timestamp = Self::env().block_timestamp();
            (new_liquidity_index, new_variable_borrow_index)
        }

        //admin allowed only, call it when it comes to system issues
        fn set_borrowing_enabled(&self, enabled: bool) {
            // todo self.borrowing_enabled : enabled
        }

        fn update_state(&self){
            // get scaled_variable_debt
            // get previous_variable_borrow_index
            // get previous_liquidity_index
            // last_updated_timestamp
            // call update_indexes
            // call mint_to_treasury
        }

    
    } 

    fn calculate_compounded_interest(rate:&u128, last_update_timestamp:u64) -> u128{
        //let time_difference = ink_env::block_timestamp() - last_update_timestamp;
        let time_difference = 0;
        if time_difference == 0 {
            return 1
        } 
        let time_difference_minus_one = time_difference - 1;
        let time_difference_minus_two = if time_difference > 2{
            time_difference - 2
        } else {
            0
        };
        // let rate_per_second = rate / ONE_YEAR;
        // let base_power_two = rate_per_second * rate_per_second;
        // let base_power_three = base_power_two * rate_per_second;
        // let second_term = time_difference * time_difference_minus_one * base_power_two / 2;
        // let third_term = time_difference * time_difference_minus_one * time_difference_minus_two * base_power_three / 6;
        // let interest = rate_per_second * time_difference + 1 + second_term + third_term;
        let interest:u128 = 0;
        interest
    }

    fn calculate_health_factor_from_balance(total_collateral_in_usd:u128, total_debt_in_usd:u128, liquidation_threshold:u128) -> u128{
        if total_debt_in_usd == 0 { return 0};
        let result = total_collateral_in_usd * liquidation_threshold / total_debt_in_usd;
        result
    }

    fn calculate_available_borrows_in_usd(total_collateral_in_usd:u128, total_debt_in_usd:u128, ltv:u128) -> u128{
        let mut available_borrows_in_usd = total_collateral_in_usd * ltv;
        if available_borrows_in_usd < total_debt_in_usd { return 0};
        available_borrows_in_usd -= total_debt_in_usd;
        available_borrows_in_usd
    }
    
    struct LendingPoolStorage {
        _reserves_list: HashMap<u128, AccountId>,
        _reserves_count: u128,
        //_reserves: HashMap<AccountId, ReserveData>,
    }
 
    struct CalculateUserAccountData {
        reserve_unit_price: u128,
        token_unit: u128,
        compounded_liquidity_balance: u128,
        compound_borrow_balance: u128,
        decimals: u128,
        ltv: u128,
        liquidation_threshold: u128,
        i: u128,
        health_factor: u128,
        total_collateral_in_usd: u128,
        total_debt_in_usd: u128,
        avg_ltv: u128,
        avg_liquidation_threshold: u128,
        current_reserve_address: AccountId,
    }

    fn calculate_user_account_data( vars:&mut CalculateUserAccountData, user:AccountId, token:AccountId, user_config:Option<UserData>) -> (u128, u128, u128, u128, u128){
        if user_config == None {return (0,0,0,0,0);}
        //loop all the reserves to {
            //make sure only use the reserves that are used for collateral and borrowing
        //get liquidation_threshold, decimals and ltv from the above reserves
        //let var.token_unit = 10**var.decimals;
        //get the reserves' price
        //if var.liquidation_threshold != 0 && user_config.use_as_collateral {
            //get the s token balance of the user from the reserve(compounded_liquidity_balance)
            //use the oracle to present the s tokens above in usd(liquidity_balance_in_usd)
            //total_collateral_in_usd += liquidity_balance_in_usd
            // avg_ltv += (liquidity_balance_in_usd * ltv)
            // ave_liquidation_threshold += (liquidity_balance_in_usd * liquidation_threshold)
        //}
        //if user_config.is_borrowing {
            // compounded_borrow_balance = stabledebt + variable_debt
            // total_debt_in_usd = compounded_borrow_balance -> oracle
        //}
        //}
        vars.total_collateral_in_usd = 0; //place_holder
        vars.total_debt_in_usd = 0;//place_holder
        
        if vars.total_collateral_in_usd == 0 { 
            vars.avg_ltv = 0;
            vars.avg_liquidation_threshold = 0
        } else {
            vars.avg_ltv /= vars.total_collateral_in_usd;
            vars.avg_liquidation_threshold /= vars.total_collateral_in_usd
        }
        vars.health_factor = calculate_health_factor_from_balance(
            vars.total_collateral_in_usd,
            vars.total_debt_in_usd,
            vars.avg_liquidation_threshold
        );
        (vars.total_collateral_in_usd, vars.total_debt_in_usd, vars.avg_ltv, vars.avg_liquidation_threshold, vars.health_factor)
    }

    struct BalanceDecreaseAllowedData{
        decimals: u128,
        liquidation_threshold: u128,
        total_collateral_in_usd: u128,
        total_debt_in_usd: u128,
        avg_liquidation_threshold: u128,
        amount_to_decrease_in_usd: u128,
        collateral_after_decrease: u128,
        liquidation_threshold_after_decrease: u128,
        health_factor_after_decrease: u128,
    }

    fn balance_decrease_allowed(vars:&mut BalanceDecreaseAllowedData, asset:AccountId, user:AccountId, amount:u128, user_config:UserData) -> bool{
        //if user in this reserve is not borrowing any and is not using as collateral then return true
        //get liquidation_threshold and decimals from the reserve getParams()
        if vars.liquidation_threshold == 0 {
            return true;
        }
        // (vars.total_collateral_in_usd, vars.total_debt_in_usd, , vars.avg_liquidation_threshold, )  =
        // calculate_user_account_data(_, user, asset, user_config);//解决引用问题，需用加个mapping可能！

        if vars.total_debt_in_usd ==0{ return true;}
        // vars.amount_to_decrease_in_usd = asset_oracle_usdprice * amount /10**vars.decimals
        // vars.collateral_after_decrease = vars.total_collateral_in_usd - vars.amount_to_decrease_in_usd
        if vars.collateral_after_decrease ==0{ return false;}

        vars.liquidation_threshold_after_decrease = vars.total_collateral_in_usd
        * vars.avg_liquidation_threshold
        - (vars.amount_to_decrease_in_usd * vars.liquidation_threshold)
        / vars.collateral_after_decrease;

        vars.health_factor_after_decrease = calculate_health_factor_from_balance(
            vars.collateral_after_decrease,
            vars.total_debt_in_usd,
            vars.liquidation_threshold_after_decrease
        );

       vars.health_factor_after_decrease >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
    }

    struct MintToTreasuryData {
        current_stable_debt: u128,
        principal_stable: u128,
        previous_stable_debt: u128,
        current_variable_debt: u128,
        previous_variable_debt: u128,
        avg_stable_rate: u128,
        cumulated_stable_interest: u128,
        total_debt_accrued: u128,
        amount_to_mint: u128,
        reserve_factor: u128,
        stable_supply_updated_timestamp: u64,
    }

    fn mint_to_treasury(
        vars:&mut MintToTreasuryData, 
        scaled_variable_debt:u128, 
        previous_variable_borrow_index:u128,
        new_liquidity_index:u128,
        new_variable_borrow_index:u128,
        timestamp:u64
    ){
        //vars.reserve_factor = get_reserve_factor();
        if vars.reserve_factor ==0{return;}
        // (vars.principal_borrow_balance, vars.current_stable_debt, vars.avg_stable_rate. vars.stable_supply_updated_timestamp) = 
        // get_supply_data()
        vars.previous_variable_debt = scaled_variable_debt * previous_variable_borrow_index;
        vars.current_variable_debt = scaled_variable_debt * new_liquidity_index;

        vars.cumulated_stable_interest = calculate_compounded_interest(&vars.avg_stable_rate, vars.stable_supply_updated_timestamp);
        vars.previous_stable_debt = vars.principal_stable * vars.cumulated_stable_interest;
        vars.total_debt_accrued = vars.current_variable_debt
            + vars.current_stable_debt
            - vars.previous_variable_debt
            - vars.previous_stable_debt;
        vars.amount_to_mint = vars.total_debt_accrued * vars.reserve_factor;
        if vars.amount_to_mint != 0{
            //用mint到treasury这个账户上amount_to_mint
        }
    }

    struct UpdateInterestRateData {
        stable_debt_token: AccountId,
        available_liqudity: u128,
        total_stable_debt: u128,
        new_liquidity_rate: u128,
        new_stable_rate: u128,
        new_variable_rate: u128,
        avg_stable_rate: u128,
        total_variable_debt: u128, 
    }

    fn update_interest_rates(
        vars:&mut UpdateInterestRateData,
        reserve_address: AccountId,
        s_token: AccountId,
        liquidity_added: u128,
        liquidity_token: u128
    ){
        //用mapping
        //(vars.total_stable_debt, vars.avg_stable_rate) = get_total_supply_and_avgrate();
        //vars.total_variable_debt = variable.scaled_total_supply() * variable_borrow_index
        //(vars.new_liquidity_rate, vars.new_stable_rate, vars.new_variable_rate) = calculate_interest_rates()
        //validate 三个数不overflow
        //同步reserve的三个参数跟这里的update一样
        //event
    }

    fn validate_transfer(from:AccountId, reserve:ReserveData){
        // 用(, , , , health_factor) = calculate_user_account_data(xxxx)
        // require health_factor >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
    }

    fn validate_liquidation_call(
        collateral_reserve:ReserveData, 
        principal_reserve:ReserveData,
        user_config:UserData,
        user_health_factor: u128,
        user_stable_debt: u128,
        user_variable_debt: u128,
    ) -> bool{
        //make sure both collateral_reserve and principal_reserve are active, otherwise return false
        if user_health_factor >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD{ return false;}
        // let is_collateral_enabled:bool = get_liquidation_threshold()>0 && is_using_as_collateral();
        let is_collateral_enabled:bool = false;
        if !is_collateral_enabled {return false;}
        if user_stable_debt == 0 && user_variable_debt==0 {return false;}
        true
    }


    fn validate_set_use_reserve_as_collateral(){
       // make sure 就是原生币的atoken的数量 > 0
       //make sure use_as_collateral && balance_decrease_allowed()
    }

    fn validate_rebalance_stable_borrow_rate(reserve:ReserveData, token:AccountId, s_token:AccountId){
        // let is_active = get_flags();
        // make sure is_active is true oterwise return error
        // let total_debt = stable_debt + variable_debt
        // available_liqudity = token里的平台stoken数量
        // if total_debt ==0{
        //     let usage_ratio = 0;
        // } else {
        //     let usage_ratio = total_debt/(available_liqudity+total_debt);
        // }

        // get current_liquidity_rate
        // max_variable_borrow_rate = get_max_variable_borrow_rate()

        // make sure
        // 1. usage_ratio > = REBALANCE_UP_USAGE_RATIO_THRESHOLD
        // 2.current_liquidity_rate <= max_variable_borrow_rate * REBALANCE_UP_USAGE_RATIO_THRESHOLD

    }
    fn validate_swap_rate_mode(
        reserve:ReserveData, 
        user_config:UserData,
        stable_debt:u128,
        variable_debt:u128,
        current_rate_mode: InterestRateMode
    ){
        // if current_rate_mode == InterestRateMode.STABLE{ make sure stable_debt >0}
        // else if current_rate_mode == InterestRateMode.VARIABLE{make sure variable>0}

        // !userConfig.isUsingAsCollateral(reserve.id) ||
        // reserve.configuration.getLtv() == 0 ||
        // total_debt > s_token_balance
    }
    fn validate_repay(
        reserve:ReserveData,
        amount_sent:u128,
        rate_mode:InterestRateMode,
        on_behalf_of:AccountId,
        stable_debt:u128,
        variable_debt:u128
    ) {
        //if amount_sent < 0 { return Error}

        //make sure:
        // stable_debt >0 && rate_mode == InterestRateMode.STABLE ||
        // variable_debt >0 && rate_mode == InterestRateMode.VARIABLE

        //if self.env().caller() != on_behalf_of { return Error}
    }

    struct ValidateBorrowData {
        current_ltv:u128,
        current_liquidation_threshold:u128,
        collateral_needed_in_usd:u128,
        user_collateral_balance_in_usd:u128,
        user_borrow_balance_in_usd:u128,
        available_liqudity:u128,
        health_factor:u128   
    }

    fn validate_borrow(
        vars:ValidateBorrowData,
        asset:AccountId,
        reserve:ReserveData,
        user_address:AccountId,
        amount:u128,
        amount_in_usd:u128,
        interest_rate_mode:InterestRateMode,
        max_stable_loan_percent:u128
    ) {
        //if amount == 0{ return Error}
        
        // if interest_rate_mode != core::InterestRateMode.STABLE || interest_rate_mode != core::InterestRateMode.VARIABLE {
        //     return Error
        // }

    //     (
    //     vars.user_collateral_balance_in_usd, 
    //     vars.user_borrow_balance_in_usd, 
    //     vars.current_ltv, 
    //     vars.current_liquidation_threshold,
    //     vars.health_factor
    // ) = calculate_user_account_data(xxxx)

        // make sure
        // vars.user_collateral_balance_in_usd > 0
        // vars.health_factor > HEALTH_FACTOR_LIQUIDATION_THRESHOLD

        // vars.collateral_needed_in_usd = vars.user_borrow_balance_in_usd + amount_in_usd /vars.current_ltv
        // make sure vars.collateral_needed_in_usd <= vars.user_collateral_balance_in_usd

        // if interest_rate_mode == core::InterestRateMode.STABLE {
        //     make sure get_ltv ==0 || amount>user_balance

        //     get vars.available_liqudity
        //     let max_loan_size_stable = vars.available_liqudity * max_stable_loan_percent
        //     amount <= max_loan_size_stable
        // }
    }
    fn validate_withdraw(
        asset:AccountId,
        amount:u128,
        user_balance:u128,
        reserve:ReserveData
    ){
        // make sure  amount != 0 && amount <= user_balance
        // make sure the reserve is active
        // then makw sure balance_decrease_allowed()
    }
    fn validate_deposit(reserve:ReserveData, amount:u128){
        // make sure amount != 0
        // is_active
    }

}