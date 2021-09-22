#![deny(missing_docs)]

//! An Uniswap-like program for the Solana blockchain.

pub mod amm_instruction;
pub mod amm_stats;
pub mod constraints;
pub mod curve;
pub mod error;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8");
