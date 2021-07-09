#[allow(unused)]
mod errors;

pub use errors::*;
use ink_prelude::string::String;
use ink_env::AccountId;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use ink_env::call::FromAccountId;
use ierc20::IERC20;

pub const REBALANCE_UP_USAGE_RATIO_THRESHOLD:u128 = 95; //需加单位
const LIQUIDATION_CLOSE_FACTOR_PERCENT:u128 = 5 * 10_u128.saturating_pow(11); //50%
pub const HEALTH_FACTOR_LIQUIDATION_THRESHOLD:u128 =1 * 10_u128.saturating_pow(12);

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct ReserveData {
    pub stable_liquidity_rate: u128,//要删！
    pub liquidity_rate:u128,//这个不再是利息，而是流动率！
    pub stable_borrow_rate: u128,
    pub stoken_address: AccountId,
    pub stable_debt_token_address: AccountId,

    pub ltv: u128,
    pub liquidity_threshold: u128,
    pub liquidity_bonus: u128,
    pub decimals: u128,
    pub reserve_factor: u128,
    pub liquidity_index: u128,//代替利息的存在！
    pub variable_borrow_index: u128,
    pub last_updated_timestamp: u64,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserReserveData {
    pub cumulated_liquidity_interest: u128,
    pub cumulated_stable_borrow_interest: u128,
    pub last_update_timestamp: u64,
    pub borrow_balance: u128,

    last_borrow_cumulative_index: u128, 
    health_factor:u128,
    //borrow_rate:u128,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std",derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct UserKycData {
    pub name: String,
    pub email: String,
}
//done
pub fn get_params(vars: &ReserveData) -> (u128, u128, u128, u128, u128){
    return (vars.ltv, vars.liquidity_threshold, vars.liquidity_bonus, vars.decimals, vars.reserve_factor)
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
//ting
pub fn mint_to_treasury(scaled_debt:u128, vars: &ReserveData){
   let amount_to_minted = scaled_debt * vars.reserve_factor;
    if amount_to_minted != 0 {
        //mint to treasury account
    } 
}
//ting
fn balance_decrease_allowed(vars:&mut ReserveData, user:AccountId, amount:u128, user_config:UserReserveData) -> bool{
    let debttoken: IERC20 =  FromAccountId::from_account_id(vars.stable_debt_token_address);
    let mut stoken: IERC20 = FromAccountId::from_account_id(vars.stoken_address);

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

//david 记得每个function下考虑要不要event,考虑这个function 的存在性！
fn update_interest_rates(vars:&mut ReserveData, liquidity_added: u128,liquidity_token: u128){
    let debttoken: IERC20 =  FromAccountId::from_account_id(vars.stable_debt_token_address);
    //池的总数而不是个人！不过要验证是否正确用法！
    let total_debttoken = debttoken.total_supply() * vars.variable_borrow_index;
    
    //let (new_liquidity_rate, new_borrow_rate) = calculate_interest_rates(liquidity_added, liquidity_taken, total_debttoken, vars.reserve_factor);
    //确保new_liquidity_rate, new_borrow_rate没overflow
    let (new_liquidity_rate, new_borrow_rate) = (0,0);

    vars.liquidity_rate = new_liquidity_rate;
    vars.stable_borrow_rate = new_stable_borrow_rate;
}

//ting
fn validate_liquidation_call(
    user_config:UserReserveData,
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

//改返回的方式？
fn validate_rebalance_stable_borrow_rate(
    //reserve:ReserveData, 
    token:AccountId, s_token:AccountId){
   
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

//另加参数！ting
fn validate_borrow(
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

}
  //另加参数！ting
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
//ting
fn validate_deposit(reserve:ReserveData, amount:u128){
    // make sure amount != 0
    // is_active
}

//ting
fn _transfer(from:AccountId, to:AccountId, token:AccountId, amount:u128, validate:bool){
  
}
//ting
fn finalize_transfer(token:AccountId, from:AccountId, to:AccountId, amount:u128,from_balance_before:u128, to_balance_before:u128){
}
//要不要限定每个用户单次可借多少？？
fn get_max_borrow_size_percent(){}
fn set_reserve_configuratio(){}

pub fn liquidation_call(r:ReserveData, collateral:AccountId, borrower:AccountId, debt_to_cover:u128, receive_s_token:bool){
    // let stoken: IERC20 = FromAccountId::from_account_id(self.reserve.stoken_address);
    // let debttoken: IERC20 = FromAccountId::from_account_id(self.reserve.stable_debt_token_address);
    // let (_, _liquidation_threshold, _, _, _) =  = ReserveConfigurationData::get_params(&r);
    // vars.health_factor = calculate_health_factor_from_balance(stoken, debttoken, _liquidation_threshold)

}

fn transfer_on_liquidation(from:AccountId, to:AccountId, value:u128){}

fn _caculate_available_collateral_to_liquidate(collateral:AccountId, debt_asset:AccountId, amoun_to_cover:u128, user_collateral_balance:u128) -> (u128, u128){
    let collateral_amount = 0;
    let debt_amount_needed = 0;

    (collateral_amount, debt_amount_needed)
}