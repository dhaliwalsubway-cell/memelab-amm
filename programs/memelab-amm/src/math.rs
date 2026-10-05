use crate::error::AmmError;

pub fn amount_out(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<u64, AmmError> {
    require_math!(amount_in > 0, AmmError::ZeroAmount);
    require_math!(reserve_in > 0, AmmError::InsufficientLiquidity);
    require_math!(reserve_out > 0, AmmError::InsufficientLiquidity);

    let fee_denominator: u64 = 10_000;

    let fee_multiplier = fee_denominator
        .checked_sub(fee_bps as u64)
        .ok_or(AmmError::InvalidFee)?;

    let amount_in_after_fee = (amount_in as u128)
        .checked_mul(fee_multiplier as u128)
        .ok_or(AmmError::MathOverflow)?
        / fee_denominator as u128;

    require_math!(
        amount_in_after_fee > 0,
        AmmError::AmountTooSmall
    );

    let numerator = amount_in_after_fee
        .checked_mul(reserve_out as u128)
        .ok_or(AmmError::MathOverflow)?;

    let denominator = (reserve_in as u128)
        .checked_add(amount_in_after_fee)
        .ok_or(AmmError::MathOverflow)?;

    let amount_out = numerator
        .checked_div(denominator)
        .ok_or(AmmError::MathOverflow)?;

    require_math!(
        amount_out < reserve_out as u128,
        AmmError::InsufficientLiquidity
    );

    Ok(amount_out as u64)
}

macro_rules! require_math {
    ($condition:expr, $error:expr) => {
        if !$condition {
            return Err($error);
        }
    };
}

pub(crate) use require_math;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_out_normal() {
        let output =
            amount_out(1_000, 10_000, 10_000, 30).unwrap();

        assert_eq!(output, 906);
    }

    #[test]
    fn test_amount_out_reverse_direction() {
        let output =
            amount_out(1_000, 10_000, 10_000, 30).unwrap();

        // The formula is direction-independent:
        // changing which reserve is "in" versus "out"
        // produces the same result when the reserves are equal.
        assert_eq!(output, 906);
    }

    #[test]
    fn test_fee_reduces_output() {
        let without_fee =
            amount_out(1_000, 10_000, 10_000, 0).unwrap();

        let with_fee =
            amount_out(1_000, 10_000, 10_000, 30).unwrap();

        assert!(with_fee < without_fee);
    }

    #[test]
    fn test_zero_input_rejected() {
        let result =
            amount_out(0, 10_000, 10_000, 30);

        assert_eq!(result, Err(AmmError::ZeroAmount));
    }

    #[test]
    fn test_zero_input_reserve_rejected() {
        let result =
            amount_out(1_000, 0, 10_000, 30);

        assert_eq!(
            result,
            Err(AmmError::InsufficientLiquidity)
        );
    }

    #[test]
    fn test_zero_output_reserve_rejected() {
        let result =
            amount_out(1_000, 10_000, 0, 30);

        assert_eq!(
            result,
            Err(AmmError::InsufficientLiquidity)
        );
    }

    #[test]
    fn test_tiny_input_rejected_after_fee() {
        let result =
            amount_out(1, 10_000, 10_000, 30);

        assert_eq!(
            result,
            Err(AmmError::AmountTooSmall)
        );
    }

    #[test]
    fn test_output_never_consumes_entire_reserve() {
        let output =
            amount_out(
                9_000,
                10_000,
                10_000,
                30,
            )
            .unwrap();

        assert!(output < 10_000);
    }

    #[test]
    fn test_constant_product_invariant_holds_after_buy() {
        let amount_in = 1_000_000u64;
        let reserve_in = 10_000_000u64;
        let reserve_out = 10_000_000u64;
        let fee_bps = 30u16;

        let amount_out_value =
            amount_out(
                amount_in,
                reserve_in,
                reserve_out,
                fee_bps,
            )
            .unwrap();

        let fee_denominator = 10_000u128;
        let fee_multiplier = fee_denominator - fee_bps as u128;
        let amount_in_after_fee =
            (amount_in as u128 * fee_multiplier)
                / fee_denominator;

        let k_before =
            reserve_in as u128 * reserve_out as u128;

        let reserve_in_after =
            reserve_in as u128 + amount_in_after_fee;

        let reserve_out_after =
            reserve_out as u128 - amount_out_value as u128;

        let k_after =
            reserve_in_after * reserve_out_after;

        assert!(
            k_after >= k_before,
            "constant-product invariant violated: before={}, after={}",
            k_before,
            k_after
        );
    }

    #[test]
    fn test_large_values_do_not_overflow() {
        let output =
            amount_out(
                1_000_000_000_000,
                10_000_000_000_000,
                10_000_000_000_000,
                30,
            )
            .unwrap();

        assert!(output > 0);
        assert!(output < 10_000_000_000_000);
    }
}
