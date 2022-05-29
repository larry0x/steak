use cosmwasm_std::Decimal;

pub fn get_reward_fee_cap() -> Decimal {
    // 10% max reward fee
    Decimal::from_ratio(10_u128, 100_u128)
}