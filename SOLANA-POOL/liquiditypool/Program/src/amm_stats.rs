//! State transition types

use crate::curve::{base::SwapCurve, fees::Fees};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use enum_dispatch::enum_dispatch;
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Trait representing access to program state across all versions
#[enum_dispatch]
pub trait AmmStatus {
    /// Is the swap initialized, with data written to it
    fn is_initialized(&self) -> bool;
    /// Bump seed used to generate the program address / authority
    fn nonce(&self) -> u8;
    /// Token program ID associated with the swap
    fn token_program_id(&self) -> &Pubkey;
    /// Address of token A liquidity account
    fn token_a_account(&self) -> &Pubkey;
    /// Address of token B liquidity account
    fn token_b_account(&self) -> &Pubkey;
    /// Address of pool token mint
    fn pool_mint(&self) -> &Pubkey;

    /// Address of token A mint
    fn token_a_mint(&self) -> &Pubkey;
    /// Address of token B mint
    fn token_b_mint(&self) -> &Pubkey;

    /// Address of fee account A
    fn fixed_fee_account_a(&self) -> &Pubkey;

    /// Address of fee account B
    fn fixed_fee_account_b(&self) -> &Pubkey;

    /// Fees associated with swap
    fn fees(&self) -> &Fees;
    /// Curve associated with swap
    fn swap_curve(&self) -> &SwapCurve;
}

/// All versions of AmmStatus
#[enum_dispatch(AmmStatus)]
pub enum SwapVersion {
    /// Latest version, used for all new swaps
    SwapV1,
}

/// SwapVersion does not implement program_pack::Pack because there are size
/// checks on pack and unpack that would break backwards compatibility, so
/// special implementations are provided here
impl SwapVersion {
    /// Size of the latest version of the AmmStatus
    pub const LATEST_LEN: usize = 1 + SwapV1::LEN; // add one for the version enum

    /// Pack a swap into a byte array, based on its version
    pub fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        match src {
            Self::SwapV1(swap_info) => {
                dst[0] = 1;
                SwapV1::pack(swap_info, &mut dst[1..])
            }
        }
    }

    /// Unpack the swap account based on its version, returning the result as a
    /// AmmStatus trait object
    pub fn unpack(input: &[u8]) -> Result<Box<dyn AmmStatus>, ProgramError> {
        let (&version, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidAccountData)?;
        match version {
            1 => Ok(Box::new(SwapV1::unpack(rest)?)),
            _ => Err(ProgramError::UninitializedAccount),
        }
    }

    /// Special check to be done before any instruction processing, works for
    /// all versions
    pub fn is_initialized(input: &[u8]) -> bool {
        match Self::unpack(input) {
            Ok(swap) => swap.is_initialized(),
            Err(_) => false,
        }
    }
}

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct SwapV1 {
    /// Initialized state.
    pub is_initialized: bool,
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,
    
    ///ID of current amm account 
    pub amm_id: Pubkey,

    ///Program ID of Serum Market
    pub dex_program_id: Pubkey,

    ///Market ID of Serum
    pub market_id: Pubkey,

    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,

    /// Token A
    pub token_a: Pubkey,
    /// Token B
    pub token_b: Pubkey,

    /// Pool tokens are issued when A or B tokens are deposited.
    /// Pool tokens can be withdrawn back to the original A or B token.
    pub pool_mint: Pubkey,

    /// Mint information for token A
    pub token_a_mint: Pubkey,
    /// Mint information for token B
    pub token_b_mint: Pubkey,

    /// Token A account to receive trading fees
    pub fixed_fee_account_a: Pubkey,

    /// Token B account to receive trading fees
    pub fixed_fee_account_b: Pubkey,

    /// All fee information
    pub fees: Fees,

    /// Swap curve parameters, to be unpacked and used by the SwapCurve, which
    /// calculates swaps, deposits, and withdrawals
    pub swap_curve: SwapCurve,
}

impl AmmStatus for SwapV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    fn nonce(&self) -> u8 {
        self.nonce
    }

    fn token_program_id(&self) -> &Pubkey {
        &self.token_program_id
    }

    fn token_a_account(&self) -> &Pubkey {
        &self.token_a
    }

    fn token_b_account(&self) -> &Pubkey {
        &self.token_b
    }

    fn pool_mint(&self) -> &Pubkey {
        &self.pool_mint
    }

    fn token_a_mint(&self) -> &Pubkey {
        &self.token_a_mint
    }

    fn token_b_mint(&self) -> &Pubkey {
        &self.token_b_mint
    }

    fn fixed_fee_account_a(&self) -> &Pubkey {
        &self.fixed_fee_account_a
    }

    fn fixed_fee_account_b(&self) -> &Pubkey {
        &self.fixed_fee_account_b
    }

    fn fees(&self) -> &Fees {
        &self.fees
    }

    fn swap_curve(&self) -> &SwapCurve {
        &self.swap_curve
    }
}

impl Sealed for SwapV1 {}
impl IsInitialized for SwapV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for SwapV1 {
    const LEN: usize = 411;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 411];
        let (
            is_initialized,
            nonce,
            amm_id,
            dex_program_id,
            market_id,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            fixed_fee_account_a,
            fixed_fee_account_b,
            fees,
            swap_curve,
        ) = mut_array_refs![output, 1, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 24, 33];
        is_initialized[0] = self.is_initialized as u8;
        nonce[0] = self.nonce;
        amm_id.copy_from_slice(self.amm_id.as_ref());
        dex_program_id.copy_from_slice(self.dex_program_id.as_ref());
        market_id.copy_from_slice(self.market_id.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        token_a.copy_from_slice(self.token_a.as_ref());
        token_b.copy_from_slice(self.token_b.as_ref());
        pool_mint.copy_from_slice(self.pool_mint.as_ref());
        token_a_mint.copy_from_slice(self.token_a_mint.as_ref());
        token_b_mint.copy_from_slice(self.token_b_mint.as_ref());
        fixed_fee_account_a.copy_from_slice(self.fixed_fee_account_a.as_ref());
        fixed_fee_account_b.copy_from_slice(self.fixed_fee_account_b.as_ref());

        self.fees.pack_into_slice(&mut fees[..]);
        self.swap_curve.pack_into_slice(&mut swap_curve[..]);
    }

    /// Unpacks a byte buffer into a [SwapV1](struct.SwapV1.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 411];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            nonce,
            amm_id,
            dex_program_id,
            market_id,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            fixed_fee_account_a,
            fixed_fee_account_b,
            fees,
            swap_curve,
        ) = array_refs![input, 1, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 24, 33];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            nonce: nonce[0],
            amm_id: Pubkey::new_from_array(*amm_id),
            dex_program_id: Pubkey::new_from_array(*dex_program_id),
            market_id: Pubkey::new_from_array(*market_id),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            token_a: Pubkey::new_from_array(*token_a),
            token_b: Pubkey::new_from_array(*token_b),
            pool_mint: Pubkey::new_from_array(*pool_mint),
            token_a_mint: Pubkey::new_from_array(*token_a_mint),
            token_b_mint: Pubkey::new_from_array(*token_b_mint),
            fixed_fee_account_a: Pubkey::new_from_array(*fixed_fee_account_a),
            fixed_fee_account_b: Pubkey::new_from_array(*fixed_fee_account_b),
            fees: Fees::unpack_from_slice(fees)?,
            swap_curve: SwapCurve::unpack_from_slice(swap_curve)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::stable::StableCurve;

    use std::convert::TryInto;

    const TEST_FEES: Fees = Fees {
        return_fee_numerator: 2,
        fixed_fee_numerator: 1,
        fee_denominator: 100
    };

    const TEST_NONCE: u8 = 255;
    const TEST_AMM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const TEST_DEX_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const TEST_MARKET_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const TEST_TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const TEST_TOKEN_A: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const TEST_TOKEN_B: Pubkey = Pubkey::new_from_array([3u8; 32]);
    const TEST_POOL_MINT: Pubkey = Pubkey::new_from_array([4u8; 32]);
    const TEST_TOKEN_A_MINT: Pubkey = Pubkey::new_from_array([5u8; 32]);
    const TEST_TOKEN_B_MINT: Pubkey = Pubkey::new_from_array([6u8; 32]);
    const TEST_SWAP_FEE_ACCOUNT_A: Pubkey = Pubkey::new_from_array([7u8; 32]);
    const TEST_SWAP_FEE_ACCOUNT_B: Pubkey = Pubkey::new_from_array([7u8; 32]);

    const TEST_CURVE_TYPE: u8 = 2;
    const TEST_AMP: u64 = 1;
    const TEST_CURVE: StableCurve = StableCurve { amp: TEST_AMP };

    #[test]
    fn swap_version_pack() {
        let curve_type = TEST_CURVE_TYPE.try_into().unwrap();
        let calculator = Box::new(TEST_CURVE);
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let swap_info = SwapVersion::SwapV1(SwapV1 {
            is_initialized: true,
            nonce: TEST_NONCE,
            amm_id: TEST_AMM_ID,
            dex_program_id: TEST_DEX_PROGRAM_ID,
            market_id: TEST_MARKET_ID,
            token_program_id: TEST_TOKEN_PROGRAM_ID,
            token_a: TEST_TOKEN_A,
            token_b: TEST_TOKEN_B,
            pool_mint: TEST_POOL_MINT,
            token_a_mint: TEST_TOKEN_A_MINT,
            token_b_mint: TEST_TOKEN_B_MINT,
            fixed_fee_account_a: TEST_SWAP_FEE_ACCOUNT_A,
            fixed_fee_account_b: TEST_SWAP_FEE_ACCOUNT_B,
            fees: TEST_FEES,
            swap_curve: swap_curve.clone(),
        });

        let mut packed = [0u8; SwapVersion::LATEST_LEN];
        SwapVersion::pack(swap_info, &mut packed).unwrap();
        let unpacked = SwapVersion::unpack(&packed).unwrap();

        assert!(unpacked.is_initialized());
        assert_eq!(unpacked.nonce(), TEST_NONCE);
        assert_eq!(*unpacked.token_program_id(), TEST_TOKEN_PROGRAM_ID);
        assert_eq!(*unpacked.token_a_account(), TEST_TOKEN_A);
        assert_eq!(*unpacked.token_b_account(), TEST_TOKEN_B);
        assert_eq!(*unpacked.pool_mint(), TEST_POOL_MINT);
        assert_eq!(*unpacked.token_a_mint(), TEST_TOKEN_A_MINT);
        assert_eq!(*unpacked.token_b_mint(), TEST_TOKEN_B_MINT);
        assert_eq!(*unpacked.fixed_fee_account_a(), TEST_SWAP_FEE_ACCOUNT_A);
        assert_eq!(*unpacked.fixed_fee_account_b(), TEST_SWAP_FEE_ACCOUNT_B);
        assert_eq!(*unpacked.fees(), TEST_FEES);
        assert_eq!(*unpacked.swap_curve(), swap_curve);
    }

    #[test]
    fn swap_v1_pack() {
        let curve_type = TEST_CURVE_TYPE.try_into().unwrap();
        let calculator = Box::new(TEST_CURVE);
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let swap_info = SwapV1 {
            is_initialized: true,
            nonce: TEST_NONCE,
            amm_id: TEST_AMM_ID,
            dex_program_id: TEST_DEX_PROGRAM_ID,
            market_id: TEST_MARKET_ID,
            token_program_id: TEST_TOKEN_PROGRAM_ID,
            token_a: TEST_TOKEN_A,
            token_b: TEST_TOKEN_B,
            pool_mint: TEST_POOL_MINT,
            token_a_mint: TEST_TOKEN_A_MINT,
            token_b_mint: TEST_TOKEN_B_MINT,
            fixed_fee_account_a: TEST_SWAP_FEE_ACCOUNT_A,
            fixed_fee_account_b: TEST_SWAP_FEE_ACCOUNT_B,
            fees: TEST_FEES,
            swap_curve,
        };

        let mut packed = [0u8; SwapV1::LEN];
        SwapV1::pack_into_slice(&swap_info, &mut packed);
        let unpacked = SwapV1::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let mut packed = vec![1u8, TEST_NONCE];
        packed.extend_from_slice(&TEST_AMM_ID.to_bytes());
        packed.extend_from_slice(&TEST_DEX_PROGRAM_ID.to_bytes());
        packed.extend_from_slice(&TEST_MARKET_ID.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_PROGRAM_ID.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_A.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_B.to_bytes());
        packed.extend_from_slice(&TEST_POOL_MINT.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_A_MINT.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_B_MINT.to_bytes());
        packed.extend_from_slice(&TEST_SWAP_FEE_ACCOUNT_A.to_bytes());
        packed.extend_from_slice(&TEST_SWAP_FEE_ACCOUNT_B.to_bytes());

        packed.extend_from_slice(&TEST_FEES.return_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.fixed_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.fee_denominator.to_le_bytes());
        packed.push(TEST_CURVE_TYPE);
        packed.extend_from_slice(&TEST_AMP.to_le_bytes());
        packed.extend_from_slice(&[0u8; 24]);
        let unpacked = SwapV1::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let packed = [0u8; SwapV1::LEN];
        let swap_info: SwapV1 = Default::default();
        let unpack_unchecked = SwapV1::unpack_unchecked(&packed).unwrap();
        assert_eq!(unpack_unchecked, swap_info);
        let err = SwapV1::unpack(&packed).unwrap_err();
        assert_eq!(err, ProgramError::UninitializedAccount);
    }
}
