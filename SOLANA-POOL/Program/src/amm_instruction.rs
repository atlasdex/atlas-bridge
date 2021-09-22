//! Instruction types

#![allow(clippy::too_many_arguments)]

use crate::curve::{base::SwapCurve, fees::Fees};
use crate::error::AmmError;
use solana_program::{
	instruction::{AccountMeta, Instruction},
	program_error::ProgramError,
	program_pack::Pack,
	pubkey::Pubkey,
}
use std::convert::TryInto;
use std::mem::size_of;