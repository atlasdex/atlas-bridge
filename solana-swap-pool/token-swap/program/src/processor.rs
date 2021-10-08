//! Program state processor

use crate::constraints::{SwapConstraints, SWAP_CONSTRAINTS};
use crate::{
    curve::{
        base::SwapCurve,
        calculator::{RoundDirection, TradeDirection},
        fees::Fees,
    },
    error::SwapError,
    instruction::{
        DepositAllTokenTypes, DepositSingleTokenTypeExactAmountIn, Initialize, Swap,
        SwapInstruction, WithdrawAllTokenTypes, WithdrawSingleTokenTypeExactAmountOut,
    },
    state::{SwapState, SwapV1, SwapVersion},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
};
use std::convert::TryInto;
use std::convert::TryFrom;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Account, SwapError> {
        if account_info.owner != token_program_id {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Account::unpack(&account_info.data.borrow())
                .map_err(|_| SwapError::ExpectedAccount)
        }
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Mint, SwapError> {
        if account_info.owner != token_program_id {
            Err(SwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Mint::unpack(&account_info.data.borrow())
                .map_err(|_| SwapError::ExpectedMint)
        }
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        nonce: u8,
    ) -> Result<Pubkey, SwapError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(SwapError::InvalidProgramAddress))
    }

    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(
            &ix,
            &[burn_account, mint, authority, token_program],
            signers,
        )
    }

    /// Issue a spl_token `MintTo` instruction.
    pub fn token_mint_to<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[mint, destination, authority, token_program], signers)
    }

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn check_accounts(
        token_swap: &dyn SwapState,
        program_id: &Pubkey,
        swap_account_info: &AccountInfo,
        authority_info: &AccountInfo,
        token_a_info: &AccountInfo,
        token_b_info: &AccountInfo,
        pool_mint_info: &AccountInfo,
        token_program_info: &AccountInfo,
        user_token_a_info: Option<&AccountInfo>,
        user_token_b_info: Option<&AccountInfo>,
        pool_fee_account_info: Option<&AccountInfo>,
    ) -> ProgramResult {
        if swap_account_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, swap_account_info.key, token_swap.nonce())?
        {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != *token_swap.token_a_account() {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *token_b_info.key != *token_swap.token_b_account() {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        if *pool_mint_info.key != *token_swap.pool_mint() {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *token_program_info.key != *token_swap.token_program_id() {
            return Err(SwapError::IncorrectTokenProgramId.into());
        }
        if let Some(user_token_a_info) = user_token_a_info {
            if token_a_info.key == user_token_a_info.key {
                return Err(SwapError::InvalidInput.into());
            }
        }
        if let Some(user_token_b_info) = user_token_b_info {
            if token_b_info.key == user_token_b_info.key {
                return Err(SwapError::InvalidInput.into());
            }
        }
        if let Some(pool_fee_account_info) = pool_fee_account_info {
            if *pool_fee_account_info.key != *token_swap.pool_fee_account() {
                return Err(SwapError::IncorrectFeeAccount.into());
            }
        }
        Ok(())
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        nonce: u8,
        fees: Fees,
        swap_curve: SwapCurve,
        accounts: &[AccountInfo],
        swap_constraints: &Option<SwapConstraints>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *token_program_info.key;
        if SwapVersion::is_initialized(&swap_info.data.borrow()) {
            return Err(SwapError::AlreadyInUse.into());
        }
        
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        let token_a = Self::unpack_token_account(token_a_info, &token_program_id)?;
        let token_b = Self::unpack_token_account(token_b_info, &token_program_id)?;
        let fee_account = Self::unpack_token_account(fee_account_info, &token_program_id)?;
        let destination = Self::unpack_token_account(destination_info, &token_program_id)?;
        let pool_mint = Self::unpack_mint(pool_mint_info, &token_program_id)?;
        if *authority_info.key != token_a.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key != token_b.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key == destination.owner {
            return Err(SwapError::InvalidOutputOwner.into());
        }
        if *authority_info.key == fee_account.owner {
            return Err(SwapError::InvalidOutputOwner.into());
        }
        if COption::Some(*authority_info.key) != pool_mint.mint_authority {
            return Err(SwapError::InvalidOwner.into());
        }

        if token_a.mint == token_b.mint {
            return Err(SwapError::RepeatedMint.into());
        }
        
        swap_curve
            .calculator
            .validate_supply(token_a.amount, token_b.amount)?;
        if token_a.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if token_b.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if token_a.close_authority.is_some() {
            return Err(SwapError::InvalidCloseAuthority.into());
        }
        if token_b.close_authority.is_some() {
            return Err(SwapError::InvalidCloseAuthority.into());
        }

        if pool_mint.supply != 0 {
            return Err(SwapError::InvalidSupply.into());
        }
        if pool_mint.freeze_authority.is_some() {
            return Err(SwapError::InvalidFreezeAuthority.into());
        }
        if *pool_mint_info.key != fee_account.mint {
            return Err(SwapError::IncorrectPoolMint.into());
        }

        if let Some(swap_constraints) = swap_constraints {
            let owner_key = swap_constraints
                .owner_key
                .parse::<Pubkey>()
                .map_err(|_| SwapError::InvalidOwner)?;
            if fee_account.owner != owner_key {
                return Err(SwapError::InvalidOwner.into());
            }
            swap_constraints.validate_curve(&swap_curve)?;
            swap_constraints.validate_fees(&fees)?;
        }
        fees.validate()?;
        swap_curve.calculator.validate()?;
                
        // let token_a_amount_u128 = to_u128(token_a.amount)?;
        // let token_b_amount_u128 = to_u128(token_a.amount)?;
        // // let initial_supply = (token_a_amount_u128 + token_b_amount_u128)/2;
        // let initial_supply = token_a_amount_u128.checked_add(token_b_amount_u128);
        // initial_supply = initial_supply.checked_div(2)?;
        // msg!("token_a_amount_u128 : {}, token_b_amount_u128 : {} , initial_supply : {}",
        //     token_a_amount_u128, token_b_amount_u128, initial_supply);
        
        let initial_amount = swap_curve.calculator.new_pool_supply();

        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            nonce,
            to_u64(initial_amount)?,
        )?;

        let obj = SwapVersion::SwapV1(SwapV1 {
            is_initialized: true,
            nonce,
            token_program_id,
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_mint_info.key,
            token_a_mint: token_a.mint,
            token_b_mint: token_b.mint,
            pool_fee_account: *fee_account_info.key,
            fees,
            swap_curve,
        });
        SwapVersion::pack(obj, &mut swap_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        // get account info iterator
        let account_info_iter = &mut accounts.iter();
        // get swap info
        let swap_info = next_account_info(account_info_iter)?;
        // get authority info
        let authority_info = next_account_info(account_info_iter)?;
        // get user transfer autority info
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        // get source info
        let source_info = next_account_info(account_info_iter)?;
        // get swap source info
        let swap_source_info = next_account_info(account_info_iter)?;
        // get swap destination info
        let swap_destination_info = next_account_info(account_info_iter)?;
        // get destination info
        let destination_info = next_account_info(account_info_iter)?;
        // get pool mint info
        let pool_mint_info = next_account_info(account_info_iter)?;
        // get pool fee account info
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        // get token program info
        let token_program_info = next_account_info(account_info_iter)?;
        // if swap owner is not program_id, then return incorrect program id error
        if swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        // get token_swap by swap_info.data
        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        // if autority_info.key is not authority id then return invalid program address error
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce())?
        {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        // if swap_source_info.key is token a account of token_swap or 
        // swap source info.key is token b account of token_swap then return incorrect swap account er
        if !(*swap_source_info.key == *token_swap.token_a_account()
            || *swap_source_info.key == *token_swap.token_b_account())
        {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        //if swap_destination_info.key is token a account of token_swap or 
        //swap_destination_info.key is token b account of token_swap then return incorrect swap account er
        if !(*swap_destination_info.key == *token_swap.token_a_account()
            || *swap_destination_info.key == *token_swap.token_b_account())
        {
            return Err(SwapError::IncorrectSwapAccount.into());
        }
        // if swap source info.key is swap destination key then return invalid input error
        if *swap_source_info.key == *swap_destination_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        // if swap source info key is source info key then return invalid input
        if swap_source_info.key == source_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        // if swap destination info key is destination info key then return invalid input key
        if swap_destination_info.key == destination_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        // if pool mint info key is not token swap pool mint
        if *pool_mint_info.key != *token_swap.pool_mint() {
            return Err(SwapError::IncorrectPoolMint.into());
        }
        if *pool_fee_account_info.key != *token_swap.pool_fee_account() {
            return Err(SwapError::IncorrectFeeAccount.into());
        }
        if *token_program_info.key != *token_swap.token_program_id() {
            return Err(SwapError::IncorrectTokenProgramId.into());
        }
        
        let source_account =
            Self::unpack_token_account(swap_source_info, token_swap.token_program_id())?;
        let dest_account =
            Self::unpack_token_account(swap_destination_info, token_swap.token_program_id())?;
        // let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        let trade_direction = if *swap_source_info.key == *token_swap.token_a_account() {
            TradeDirection::AtoB
        } else {
            TradeDirection::BtoA
        };
        let result = token_swap
            .swap_curve()
            .swap(
                to_u128(amount_in)?,
                to_u128(source_account.amount)?,
                to_u128(dest_account.amount)?,
                trade_direction,
                token_swap.fees(),
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        if result.dest_amount < to_u128(minimum_amount_out)? {
            return Err(SwapError::ExceededSlippage.into());
        }

        // let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
        //     TradeDirection::AtoB => (
        //         result.new_swap_amount,
        //         result.dest_amount,
        //     ),
        //     TradeDirection::BtoA => (
        //         result.dest_amount,
        //         result.new_swap_amount,
        //     ),
        // };

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            swap_source_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(result.dest_amount)?,
        )?;

        // let mut pool_token_amount = token_swap
        //     .swap_curve()
        //     .withdraw_single_token_type_exact_out(
        //         result.owner_fee,
        //         swap_token_a_amount,
        //         swap_token_b_amount,
        //         to_u128(pool_mint.supply)?,
        //         trade_direction,
        //         token_swap.fees(),
        //     )
        //     .ok_or(SwapError::FeeCalculationFailure)?;

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            swap_destination_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            to_u64(result.dest_amount)?,
        )?;

        Ok(())
    }
    fn calculate_stable_lp_amount(
        total_supply:u128,
        token_a_amount: u128,
        token_b_amount: u128,
        input_a_amount: u128,
        input_b_amount: u128,
    )->Option<u128>{
        let total_amount = token_a_amount.checked_add(token_b_amount)?;
        let input_amount = input_a_amount.checked_add(input_b_amount)?;
        let pool_amount = input_amount.checked_mul(total_supply)?.checked_div(total_amount)?;
        
        Some(pool_amount)
        // u128::try_from(pool_amount).ok()
    }
    /// Processes an [DepositAllTokenTypes](enum.Instruction.html).
    pub fn process_deposit_all_token_types(
        program_id: &Pubkey,
        pool_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_a_info = next_account_info(account_info_iter)?;
        let source_b_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let calculator = &token_swap.swap_curve().calculator;
        if !calculator.allows_deposits() {
            return Err(SwapError::UnsupportedCurveOperation.into());
        }
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            token_a_info,
            token_b_info,
            pool_mint_info,
            token_program_info,
            Some(source_a_info),
            Some(source_b_info),
            None,
        )?;
        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
                
        let pool_amount = Self::calculate_stable_lp_amount(
            to_u128(pool_mint.supply)?,
            to_u128(token_a.amount)?,
            to_u128(token_b.amount)?,
            to_u128(maximum_token_a_amount)?,
            to_u128(maximum_token_b_amount)?
        ).ok_or(SwapError::ZeroTradingTokens)?;

        msg!("\n");
        msg!("----  token_a lp amount : {:?} ----", token_a.amount);
        msg!("----  token_b lp amount : {:?} ----", token_b.amount);
        msg!("----  input a amount : {:?} ----", maximum_token_a_amount);
        msg!("----  input b amount : {:?} ----", maximum_token_b_amount);
        msg!("----  current supply : {:?} ----", pool_mint.supply);
        msg!("----  temp supply amount : {:?} ----", pool_amount);

        msg!("\n");
 
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_a_info.clone(),
            token_a_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            maximum_token_a_amount,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_b_info.clone(),
            token_b_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            maximum_token_b_amount,
        )?;
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            dest_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            to_u64(pool_amount)?,
        )?;

        Ok(())
    }

    /// Processes an [WithdrawAllTokenTypes](enum.Instruction.html).
    pub fn process_withdraw_all_token_types(
        program_id: &Pubkey,
        pool_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            token_a_info,
            token_b_info,
            pool_mint_info,
            token_program_info,
            Some(dest_token_a_info),
            Some(dest_token_b_info),
            Some(pool_fee_account_info),
        )?;

        let token_a = Self::unpack_token_account(token_a_info, token_swap.token_program_id())?;
        let token_b = Self::unpack_token_account(token_b_info, token_swap.token_program_id())?;
        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;

        let calculator = &token_swap.swap_curve().calculator;

        // let withdraw_fee: u128 = if *pool_fee_account_info.key == *source_info.key {
        //     // withdrawing from the fee account, don't assess withdraw fee
        //     0
        // } else {
        //     token_swap
        //         .fees()
        //         .owner_withdraw_fee(to_u128(pool_token_amount)?)
        //         .ok_or(SwapError::FeeCalculationFailure)?
        // };
        let pool_token_amount = to_u128(pool_token_amount)?;
        //     .checked_sub(withdraw_fee)
        //     .ok_or(SwapError::CalculationFailure)?;
        msg!("\n ---- withdraw token ----");
        msg!(" pool token amount : {} ", pool_token_amount);
        msg!(" token a amount : {} ", token_a.amount);
        msg!(" token b amount : {} ", token_b.amount);
        msg!(" minimum_token_a_amount : {} ", minimum_token_a_amount);
        msg!(" minimum_token_b_amount : {} ", minimum_token_b_amount);
        msg!("\n");

        let results = calculator
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                to_u128(pool_mint.supply)?,
                to_u128(token_a.amount)?,
                to_u128(token_b.amount)?,
                RoundDirection::Floor,
            )
            .ok_or(SwapError::ZeroTradingTokens)?;
        let token_a_amount = to_u64(results.token_a_amount)?;
        let token_a_amount = std::cmp::min(token_a.amount, token_a_amount);
        if token_a_amount < minimum_token_a_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_a_amount == 0 && token_a.amount != 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }
        let token_b_amount = to_u64(results.token_b_amount)?;
        let token_b_amount = std::cmp::min(token_b.amount, token_b_amount);
        if token_b_amount < minimum_token_b_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if token_b_amount == 0 && token_b.amount != 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        // if withdraw_fee > 0 {
        //     Self::token_transfer(
        //         swap_info.key,
        //         token_program_info.clone(),
        //         source_info.clone(),
        //         pool_fee_account_info.clone(),
        //         user_transfer_authority_info.clone(),
        //         token_swap.nonce(),
        //         to_u64(withdraw_fee)?,
        //     )?;
        // }
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(pool_token_amount)?,
        )?;

        if token_a_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                token_a_info.clone(),
                dest_token_a_info.clone(),
                authority_info.clone(),
                token_swap.nonce(),
                token_a_amount,
            )?;
        }
        if token_b_amount > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                token_b_info.clone(),
                dest_token_b_info.clone(),
                authority_info.clone(),
                token_swap.nonce(),
                token_b_amount,
            )?;
        }
        Ok(())
    }

    /// Processes DepositSingleTokenTypeExactAmountIn
    pub fn process_deposit_single_token_type_exact_amount_in(
        program_id: &Pubkey,
        source_token_amount: u64,
        minimum_pool_token_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_token_a_info = next_account_info(account_info_iter)?;
        let swap_token_b_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let source_account =
            Self::unpack_token_account(source_info, token_swap.token_program_id())?;
        let swap_token_a =
            Self::unpack_token_account(swap_token_a_info, token_swap.token_program_id())?;
        let swap_token_b =
            Self::unpack_token_account(swap_token_b_info, token_swap.token_program_id())?;

        let trade_direction = if source_account.mint == swap_token_a.mint {
            TradeDirection::AtoB
        } else if source_account.mint == swap_token_b.mint {
            TradeDirection::BtoA
        } else {
            return Err(SwapError::IncorrectSwapAccount.into());
        };

        let (source_a_info, source_b_info) = match trade_direction {
            TradeDirection::AtoB => (Some(source_info), None),
            TradeDirection::BtoA => (None, Some(source_info)),
        };

        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            swap_token_a_info,
            swap_token_b_info,
            pool_mint_info,
            token_program_info,
            source_a_info,
            source_b_info,
            None,
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = to_u128(pool_mint.supply)?;
        let pool_token_amount = if pool_mint_supply > 0 {
            token_swap
                .swap_curve()
                .deposit_single_token_type(
                    to_u128(source_token_amount)?,
                    to_u128(swap_token_a.amount)?,
                    to_u128(swap_token_b.amount)?,
                    pool_mint_supply,
                    trade_direction,
                    token_swap.fees(),
                )
                .ok_or(SwapError::ZeroTradingTokens)?
        } else {
            token_swap.swap_curve().calculator.new_pool_supply()
        };

        let pool_token_amount = to_u64(pool_token_amount)?;
        if pool_token_amount < minimum_pool_token_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if pool_token_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        match trade_direction {
            TradeDirection::AtoB => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    source_info.clone(),
                    swap_token_a_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.nonce(),
                    source_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    source_info.clone(),
                    swap_token_b_info.clone(),
                    user_transfer_authority_info.clone(),
                    token_swap.nonce(),
                    source_token_amount,
                )?;
            }
        }
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce(),
            pool_token_amount,
        )?;

        Ok(())
    }

    /// Processes a [WithdrawSingleTokenTypeExactAmountOut](enum.Instruction.html).
    pub fn process_withdraw_single_token_type_exact_amount_out(
        program_id: &Pubkey,
        destination_token_amount: u64,
        maximum_pool_token_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_token_a_info = next_account_info(account_info_iter)?;
        let swap_token_b_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapVersion::unpack(&swap_info.data.borrow())?;
        let destination_account =
            Self::unpack_token_account(destination_info, token_swap.token_program_id())?;
        let swap_token_a =
            Self::unpack_token_account(swap_token_a_info, token_swap.token_program_id())?;
        let swap_token_b =
            Self::unpack_token_account(swap_token_b_info, token_swap.token_program_id())?;

        let trade_direction = if destination_account.mint == swap_token_a.mint {
            TradeDirection::AtoB
        } else if destination_account.mint == swap_token_b.mint {
            TradeDirection::BtoA
        } else {
            return Err(SwapError::IncorrectSwapAccount.into());
        };

        let (destination_a_info, destination_b_info) = match trade_direction {
            TradeDirection::AtoB => (Some(destination_info), None),
            TradeDirection::BtoA => (None, Some(destination_info)),
        };
        Self::check_accounts(
            token_swap.as_ref(),
            program_id,
            swap_info,
            authority_info,
            swap_token_a_info,
            swap_token_b_info,
            pool_mint_info,
            token_program_info,
            destination_a_info,
            destination_b_info,
            Some(pool_fee_account_info),
        )?;

        let pool_mint = Self::unpack_mint(pool_mint_info, token_swap.token_program_id())?;
        let pool_mint_supply = to_u128(pool_mint.supply)?;
        let swap_token_a_amount = to_u128(swap_token_a.amount)?;
        let swap_token_b_amount = to_u128(swap_token_b.amount)?;

        let burn_pool_token_amount = token_swap
            .swap_curve()
            .withdraw_single_token_type_exact_out(
                to_u128(destination_token_amount)?,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_mint_supply,
                trade_direction,
                token_swap.fees(),
            )
            .ok_or(SwapError::ZeroTradingTokens)?;

        let withdraw_fee: u128 = if *pool_fee_account_info.key == *source_info.key {
            // withdrawing from the fee account, don't assess withdraw fee
            0
        }
         else {
             0
            // token_swap
            //     .fees()
            //     .owner_withdraw_fee(burn_pool_token_amount)
            //     .ok_or(SwapError::FeeCalculationFailure)?
        };
        let pool_token_amount = burn_pool_token_amount
            .checked_add(withdraw_fee)
            .ok_or(SwapError::CalculationFailure)?;

        if to_u64(pool_token_amount)? > maximum_pool_token_amount {
            return Err(SwapError::ExceededSlippage.into());
        }
        if pool_token_amount == 0 {
            return Err(SwapError::ZeroTradingTokens.into());
        }

        if withdraw_fee > 0 {
            Self::token_transfer(
                swap_info.key,
                token_program_info.clone(),
                source_info.clone(),
                pool_fee_account_info.clone(),
                user_transfer_authority_info.clone(),
                token_swap.nonce(),
                to_u64(withdraw_fee)?,
            )?;
        }
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            user_transfer_authority_info.clone(),
            token_swap.nonce(),
            to_u64(burn_pool_token_amount)?,
        )?;

        match trade_direction {
            TradeDirection::AtoB => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    swap_token_a_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.nonce(),
                    destination_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                Self::token_transfer(
                    swap_info.key,
                    token_program_info.clone(),
                    swap_token_b_info.clone(),
                    destination_info.clone(),
                    authority_info.clone(),
                    token_swap.nonce(),
                    destination_token_amount,
                )?;
            }
        }

        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        Self::process_with_constraints(program_id, accounts, input, &SWAP_CONSTRAINTS)
    }

    /// Processes an instruction given extra constraint
    pub fn process_with_constraints(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
        swap_constraints: &Option<SwapConstraints>,
    ) -> ProgramResult {
        msg!("unpack start");
        let instruction = SwapInstruction::unpack(input)?;
        msg!("unpack end");
        match instruction {
            SwapInstruction::Initialize(Initialize {
                nonce,
                fees,
                swap_curve,
            }) => {
                msg!("Instruction: Init");
                Self::process_initialize(
                    program_id,
                    nonce,
                    fees,
                    swap_curve,
                    accounts,
                    swap_constraints,
                )
            }
            SwapInstruction::Swap(Swap {
                amount_in,
                minimum_amount_out,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, accounts)
            }
            SwapInstruction::DepositAllTokenTypes(DepositAllTokenTypes {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            }) => {
                msg!("Instruction: DepositAllTokenTypes");
                Self::process_deposit_all_token_types(
                    program_id,
                    pool_token_amount,
                    maximum_token_a_amount,
                    maximum_token_b_amount,
                    accounts,
                )
            }
            SwapInstruction::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            }) => {
                msg!("Instruction: WithdrawAllTokenTypes");
                Self::process_withdraw_all_token_types(
                    program_id,
                    pool_token_amount,
                    minimum_token_a_amount,
                    minimum_token_b_amount,
                    accounts,
                )
            }
            SwapInstruction::DepositSingleTokenTypeExactAmountIn(
                DepositSingleTokenTypeExactAmountIn {
                    source_token_amount,
                    minimum_pool_token_amount,
                },
            ) => {
                msg!("Instruction: DepositSingleTokenTypeExactAmountIn");
                Self::process_deposit_single_token_type_exact_amount_in(
                    program_id,
                    source_token_amount,
                    minimum_pool_token_amount,
                    accounts,
                )
            }
            SwapInstruction::WithdrawSingleTokenTypeExactAmountOut(
                WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount,
                    maximum_pool_token_amount,
                },
            ) => {
                msg!("Instruction: WithdrawSingleTokenTypeExactAmountOut");
                Self::process_withdraw_single_token_type_exact_amount_out(
                    program_id,
                    destination_token_amount,
                    maximum_pool_token_amount,
                    accounts,
                )
            }
        }
    }
}

impl PrintProgramError for SwapError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            SwapError::AlreadyInUse => msg!("Error: Swap account already in use"),
            SwapError::InvalidProgramAddress => {
                msg!("Error: Invalid program address generated from nonce and key")
            }
            SwapError::InvalidOwner => {
                msg!("Error: The input account owner is not the program address")
            }
            SwapError::InvalidOutputOwner => {
                msg!("Error: Output pool account owner cannot be the program address")
            }
            SwapError::ExpectedMint => msg!("Error: Deserialized account is not an SPL Token mint"),
            SwapError::ExpectedAccount => {
                msg!("Error: Deserialized account is not an SPL Token account")
            }
            SwapError::EmptySupply => msg!("Error: Input token account empty"),
            SwapError::InvalidSupply => msg!("Error: Pool token mint has a non-zero supply"),
            SwapError::RepeatedMint => msg!("Error: Swap input token accounts have the same mint"),
            SwapError::InvalidDelegate => msg!("Error: Token account has a delegate"),
            SwapError::InvalidInput => msg!("Error: InvalidInput"),
            SwapError::IncorrectSwapAccount => {
                msg!("Error: Address of the provided swap token account is incorrect")
            }
            SwapError::IncorrectPoolMint => {
                msg!("Error: Address of the provided pool token mint is incorrect")
            }
            SwapError::InvalidOutput => msg!("Error: InvalidOutput"),
            SwapError::CalculationFailure => msg!("Error: CalculationFailure"),
            SwapError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            SwapError::ExceededSlippage => {
                msg!("Error: Swap instruction exceeds desired slippage limit")
            }
            SwapError::InvalidCloseAuthority => msg!("Error: Token account has a close authority"),
            SwapError::InvalidFreezeAuthority => {
                msg!("Error: Pool token mint has a freeze authority")
            }
            SwapError::IncorrectFeeAccount => msg!("Error: Pool fee token account incorrect"),
            SwapError::ZeroTradingTokens => {
                msg!("Error: Given pool token amount results in zero trading tokens")
            }
            SwapError::FeeCalculationFailure => msg!(
                "Error: The fee calculation failed due to overflow, underflow, or unexpected 0"
            ),
            SwapError::ConversionFailure => msg!("Error: Conversion to or from u64 failed."),
            SwapError::InvalidFee => {
                msg!("Error: The provided fee does not match the program owner's constraints")
            }
            SwapError::IncorrectTokenProgramId => {
                msg!("Error: The provided token program does not match the token program expected by the swap")
            }
            SwapError::UnsupportedCurveType => {
                msg!("Error: The provided curve type is not supported by the program owner")
            }
            SwapError::InvalidCurve => {
                msg!("Error: The provided curve parameters are invalid")
            }
            SwapError::UnsupportedCurveOperation => {
                msg!("Error: The operation cannot be performed on the given curve")
            }
        }
    }
}

fn to_u128(val: u64) -> Result<u128, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}

fn to_u64(val: u128) -> Result<u64, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}