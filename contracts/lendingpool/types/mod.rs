#[allow(unused)]
mod errors;

pub use errors::*;
use ink_prelude::string::String;
use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use ink_env::call::FromAccountId;
use ierc20::IERC20;
use price::Price;

/// The representation of the number one as a precise number as 10^12
pub const ONE: u128 = 1_000_000_000_000;
pub const ONE_PERCENTAGE: u128 = 10_000_000_000;

pub const ONE_YEAR:u128 = 365*24*60*60*1000;
pub const LIQUIDATION_CLOSE_FACTOR_PERCENT: u128 = 50 * ONE_PERCENTAGE; //50%
pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD: u128 = ONE;

pub const BASE_LIQUIDITY_RATE: u128 = 10 * ONE_PERCENTAGE; // 10% 
pub const BASE_BORROW_RATE: u128 = 18 * ONE_PERCENTAGE; // 18%
pub const BASE_LIQUIDITY_INDEX: u128 = ONE; // 1
pub const BASE_BORROW_INDEX: u128 = ONE; // 1

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct ReserveData {
    pub liquidity_rate:u128,
    pub borrow_rate: u128,
    pub stoken_address: AccountId,
    pub debt_token_address: AccountId,
    pub oracle_price_address: AccountId,
    pub ltv: u128,
    pub liquidity_threshold: u128,
    pub liquidity_bonus: u128,
    pub decimals: u128,
    pub liquidity_index: u128,
    pub last_updated_timestamp: u64,
    pub borrow_index: u128,
}

impl ReserveData {
    pub fn new(
        stoken_address: AccountId,
        debt_token_address: AccountId,
        oracle_price_address: AccountId,
        ltv: u128,
        liquidity_threshold: u128,
        liquidity_bonus: u128,
    ) -> ReserveData {
        ReserveData {
            liquidity_rate: BASE_LIQUIDITY_RATE,
            borrow_rate: BASE_BORROW_RATE,
            stoken_address: stoken_address,
            debt_token_address: debt_token_address,
            oracle_price_address: oracle_price_address,
            ltv: ltv * ONE_PERCENTAGE,
            liquidity_threshold: liquidity_threshold * ONE_PERCENTAGE,
            liquidity_bonus: liquidity_bonus * ONE_PERCENTAGE,
            decimals: 12,
            liquidity_index: BASE_LIQUIDITY_INDEX,
            borrow_index:BASE_BORROW_INDEX,
            last_updated_timestamp: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct InterestRateData {
    pub optimal_utilization_rate: u128,
    pub excess_utilization_rate: u128,
    pub rate_slope1: u128,
    pub rate_slope2: u128,
    pub utilization_rate: u128,
}

impl InterestRateData {
    pub fn new(
        optimal_utilization: u128,
        slope1: u128,
        slope2: u128,
    ) -> InterestRateData {
        InterestRateData {
            optimal_utilization_rate: optimal_utilization * ONE_PERCENTAGE,
            excess_utilization_rate: ONE -  optimal_utilization * ONE_PERCENTAGE,
            rate_slope1: slope1 * ONE_PERCENTAGE,
            rate_slope2: slope2 * ONE_PERCENTAGE,
            utilization_rate: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserReserveData {
    pub cumulated_liquidity_interest: u128,
    pub cumulated_borrow_interest: u128,
    pub last_update_timestamp: u64,
    //pub borrow_balance: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserKycData {
    pub name: String,
    pub email: String,
}


pub fn calculate_health_factor_from_balance(total_collateral_in_usd:u128, total_debt_in_usd:u128, liquidation_threshold:u128) -> u128{
    if total_debt_in_usd == 0 { return 0};
    let result = total_collateral_in_usd * liquidation_threshold / total_debt_in_usd;
    result
}

// fn calculate_available_borrows_in_usd(total_collateral_in_usd:u128, total_debt_in_usd:u128, ltv:u128) -> u128{
//     let mut available_borrows_in_usd = total_collateral_in_usd * ltv;
//     if available_borrows_in_usd < total_debt_in_usd { return 0};
//     available_borrows_in_usd -= total_debt_in_usd;
//     available_borrows_in_usd
// } 

/**
    * @dev Checks if a specific balance decrease is allowed
    * (i.e. doesn't bring the user borrow position health factor under HEALTH_FACTOR_LIQUIDATION_THRESHOLD)
    * @param reservesData The data of all the reserves
    * @param user The address of the user
    * @param amount The amount to decrease
    * @return true if the decrease of the balance is allowed
**/
pub fn balance_decrease_allowed(vars:&mut ReserveData, user:AccountId, amount:u128) -> bool{
    let debttoken: IERC20 =  FromAccountId::from_account_id(vars.debt_token_address);
    let stoken: IERC20 = FromAccountId::from_account_id(vars.stoken_address);
    if debttoken.balance_of(user) == 0 {return true;}
    if vars.liquidity_threshold == 0 {return true;}
    let mut oracle: Price = FromAccountId::from_account_id(vars.oracle_price_address);
    oracle.update().expect("Failed to update price");
    let unit_price = oracle.get();
    let _total_collateral_in_usd = unit_price * stoken.balance_of(user);

    let _total_debt_in_usd = unit_price * debttoken.balance_of(user);
    let amount_to_decrease_in_usd = unit_price * amount;
    let collateral_balance_after_decrease_in_usd = _total_collateral_in_usd - amount_to_decrease_in_usd;

    if collateral_balance_after_decrease_in_usd == 0 {return false;}

    let liquidity_threshold_after_decrease = 
    _total_collateral_in_usd * vars.liquidity_threshold - (amount_to_decrease_in_usd*vars.liquidity_threshold)/collateral_balance_after_decrease_in_usd;
    let health_factor_after_decrease = calculate_health_factor_from_balance(
        collateral_balance_after_decrease_in_usd,
        _total_debt_in_usd,
        liquidity_threshold_after_decrease
    );
    health_factor_after_decrease >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
}

//double check
/**
   * @dev Calculates how much of a specific collateral can be liquidated, given
   * a certain amount of debt asset.
   * - This function needs to be called after all the checks to validate the liquidation have been performed,
   *   otherwise it might fail.
   * @param vars The data of the collateral reserve
   * @param debt_to_cover The debt amount of borrowed `asset` the liquidator wants to cover
   * @param user_collateral_balance The collateral balance for DOT of the user being liquidated
   * @return collateral_amount: The maximum amount that is possible to liquidate given all the liquidation constraints
   *                           (user balance, close factor)
   *         debt_amount_needed: The amount to repay with the liquidation
**/
pub fn caculate_available_collateral_to_liquidate(vars:&ReserveData, debt_to_cover:u128, user_collateral_balance:u128) -> (u128, u128){
    let collateral_amount;
    let debt_amount_needed;
    let mut oracle: Price = FromAccountId::from_account_id(vars.oracle_price_address);
    oracle.update().expect("Failed to update price");
    let unit_price = oracle.get();
    let debt_asset_price = 1;

    let max_amount_collateral_to_liquidate = debt_asset_price * debt_to_cover * vars.liquidity_bonus / unit_price;
    if max_amount_collateral_to_liquidate > user_collateral_balance {
        collateral_amount = user_collateral_balance;
        debt_amount_needed = unit_price * user_collateral_balance / debt_asset_price / vars.liquidity_bonus;
    } else {
        collateral_amount = max_amount_collateral_to_liquidate;
        debt_amount_needed = debt_to_cover;
    }
    (collateral_amount, debt_amount_needed)
}


 /**
   * @dev Calculates the interest rates depending on the reserve's state and configurations
   * @param reserve The address of the reserve
   * @param vars The interest rate data
   * @param liquidity_added The liquidity added during the operation
   * @param liquidity_taken The liquidity taken during the operation
   * @param total_debt The total borrowed from the reserve
   * @param borrow_rate The borrow rate
   * @return The liquidity rate, the stable borrow rate and the variable borrow rate
**/
pub fn calculate_interest_rates(
    reserve:&ReserveData,
    vars:&InterestRateData,
    liquidity_added:u128,
    liquidity_taken:u128,
    total_debt:u128,
    borrow_rate:u128
) -> (u128, u128, u128) {
    let stoken: IERC20 = FromAccountId::from_account_id(reserve.stoken_address);
    let total_debt = total_debt/ONE;
    let _available_liqudity = stoken.total_supply()/ONE;
    let current_available_liqudity = _available_liqudity + liquidity_added - liquidity_taken;
    let current_borrow_rate;
    let mut current_liquidity_rate = reserve.liquidity_rate;
    let utilization_rate;
    if total_debt == 0 {
        utilization_rate = 0
    } else {
        utilization_rate = total_debt  * 100/ (current_available_liqudity + total_debt)
    }
    if utilization_rate > vars.optimal_utilization_rate{
        let excess_utilization_rate_ratio = utilization_rate - vars.optimal_utilization_rate / vars.excess_utilization_rate;
        current_borrow_rate = reserve.borrow_rate + vars.rate_slope1 + vars.rate_slope2 * excess_utilization_rate_ratio;
    } else {
        current_borrow_rate = reserve.borrow_rate + vars.rate_slope1 * (utilization_rate/ vars.optimal_utilization_rate);
    }
    if total_debt != 0 {
        
        current_liquidity_rate = (borrow_rate  * utilization_rate) /100;
    }
    else{
        current_liquidity_rate = 0;
    }
    (current_liquidity_rate, current_borrow_rate, utilization_rate)
}