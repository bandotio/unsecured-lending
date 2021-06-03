#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod interest_rate {
    use ink_storage::traits::{PackedLayout, SpreadLayout};

    #[ink(storage)]
    pub struct InterestRate {
        //TODO which pool
        //provider: ILendingPoolAddressesProvider,
        optimal_utilization_rate: u128,
        base_variable_borrow_rate: u128,
        stable_rate_slope1: u128,
        stable_rate_slope2: u128,
    }

    #[derive(
        Debug, Default, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub struct InterestRatesLocalVars {
        pub total_debt: u128,
        pub current_stable_borrow_rate: u128,
        pub current_liquidity_rate: u128,
        pub utilization_rate: u128,
    }

    impl InterestRate {

        #[ink(constructor)]
        pub fn new(optimal_utilization_rate: u128,
            base_variable_borrow_rate: u128,
            stable_rate_slope1: u128,
            stable_rate_slope2: u128) -> Self {
            Self {
                optimal_utilization_rate,
                base_variable_borrow_rate,
                stable_rate_slope1,
                stable_rate_slope2,
            }
        }

        #[ink(message)]
        pub fn optimal_utilization_rate(&self) -> u128 {
            self.optimal_utilization_rate
        }

        #[ink(message)]
        pub fn base_variable_borrow_rate(&self) -> u128 {
            self.base_variable_borrow_rate
        }

        #[ink(message)]
        pub fn stable_rate_slope1(&self) -> u128 {
            self.stable_rate_slope1
        }

        #[ink(message)]
        pub fn stable_rate_slope2(&self) -> u128 {
            self.stable_rate_slope2
        }


        /**
        * @dev Calculates the interest rates depending on the reserve's state and configurations.
        * @param reserve The address of the reserve
        * @param available_liquidity The liquidity available in the corresponding sToken
        * @param total_stable_debt The total borrowed from the reserve a stable rate
        * @param reserve_factor The reserve portion of the interest that goes to the treasury of the market
        * @return The liquidity rate, the stable borrow rate
        **/
        #[ink(message)]
        pub fn calculate_interest_rates(&mut self,
            reserve: AccountId,
            available_liquidity: u128,
            total_stable_debt: u128,
            reserve_factor: u128) -> (u128, u128) {
            
            let mut vars = InterestRatesLocalVars {
                total_debt: total_stable_debt,
                current_stable_borrow_rate: 0,
                current_liquidity_rate: 0,
                utilization_rate: 0,
            };

            if vars.total_debt != 0 {
                vars.utilization_rate = vars.total_debt /(available_liquidity + vars.total_debt);
            }
            
            //TODO pool oracle
            //vars.current_stable_borrow_rate = ILendingRateOracle(addressesProvider.getLendingRateOracle()).getMarketBorrowRate(reserve);
            
            if vars.utilization_rate > self.optimal_utilization_rate {
                //TODO decimal
                let excess_utilization_rate = 1 - self.optimal_utilization_rate;
                let excess_utilization_rate_ratio = vars.utilization_rate - self.optimal_utilization_rate/excess_utilization_rate;
        
                vars.current_stable_borrow_rate = vars.current_stable_borrow_rate + self.stable_rate_slope1 + self.stable_rate_slope2 * excess_utilization_rate_ratio;
            } else {
                vars.current_stable_borrow_rate = vars.current_stable_borrow_rate + self.stable_rate_slope1 * (vars.utilization_rate / self.optimal_utilization_rate);
            }
        
            //TODO percentage
            vars.current_liquidity_rate = vars.current_stable_borrow_rate * vars.utilization_rate * (1 - reserve_factor);
            
            (vars.current_liquidity_rate, vars.current_stable_borrow_rate)
        }
    }
}
