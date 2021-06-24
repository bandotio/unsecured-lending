#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod collateral_manager {
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    pub const LIQUIDATION_CLOSE_FACTOR_PERCENT:u128 = 5000;
    pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 = 1; 

    #[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub struct AvailableCollateralToLiquidate {
        user_compouned_borrow_balance:u128,
        liquidation_bonus:u128,
        collateral_price:u128,
        debt_asset_price:u128,
        max_amount_collateral_to_liquidate:u128,
        debt_asset_decimals:u128,
        collateral_decimal:u128,
    }

    #[ink(storage)]
    pub struct LiquidationCall {
        user_collateral_balance: u128,
        user_stable_debt:u128,
        user_variable_debt:u128,
        max_liquidatable_debt:u128,
        actual_debt_to_liquidate:u128,
        liquidation_ratio:u128,
        max_collateral_to_liquidate:u128,
        debt_amount_needed:u128,
        health_factor:u128,
        liquidator_previous_s_token_balance:u128,
        collateral_s_token: AccountId,
        is_collateral_enabled: bool,
    }

    impl LiquidationCall {
        #[ink(constructor)]
        pub fn new() -> Self{
            unimplemented!()
        }

        #[ink(message)]
        pub fn liquidation_call(&self, collateral:AccountId, debt_asset:AccountId, borrower:AccountId, debt_to_cover:u128, receive_s_token:bool){
        //     mapping useraddress -> userdata
        //     calculate_user_account_data to get the health_factor
        //     get_user_current_debt
        //     validate_liquidation_call
        //     get collateral_s_token of the borrower to see the collateral amount 
        //     get max_collateral_to_liquidate = (user_stable_debt + user_variable_debt)/LIQUIDATION_CLOSE_FACTOR_PERCENT

        //     if debt_to_cover > max_liquidatable_debt {actual_debt_to_liquidate = max_liquidatable_debt}
        //     else {actual_debt_to_liquidate = debt_to_cover}

        //     get(max_collateral_to_liquidate, debt_amount_needed) = caculate_available_collateral_to_liquidate(actual_debt_to_liquidate, collateral_s_token)
        //     if debt_amount_needed < actual_debt_to_liquidate{actual_debt_to_liquidate = debt_amount_needed}

        //     if !receive_s_token {
        //         get current_available_collateral, make sure there is enough available liquidity in the collateral reserve             
        //         if current_available_collateral < max_collateral_to_liquidate{
        //             return error
        //         }
        //     }
        //     update_state: Updates the liquidity cumulative index and the variable borrow index

        //    if user_variable_debt >= actual_debt_to_liquidate {
        //        burn actual_debt_to_liquidate
        //    } else {
        //        if user_variable_debt > 0{
        //            burn user_variable_debt 
        //            burn (actual_debt_to_liquidate- variable) amount of stable_debt
        //        }
        //    }

        //    update_interest_rate: Updates the reserve current stable borrow rate, the current variable borrow rate and the current liquidity rate

        //    if receive_s_token {
        //        get liquidator_previous_s_token_balance
        //        transfer_on_liquidation(borrower, liquidator, max_collateral_to_liquidate) to get s_token from borrower to liquidator

        //        if liquidator_previous_s_token_balance == 0 {setUsingAsCollateral for liquidator}
        //        else {updateState & updateInterestRates}

        //        burn the equivalent amount of aToken, sending the underlying to the liquidator

        //        If the collateral being liquidated is equal to the user balance,we set the currency as not being used as collateral anymore
        //        Transfers the debt asset being repaid to the aToken, where the liquidity is kept
        
        }
    }


    fn transfer_on_liquidation(from:AccountId, to:AccountId, value:u128){
        //only pooladmin can do it
        // let token = s_token
        //self._transfer(from, to, token, value, false);
        //event
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