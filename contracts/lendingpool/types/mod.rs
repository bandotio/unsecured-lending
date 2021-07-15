#[allow(unused)]
mod errors;

pub use errors::*;
use ink_prelude::string::String;
use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use ink_env::call::FromAccountId;
use ierc20::IERC20;
pub const ONE: u128 = 1_000_000_000_000;
pub const ONE_YEAR:u128 = 365*24*60*60*1000;
pub const LIQUIDATION_CLOSE_FACTOR_PERCENT:u128 = 5 * 10_u128.saturating_pow(11); //50%
pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 = ONE;
/// The representation of the number one as a precise number as 10^12
pub const BASE_LIQUIDITY_RATE: u128 = ONE / 100 * 10; // 10% 
pub const BASE_BORROW_RATE: u128 = ONE / 100 * 18; // 18%
pub const BASE_LIQUIDITY_INDEX: u128 = ONE; // 1

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

impl ReserveData {
    pub fn new(
        stoken_address: AccountId,
        debt_token_address: AccountId,
        ltv: u128,
        liquidity_threshold: u128,
        liquidity_bonus: u128,
        reserve_factor: u128,
    ) -> ReserveData {
        ReserveData {
            liquidity_rate: BASE_LIQUIDITY_RATE,
            borrow_rate: BASE_BORROW_RATE,
            stoken_address: stoken_address,
            debt_token_address: debt_token_address,
            ltv: ltv,
            liquidity_threshold: liquidity_threshold,
            liquidity_bonus: liquidity_bonus,
            decimals: 12,
            reserve_factor: reserve_factor,
            liquidity_index: BASE_LIQUIDITY_INDEX,
            last_updated_timestamp: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct InterestRateData {
    pub optimal_utilization_rate:u128,
    pub excess_utilization_rate:u128,
    pub rate_slope1: u128,
    pub rate_slope2:u128,
    pub utilization_rate: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserReserveData {
    pub cumulated_liquidity_interest: u128,
    pub cumulated_borrow_interest: u128,
    pub last_update_timestamp: u64,
    pub borrow_balance: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserKycData {
    pub name: String,
    pub email: String,
}

//done
pub fn calculate_health_factor_from_balance(total_collateral_in_usd:u128, total_debt_in_usd:u128, liquidation_threshold:u128) -> u128{
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

//大家可思考下！
pub fn balance_decrease_allowed(vars:&mut ReserveData, user:AccountId, amount:u128) -> bool{
    let debttoken: IERC20 =  FromAccountId::from_account_id(vars.debt_token_address);
    let stoken: IERC20 = FromAccountId::from_account_id(vars.stoken_address);
    if debttoken.balance_of(user) == 0 {return true;}
    if vars.liquidity_threshold == 0 {return true;}
    //david dot 的单价
    //let unit_price = self.env().extension().fetch_price();
    let unit_price = 16;//小数点！
    //这里就该要表示dot的数量！这个公式是stoken是己有index的功能的情况下成立，不然要加个index!!!
    let _total_collateral_in_usd = unit_price * stoken.balance_of(user);

    let amount_to_decrease_in_usd = unit_price * amount;
    let collateral_balance_after_decrease_in_usd = _total_collateral_in_usd - amount_to_decrease_in_usd;
    //这里就是healthfactor的功能，只是余额到少要大于0
    if collateral_balance_after_decrease_in_usd == 0 {return false;}
    //大家看下这个公式在单币下make sense??
    let liquidity_threshold_after_decrease = 
    _total_collateral_in_usd * vars.liquidity_threshold - (amount_to_decrease_in_usd*vars.liquidity_threshold)/collateral_balance_after_decrease_in_usd;
    //需要确保参数的正确性！！比如这里debttoken下估计需要加上用户要付的利息？！！
    let health_factor_after_decrease = calculate_health_factor_from_balance(
        collateral_balance_after_decrease_in_usd,
        debttoken.balance_of(user),
        liquidity_threshold_after_decrease
    );
    health_factor_after_decrease >= HEALTH_FACTOR_LIQUIDATION_THRESHOLD
}

//大家可思考下！
pub fn caculate_available_collateral_to_liquidate(vars:&ReserveData, debt_to_cover:u128, user_collateral_balance:u128) -> (u128, u128){
    let mut collateral_amount = 0;
    let mut debt_amount_needed = 0;
    //let unit_price = self.env().extension().fetch_price();
    let dot_unit_price = 16;//小数点！
    let debt_asset_price = 1; //这个不是debttoken吗？为什么需要价格？？和index有关？todo
    //这个算式有待验证！
    let max_amount_collateral_to_liquidate = debt_asset_price * debt_to_cover * vars.liquidity_bonus / dot_unit_price;
    if max_amount_collateral_to_liquidate > user_collateral_balance {
        collateral_amount = user_collateral_balance;
        debt_amount_needed = dot_unit_price * collateral_amount / debt_asset_price / vars.liquidity_bonus;
    } else {
        collateral_amount = max_amount_collateral_to_liquidate;
        debt_amount_needed = debt_to_cover;
    }
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
        current_liquidity_rate = borrow_rate  * utilization_rate * (ONE-reserve_factor);
    }
    vars.utilization_rate = utilization_rate;
    (current_liquidity_rate, current_borrow_rate)
}