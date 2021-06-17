#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod core {
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    pub const ONE_YEAR:u128 = 365; 

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
        use_as_collateral: bool,
    }
    
    #[ink(storage)]
    pub struct ReserveData {
        liquidity_index: u128,//last_liquidity_cumulatvie_index 
        current_liquidity_rate: u128,//same
        total_borrows_stable: u128,
        total_borrows_variable: u128,
        current_variable_borrow_rate: u128,//same
        current_stable_borrow_rate: u128,//same
        current_average_stable_borrow_rate: u128,
        last_variable_borrow_cumulative_index: u128,
        //variableBorrowIndex
        base_ltv_as_collateral: u128,
        liquidation_threshold: u128,
        liquidation_bonus: u128,
        decimals: u128,
        s_token_address: AccountId,
        //stabledebt_token_address: AccountId,
        //variabledebt_token_address: AccountId,
        //id: u8,
        last_update_timestamp: u64, //same
        borrowing_enabled: bool,
        usage_as_collateral_enabled: bool,
        is_stable_borrow_rate_enabled: bool,
        is_active: bool, 
        is_freezed: bool,
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
            // to-do, replace *
            let cumulated:u128 = self.calcuate_linear_interest() * &self.liquidity_index;
            cumulated
        }

        #[ink(message)]
        pub fn calcuate_linear_interest(&self) -> u128{
            let time_difference:u64 = Self::env().block_timestamp() - &self.last_update_timestamp;
            //let interest:u128 = &self.current_liquidity_rate * time_difference.into() / ONE_YEAR + 1; 
            let interest:u128 = 0;
            interest
        }
    } 

}
