#[allow(unused)]
mod errors;

pub use errors::*;
use ink_prelude::string::String;
use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};

pub const ONE_YEAR:u128 = 365; //需要加一些0
pub const REBALANCE_UP_USAGE_RATIO_THRESHOLD:u128 = 95; //需加单位


#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct ReserveData {
    pub stable_liquidity_rate: u128,
    pub stable_borrow_rate: u128,
    pub stoken_address: AccountId,
    pub stable_debt_token_address: AccountId,

    pub ltv: u128,
    pub liquidity_threshold: u128,
    pub liquidity_bonus: u128,
    pub decimals: u128,
    pub reserve_factor: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserReserveData {
    pub cumulated_liquidity_interest: u128,
    pub cumulated_stable_borrow_interest: u128,
    pub last_update_timestamp: u64,
    pub borrow_balance: u128,

    last_borrow_cumulative_index: u128, 
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserKycData {
    pub name: String,
    pub email: String,
}
//如何被前端调用？
pub fn get_params(vars: &ReserveData) -> (u128, u128, u128, u128, u128){
    return (vars.ltv, vars.liquidity_threshold, vars.liquidity_bonus, vars.decimals, vars.reserve_factor)
}