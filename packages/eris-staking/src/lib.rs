pub mod hub;


mod decimal_checked_ops {
    use cosmwasm_bignumber::Decimal256;
    use cosmwasm_std::{Decimal, Fraction, OverflowError, StdError, Uint128, Uint256};
    use std::{convert::TryInto, ops::Mul};
    pub trait DecimalCheckedOps {
        fn checked_add(self, other: Decimal) -> Result<Decimal, StdError>;
        fn checked_mul(self, other: Uint128) -> Result<Uint128, StdError>;
        fn checked_mul_dec(self, other: Decimal) -> Result<Decimal, StdError>;
    }

    impl DecimalCheckedOps for Decimal {
        fn checked_add(self, other: Decimal) -> Result<Decimal, StdError> {
            Uint128::from(self.numerator())
                .checked_add(other.numerator().into())
                .map(|_| self + other)
                .map_err(StdError::overflow)
        }

        fn checked_mul(self, other: Uint128) -> Result<Uint128, StdError> {
            if self.is_zero() || other.is_zero() {
                return Ok(Uint128::zero());
            }
            let multiply_ratio =
                other.full_mul(self.numerator()) / Uint256::from(self.denominator());
            if multiply_ratio > Uint256::from(Uint128::MAX) {
                Err(StdError::overflow(OverflowError::new(
                    cosmwasm_std::OverflowOperation::Mul,
                    self,
                    other,
                )))
            } else {
                Ok(multiply_ratio.try_into().unwrap())
            }
        }

        fn checked_mul_dec(self, other: Decimal) -> Result<Decimal, StdError> {
            if self.is_zero() || other.is_zero() {
                return Ok(Decimal::zero());
            }
            let result: Decimal = Decimal256::from(self)
                .mul(Decimal256::from(other))
                .try_into()
                .map_err(|_| StdError::generic_err("Could not convert"))?;
            Ok(result)
        }
    }
}

pub use decimal_checked_ops::DecimalCheckedOps;