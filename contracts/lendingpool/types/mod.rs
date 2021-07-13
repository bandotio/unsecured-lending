#[allow(unused)]
mod errors;

pub use errors::*;
use ink_prelude::string::String;
use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use ink_env::call::FromAccountId;
use ierc20::IERC20;
pub const ONE_YEAR:u128 = 365*24*60*60*1000; //david SECONDS_PER_YEAR
pub const LIQUIDATION_CLOSE_FACTOR_PERCENT:u128 = 5 * 10_u128.saturating_pow(11); //50%
pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 =1 * 10_u128.saturating_pow(12);

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct ReserveData {
    pub liquidity_rate:u128,
    pub borrow_rate: u128,
    pub stoken_address: AccountId,
    pub debt_token_address: AccountId,

    pub ltv: u128,
    pub liquidity_threshold: u128,
    pub liquidity_bonus: u128,
    pub decimals: u128,
    pub reserve_factor: u128,
    pub liquidity_index: u128,
    pub last_updated_timestamp: u64,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct InterestRateData {
    pub optimal_utilization_rate:u128,
    pub excess_utilization_rate:u128,
    pub base_borrow_rate: u128,//重复?直接设0？
    pub rate_slope1: u128,
    pub rate_slope2:u128,
    pub utilization_rate: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserReserveData {
    //pub cumulated_liquidity_interest: u128,//要删？
    //pub cumulated_borrow_interest: u128,//要删？
    pub last_update_timestamp: u64,
    pub borrow_balance: u128,
    //origination_fee:u128,//平台收的费 //还不知要不要删 0.0025 * 1e18
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserKycData {
    pub name: String,
    pub email: String,
}

//done
fn calculate_health_factor_from_balance(total_collateral_in_usd:u128, total_debt_in_usd:u128, liquidation_threshold:u128) -> u128{
    if total_debt_in_usd == 0 { return 0};
    let result = total_collateral_in_usd * liquidation_threshold / total_debt_in_usd;
    result
}
//done
fn calculate_available_borrows_in_usd(total_collateral_in_usd:u128, total_debt_in_usd:u128, ltv:u128) -> u128{
    let mut available_borrows_in_usd = total_collateral_in_usd * ltv;
    if available_borrows_in_usd < total_debt_in_usd { return 0};
    available_borrows_in_usd -= total_debt_in_usd;
    available_borrows_in_usd
} 
//david
pub fn mint_to_treasury(scaled_debt:u128, vars: &ReserveData){
    //这里要加时间差形成的debt利息
   let amount_to_minted = scaled_debt * vars.reserve_factor;
    if amount_to_minted != 0 {
        //mint to treasury account
    } 
}
//ting
fn balance_decrease_allowed(vars:&mut ReserveData, user:AccountId, amount:u128, user_config:UserReserveData) -> bool{
    let debttoken: IERC20 =  FromAccountId::from_account_id(vars.debt_token_address);
    let stoken: IERC20 = FromAccountId::from_account_id(vars.stoken_address);

    if debttoken.balance_of(user) == 0 {return true;}
    if vars.liquidity_threshold == 0 {
        return true;
    }
    //oracle,这里要看是取dot的价格还是stoken和价格？净值和dot价格？？？？？
    //let unit_price = self.env().extension().fetch_price();
    let unit_price = 0;//小数点！
    //这里就该要表示dot的数量！
    let _total_collateral_in_usd = unit_price * stoken.balance_of(user);

    let amount_to_decrease_in_usd = unit_price * amount;
    let collateral_balance_after_decrease_in_usd = _total_collateral_in_usd - amount_to_decrease_in_usd;
    if collateral_balance_after_decrease_in_usd == 0 {return false;}
    //这个公式在单币下make sense??
    let liquidity_threshold_after_decrease = 
    _total_collateral_in_usd * vars.liquidity_threshold - (amount_to_decrease_in_usd*vars.liquidity_threshold)/collateral_balance_after_decrease_in_usd;
    //需要确保参数的正确性！！
    let health_factor_after_decrease = calculate_health_factor_from_balance(
        collateral_balance_after_decrease_in_usd,
        debttoken.balance_of(user),
        liquidity_threshold_after_decrease
    );
    health_factor_after_decrease >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
}


//ting
fn validate_liquidation_call(
    user_config:UserReserveData,
    user_health_factor: u128,
    user_debt: u128,
) -> bool{
    //make sure both collateral_reserve and principal_reserve are active, otherwise return false
    if user_health_factor >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD{ return false;}
    // let is_collateral_enabled:bool = get_liquidation_threshold()>0 && is_using_as_collateral();
    let is_collateral_enabled:bool = false;
    if !is_collateral_enabled {return false;}
    if user_debt == 0{return false;}
    true
}

//ting 把finalize_transfer加入
fn transfer_on_liquidation(from:AccountId, to:AccountId, value:u128){}
//ting
pub fn caculate_available_collateral_to_liquidate(collateral:AccountId, debt_asset:AccountId, amoun_to_cover:u128, user_collateral_balance:u128) -> (u128, u128){
    let collateral_amount = 0;
    let debt_amount_needed = 0;
    (collateral_amount, debt_amount_needed)
}
//以下算述需考虑精位还有算法顺序！因为算法的复杂性，还要考虑用不用后面！
pub fn calculate_interest_rates(
    reserve:&ReserveData,
    vars:&mut InterestRateData,
    liquidity_added:u128,
    liquidity_taken:u128,
    total_debt:u128,
    borrow_rate:u128,
    reserve_factor:u128,
) -> (u128, u128) {
    let stoken: IERC20 = FromAccountId::from_account_id(reserve.stoken_address);
    let _available_liqudity = stoken.total_supply();
    let current_available_liqudity = _available_liqudity + liquidity_added - liquidity_taken;

    let mut current_borrow_rate = 0;
    let mut current_liquidity_rate = 0;
    let mut utilization_rate = 0;

    if total_debt == 0 {
        utilization_rate = 0
    } else {
        utilization_rate = total_debt / (current_available_liqudity + total_debt)
    }

    if utilization_rate > vars.optimal_utilization_rate{
        let excess_utilization_rate_ratio = utilization_rate - vars.optimal_utilization_rate / vars.excess_utilization_rate;
        current_borrow_rate = reserve.borrow_rate + vars.rate_slope1 + vars.rate_slope2 * excess_utilization_rate_ratio;
    } else {
        current_borrow_rate = reserve.borrow_rate + vars.rate_slope1 * (utilization_rate/ vars.optimal_utilization_rate);
    }
    if total_debt != 0 {//这种算法不知对不对！
        current_liquidity_rate = borrow_rate  * utilization_rate * (1-reserve_factor);
    }
    vars.utilization_rate = utilization_rate;
    (current_liquidity_rate, current_borrow_rate)
}