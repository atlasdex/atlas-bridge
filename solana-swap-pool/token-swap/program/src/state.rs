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
pub trait SwapState {
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

    /// Address of pool fee account
    fn pool_fee_account(&self) -> &Pubkey;

    /// Fees associated with swap
    fn fees(&self) -> &Fees;
    /// Curve associated with swap
    fn swap_curve(&self) -> &SwapCurve;
}

/// All versions of SwapState
#[enum_dispatch(SwapState)]
pub enum SwapVersion {
    /// Latest version, used for all new swaps
    SwapV1,
}

/// SwapVersion does not implement program_pack::Pack because there are size
/// checks on pack and unpack that would break backwards compatibility, so
/// special implementations are provided here
impl SwapVersion {
    /// Size of the latest version of the SwapState
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
    /// SwapState trait object
    pub fn unpack(input: &[u8]) -> Result<Box<dyn SwapState>, ProgramError> {
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

    /// Pool token account to receive trading and / or withdrawal fees
    pub pool_fee_account: Pubkey,

    /// All fee information
    pub fees: Fees,

    /// Swap curve parameters, to be unpacked and used by the SwapCurve, which
    /// calculates swaps, deposits, and withdrawals
    pub swap_curve: SwapCurve,
}

impl SwapState for SwapV1 {
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

    fn pool_fee_account(&self) -> &Pubkey {
        &self.pool_fee_account
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
    const LEN: usize = 323;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 323];
        let (
            is_initialized,
            nonce,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            pool_fee_account,
            fees,
            swap_curve,
        ) = mut_array_refs![output, 1, 1, 32, 32, 32, 32, 32, 32, 32, 64, 33];
        is_initialized[0] = self.is_initialized as u8;
        nonce[0] = self.nonce;
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        token_a.copy_from_slice(self.token_a.as_ref());
        token_b.copy_from_slice(self.token_b.as_ref());
        pool_mint.copy_from_slice(self.pool_mint.as_ref());
        token_a_mint.copy_from_slice(self.token_a_mint.as_ref());
        token_b_mint.copy_from_slice(self.token_b_mint.as_ref());
        pool_fee_account.copy_from_slice(self.pool_fee_account.as_ref());
        self.fees.pack_into_slice(&mut fees[..]);
        self.swap_curve.pack_into_slice(&mut swap_curve[..]);
    }

    /// Unpacks a byte buffer into a [SwapV1](struct.SwapV1.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 323];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            nonce,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            pool_fee_account,
            fees,
            swap_curve,
        ) = array_refs![input, 1, 1, 32, 32, 32, 32, 32, 32, 32, 64, 33];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            nonce: nonce[0],
            token_program_id: Pubkey::new_from_array(*token_program_id),
            token_a: Pubkey::new_from_array(*token_a),
            token_b: Pubkey::new_from_array(*token_b),
            pool_mint: Pubkey::new_from_array(*pool_mint),
            token_a_mint: Pubkey::new_from_array(*token_a_mint),
            token_b_mint: Pubkey::new_from_array(*token_b_mint),
            pool_fee_account: Pubkey::new_from_array(*pool_fee_account),
            fees: Fees::unpack_from_slice(fees)?,
            swap_curve: SwapCurve::unpack_from_slice(swap_curve)?,
        })
    }
}