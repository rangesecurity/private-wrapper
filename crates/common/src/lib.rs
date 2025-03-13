//! Common utilities for working with the confidential blink specification

pub mod accounts;
pub mod key_generator;
pub mod proofs;

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;


pub const PRIVATE_WRAPPER_ID: solana_sdk::pubkey::Pubkey = solana_sdk::pubkey!("PtwjzDzqbJr41iHYy8KG3Jb8VcwgRagJWj9Gk3Jg9f9");