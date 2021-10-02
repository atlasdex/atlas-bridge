//! Base curve implementation

use solana_program::{
    program_error::ProgramError,
    msg,
    program_pack::{Pack, Sealed},
};
use crate::curve::{
    calculator::{CurveCalculator, SwapWithoutFeesResult, TradeDirection},
    constant_price::ConstantPriceCurve,
    constant_product::ConstantProductCurve,
    fees::Fees,
    offset::OffsetCurve,
    stable::StableCurve,
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;

/// Curve types supported by the token-swap program.
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    ConstantPrice,
    /// Stable, like uniswap, but with wide zone of 1:1 instead of one point
    Stable,
    /// Offset curve, like Uniswap, but the token B side has a faked offset
    Offset,
}

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapResult {
    /// source swap amount - trade fee
    pub new_swap_amount: u128,
    /// current supply of source token.
    pub swap_source_amount:u128,
    /// destination supply of dest token
    pub dest_amount: u128,
    /// dest
    pub dest_source_amount:u128,
    ///
    pub trade_fee: u128,
}

/// Concrete struct to wrap around the trait object which performs calculation.
#[repr(C)]
#[derive(Debug)]
pub struct SwapCurve {
    /// The type of curve contained in the calculator, helpful for outside
    /// queries
    pub curve_type: CurveType,
    /// The actual calculator, represented as a trait object to allow for many
    /// different types of curves
    pub calculator: Box<dyn CurveCalculator>,
}

impl SwapCurve {
    /// Subtract fees and calculate how much destination token will be provided
    /// given an amount of source token.
    pub fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped        

        msg!("\n\n ---- swap ----");
        msg!("source_amount : {}", source_amount);
        msg!("swap_source_amount : {}", swap_source_amount);
        msg!("swap_destination_amount : {}", swap_destination_amount);
        let trade_fee = fees.trading_fee(source_amount)?;
        msg!("trade_fee : {}", trade_fee);
        let new_source_amount = source_amount.checked_sub(trade_fee)?;
        msg!("new_source_amount : {}", new_source_amount);
        
        msg!("\n");
        Some(SwapResult {
            new_swap_amount: new_source_amount,
            swap_source_amount: swap_source_amount,
            dest_amount:new_source_amount,
            dest_source_amount:swap_destination_amount,
            trade_fee:trade_fee,
        })
    }

    /// Get the amount of pool tokens for the deposited amount of token A or B
    pub fn deposit_single_token_type(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let source_amount = source_amount.checked_sub(trade_fee)?;
        self.calculator.deposit_single_token_type(
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
        )
    }

    /// Get the amount of pool tokens for the withdrawn amount of token A or B
    pub fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let source_amount = source_amount.checked_sub(trade_fee)?;
        self.calculator.withdraw_single_token_type_exact_out(
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
        )
    }
}

/// Default implementation for SwapCurve cannot be derived because of
/// the contained Box.
impl Default for SwapCurve {
    fn default() -> Self {
        let curve_type: CurveType = Default::default();
        let calculator: ConstantProductCurve = Default::default();
        Self {
            curve_type,
            calculator: Box::new(calculator),
        }
    }
}

/// Clone takes advantage of pack / unpack to get around the difficulty of
/// cloning dynamic objects.
/// Note that this is only to be used for testing.
#[cfg(any(test, feature = "fuzz"))]
impl Clone for SwapCurve {
    fn clone(&self) -> Self {
        let mut packed_self = [0u8; Self::LEN];
        Self::pack_into_slice(self, &mut packed_self);
        Self::unpack_from_slice(&packed_self).unwrap()
    }
}

/// Simple implementation for PartialEq which assumes that the output of
/// `Pack` is enough to guarantee equality
impl PartialEq for SwapCurve {
    fn eq(&self, other: &Self) -> bool {
        let mut packed_self = [0u8; Self::LEN];
        Self::pack_into_slice(self, &mut packed_self);
        let mut packed_other = [0u8; Self::LEN];
        Self::pack_into_slice(other, &mut packed_other);
        packed_self[..] == packed_other[..]
    }
}

impl Sealed for SwapCurve {}
impl Pack for SwapCurve {
    /// Size of encoding of all curve parameters, which include fees and any other
    /// constants used to calculate swaps, deposits, and withdrawals.
    /// This includes 1 byte for the type, and 72 for the calculator to use as
    /// it needs.  Some calculators may be smaller than 72 bytes.
    const LEN: usize = 33;

    /// Unpacks a byte buffer into a SwapCurve
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 33];
        #[allow(clippy::ptr_offset_with_cast)]
        let (curve_type, calculator) = array_refs![input, 1, 32];
        let curve_type = curve_type[0].try_into()?;
        Ok(Self {
            curve_type,
            calculator: match curve_type {
                CurveType::ConstantProduct => {
                    Box::new(ConstantProductCurve::unpack_from_slice(calculator)?)
                }
                CurveType::ConstantPrice => {
                    Box::new(ConstantPriceCurve::unpack_from_slice(calculator)?)
                }
                CurveType::Stable => Box::new(StableCurve::unpack_from_slice(calculator)?),
                CurveType::Offset => Box::new(OffsetCurve::unpack_from_slice(calculator)?),
            },
        })
    }

    /// Pack SwapCurve into a byte buffer
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 33];
        let (curve_type, calculator) = mut_array_refs![output, 1, 32];
        curve_type[0] = self.curve_type as u8;
        self.calculator.pack_into_slice(&mut calculator[..]);
    }
}

/// Sensible default of CurveType to ConstantProduct, the most popular and
/// well-known curve type.
impl Default for CurveType {
    fn default() -> Self {
        CurveType::ConstantProduct
    }
}

impl TryFrom<u8> for CurveType {
    type Error = ProgramError;

    fn try_from(curve_type: u8) -> Result<Self, Self::Error> {
        match curve_type {
            0 => Ok(CurveType::ConstantProduct),
            1 => Ok(CurveType::ConstantPrice),
            2 => Ok(CurveType::Stable),
            3 => Ok(CurveType::Offset),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}