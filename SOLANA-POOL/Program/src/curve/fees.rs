//! All fee information, to be used for validation currently

use crate::error::AmmError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryFrom;

/// Encapsulates all fee information and calculations for swap operations
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fees {
    /// Trade fees are extra token amounts that are held inside the token
    /// accounts during a trade, making the value of liquidity tokens rise.
    
    /// fee numerator to reinjected to the pool
    pub return_fee_numerator: u64,
    
    /// fee numerator to reinjected to the fixed account
    pub fixed_fee_numerator: u64,

    /// fee dominator 
    pub fee_denominator: u64
}

/// Helper function for calculating swap fee
pub fn calculate_fee(
    token_amount: u128,
    fee_numerator: u128,
    fee_denominator: u128,
) -> Option<u128> {
    if fee_numerator == 0 || token_amount == 0 {
        Some(0)
    } else {
        let fee = token_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        if fee == 0 {
            Some(1) // minimum fee of one token
        } else {
            Some(fee)
        }
    }
}

// fn validate_fraction(numerator: u64, denominator: u64) -> Result<(), AmmError> {
//     if denominator == 0 && numerator == 0 {
//         Ok(())
//     } else if numerator >= denominator {
//         Err(AmmError::InvalidFee)
//     } else {
//         Ok(())
//     }
// }

impl Fees {
    /// Calculate the withdraw fee in pool tokens
    pub fn return_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.return_fee_numerator).ok()?,
            u128::try_from(self.fee_denominator).ok()?,
        )
    }

    /// Calculate the trading fee in trading tokens
    pub fn fixed_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.fixed_fee_numerator).ok()?,
            u128::try_from(self.fee_denominator).ok()?,
        )
    }

    /// Validate that the fees are reasonable
    pub fn validate(&self) -> Result<(), AmmError> {

        if self.fee_denominator == 0 && self.fixed_fee_numerator == 0  && self.return_fee_numerator == 0{
            Ok(())
        } else if self.fixed_fee_numerator +  self.return_fee_numerator >= self.fee_denominator {
            Err(AmmError::InvalidFee)
        } else {
            Ok(())
        }
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for Fees {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Sealed for Fees {}
impl Pack for Fees {
    const LEN: usize = 24;
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 24];
        let (
            return_fee_numerator,
            fixed_fee_numerator,
            fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8];
        *return_fee_numerator = self.return_fee_numerator.to_le_bytes();
        *fixed_fee_numerator = self.fixed_fee_numerator.to_le_bytes();
        *fee_denominator = self.fee_denominator.to_le_bytes();
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Fees, ProgramError> {
        let input = array_ref![input, 0, 24];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            return_fee_numerator,
            fixed_fee_numerator,
            fee_denominator,
        ) = array_refs![input, 8, 8, 8];
        Ok(Self {
            return_fee_numerator: u64::from_le_bytes(*return_fee_numerator),
            fixed_fee_numerator: u64::from_le_bytes(*fixed_fee_numerator),
            fee_denominator: u64::from_le_bytes(*fee_denominator),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_fees() {
        let return_fee_numerator = 2;
        let fixed_fee_numerator = 1;
        let fee_denominator = 100;
        let fees = Fees {
            return_fee_numerator,
            fixed_fee_numerator,
            fee_denominator
        };

        let mut packed = [0u8; Fees::LEN];
        Pack::pack_into_slice(&fees, &mut packed[..]);
        let unpacked = Fees::unpack_from_slice(&packed).unwrap();
        assert_eq!(fees, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&return_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&fixed_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&fee_denominator.to_le_bytes());
        let unpacked = Fees::unpack_from_slice(&packed).unwrap();
        assert_eq!(fees, unpacked);
    }
}
