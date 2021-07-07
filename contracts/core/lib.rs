#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod core {
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use ink_storage::collections::HashMap;
    pub const ONE_YEAR:u128 = 365; //需要加一些0
    pub const REBALANCE_UP_USAGE_RATIO_THRESHOLD:u128 = 95; //需加单位
    //HashMap要用到PackedLayout
    pub struct UserConfig {
        user_config: HashMap<AccountId, UserData>,
    }

    #[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub struct UserData {
        principal_borrow_balance: u128,
        last_borrow_cumulative_index: u128, 
        origination_fee: u128,
        borrow_rate: u128,
        last_update_timestamp: u64,
        //use_as_collateral: bool,
    }
    #[ink(storage)]
    pub struct ReserveConfigurationData {
        ltv: u128,
        liquidity_threshold: u128,
        liquidity_bonus: u128,
        decimals: u128,
        reserve_factor: u128,
    }

    impl ReserveConfigurationData{
        #[ink(constructor)]
        pub fn new(ltv:u128, liquidity_threshold:u128, liquidity_bonus: u128, reserve_factor:u128) -> Self{
            Self{
                ltv: ltv,
                liquidity_threshold: liquidity_threshold,
                liquidity_bonus: liquidity_bonus,
                decimals: 12,
                reserve_factor: reserve_factor,
            }
        }
        #[ink(message)]
        pub fn get_params(&self) -> (u128, u128, u128, u128, u128){
            return (self.ltv, self.liquidity_threshold, self.liquidity_bonus, self.decimals, self.reserve_factor)
        }
    }


    pub struct ReserveData {
        liquidity_index: u128,//last_liquidity_cumulatvie_index
        variable_borrow_index: u128,//last_variable_borrow_cumulative_index
        current_liquidity_rate: u128,
        //current_variable_borrow_rate: u128,
        current_stable_borrow_rate: u128,
        last_update_timestamp: u64, 
        borrowing_enabled: bool,
    }

    // impl ReserveData {
    //     #[ink(constructor)]
    //     pub fn new() -> Self{
    //         unimplemented!()
    //     }
    //     #[ink(message)]//后面要全局注意修改&
    //     pub fn get_normalized_income(&self) -> u128{
    //         let timestamp = self.last_update_timestamp; 
    //         if timestamp == Self::env().block_timestamp() {
    //             return self.liquidity_index
    //         }
    //         // to-do, replace *，注意引用可以这样的
    //         let cumulated:u128 = self.caculate_linear_interest() * &self.liquidity_index;
    //         cumulated
    //     }

    //    fn caculate_linear_interest(&self) -> u128{
    //         let time_difference = Self::env().block_timestamp() - &self.last_update_timestamp;
    //         //let interest:u128 = &self.current_liquidity_rate * time_difference.into() / ONE_YEAR + 1; 
    //         let interest:u128 = 0;
    //         interest
    //     }

    //     #[ink(message)]
    //     pub fn get_normalized_debt(&self) -> u128{
    //         let timestamp = self.last_update_timestamp; 
    //         if timestamp == Self::env().block_timestamp() {
    //             return self.variable_borrow_index
    //         }
    //         let cumulated:u128 = calculate_compounded_interest(&self.variable_borrow_index, timestamp) * &self.variable_borrow_index;
    //         cumulated
    //     }

    //     #[ink(message)]
    //     pub fn update_indexes(&self,scaled_variable_debt:u128, timestamp:u64) -> (u128, u128){
    //         let mut new_liquidity_index = self.liquidity_index;
    //         let mut new_variable_borrow_index = self.variable_borrow_index;

    //         if self.current_liquidity_rate > 0 {
    //             let cumulated_liquidity_interest = self.caculate_linear_interest();
    //             new_liquidity_index *= cumulated_liquidity_interest;
    //             //todo 不要让new_liquidity_index overflow
    //             // todo self.liquidity_index = new_liquidity_index
    //             if scaled_variable_debt ==! 0 {
    //                 let cumulated_variable_borrow_interest = calculate_compounded_interest(&self.current_liquidity_rate, timestamp);
    //                 new_variable_borrow_index *= cumulated_variable_borrow_interest;
    //                 //todo 不要overflow
    //                 //todo self.variable_borrow_index = new_variable_borrow_index
    //             }
    //         }
    //         //todo  self.last_update_timestamp = Self::env().block_timestamp();
    //         (new_liquidity_index, new_variable_borrow_index)
    //     }

    //     //admin allowed only, call it when it comes to system issues
    //     fn set_borrowing_enabled(&self, enabled: bool) {
    //         // todo self.borrowing_enabled : enabled
    //     }

    //     fn update_state(&self){
    //         // get scaled_variable_debt
    //         // get previous_variable_borrow_index
    //         // get previous_liquidity_index
    //         // last_updated_timestamp
    //         // call update_indexes
    //         // call mint_to_treasury
    //     }

    
    // } 

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
//checked
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
//参数要另加，被引用时怎么办？
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
        //这里记得要改
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

    //参数要另加
    // fn validate_transfer(from:AccountId, reserve:ReserveData){
    //     // 用(, , , , health_factor) = calculate_user_account_data(xxxx)
    //     // require health_factor >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
    // }

    fn validate_liquidation_call(
        //collateral_reserve:ReserveData, 
        //principal_reserve:ReserveData,
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

    //另加参数
    fn validate_set_use_reserve_as_collateral(){
       // make sure 就是原生币的atoken的数量 > 0
       //make sure use_as_collateral && balance_decrease_allowed()
    }
//改返回的方式？
    fn validate_rebalance_stable_borrow_rate(
        //reserve:ReserveData, 
        token:AccountId, s_token:AccountId){
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
   
    fn validate_repay(
        reserve:ReserveData,
        amount_sent:u128,
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

    //另加参数！
    fn validate_borrow(
        vars:ValidateBorrowData,
        asset:AccountId,
        //reserve:ReserveData,
        user_address:AccountId,
        amount:u128,
        amount_in_usd:u128,
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
      //另加参数！
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


    fn _transfer(from:AccountId, to:AccountId, token:AccountId, amount:u128, validate:bool){
        //用mapping到特定reserve, token用在这里
        // let index = reserve.get_normalized_income();
        // let from_balance_before = balanceOf(from) * index;
        // let to_balance_before = balanceOf(to) * index;
        // erc20._transfer(from, to, amount/index);
        // if validate {
        //     self.finalize_transfer(token, from, to, amount, from_balance_before, to_balance_before)
        // }
        // event
    }

    fn finalize_transfer(token:AccountId, from:AccountId, to:AccountId, amount:u128,from_balance_before:u128, to_balance_before:u128){
        //Only callable by the overlying aToken of the `asset`
        //core.validate_transfer()
        // if from != to {
        //     if from_balance_before - amount ==0 {
        //         set_using_as_collateral(reserveid, false)
        //     }
        // }
        // if to_balance_before ==0 & amount != 0 {
        //     set_using_as_collateral(reserveid, true)
        // }
    }

    fn get_reserve_data(asset:AccountId){}
    fn get_reserve_configuration(asset:AccountId){}
    fn get_reserve_normalized_income(asset:AccountId){}
    fn get_reserve_normalized_debt(asset:AccountId){}
    fn get_max_borrow_size_percent(){}
    fn set_reserve_configuratio(){}

    const LIQUIDATION_CLOSE_FACTOR_PERCENT:u128 = 5 * 10_u128.saturating_pow(11); //50%
    pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 =1 * 10_u128.saturating_pow(12);

    struct AvailableCollateralToLiquidateData {
        user_compouned_borrow_balance:u128,
        liquidation_bonus:u128,
        collateral_price:u128,
        debt_asset_price:u128,
        max_amount_collateral_to_liquidate:u128,
        debt_asset_decimals:u128,
        collateral_decimal:u128,
    }

    pub struct LiquidationCallData {
        user_collateral_balance: u128,
        user_debt:u128,
        max_liquidatable_debt:u128,
        actual_debt_to_liquidate:u128,
        liquidation_ratio:u128,
        max_collateral_to_liquidate:u128,
        debt_amount_needed:u128,
        health_factor:u128,
        liquidator_previous_s_token_balance:u128,
        collateral_s_token: AccountId,
    }

    pub fn liquidation_call(vars:&LiquidationCallData, u:UserConfig, r:ReserveConfigurationData, collateral:AccountId, borrower:AccountId, debt_to_cover:u128, receive_s_token:bool){
        let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
        let debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.stable_debt_token_address);
        let (_, _liquidation_threshold, _, _, _) =  = ReserveConfigurationData::get_params(&r);
        vars.health_factor = calculate_health_factor_from_balance(stoken, debttoken, _liquidation_threshold);




   
    }

    fn transfer_on_liquidation(from:AccountId, to:AccountId, value:u128){
    }


    fn _caculate_available_collateral_to_liquidate(collateral:AccountId, debt_asset:AccountId, amoun_to_cover:u128, user_collateral_balance:u128) -> (u128, u128){
        let collateral_amount = 0;
        let debt_amount_needed = 0;
        //get collateral token price
        //get debt token price
        //看怎样用mapping加上对应的token,
        //get collatral_decimal, liquidation_bonus
        //get debt_asset_decimals
        //max_collateral_to_liquidate = debt_asset_price * debt_to_cover * 10**collateral_decimal
        //          * liquidation_bonus/ (collateral_price * 10**debt_asset_decimals);
        //if max_amount_collateral_to_liquidate > user_collateral_balance {
        //     collateral_amount = user_collateral_balance;
        //     debt_amount_needed = collateral_price 
        //         * collateral_amount 
        //         * 10**debt_asset_decimals
        //         / (debt_asset_price * 10**collateral_decimal)
        //         / liquidation_bonus
        // } else {
        //     collateral_amount = max_amount_collateral_to_liquidate;
        //     debt_amount_needed = debt_to_cover;
        // }

        (collateral_amount, debt_amount_needed)
    }
}