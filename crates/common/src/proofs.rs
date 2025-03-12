//! Utilities for generating confidential transfer proofs

use {
    solana_sdk::{pubkey::Pubkey, system_instruction},
    solana_zk_sdk::zk_elgamal_proof_program::{self, instruction::ContextStateInfo},
    spl_token_confidential_transfer_proof_extraction::instruction::zk_proof_type_to_instruction,
    std::mem::size_of,
};

/// Refactored version of spl_token_client::token::Token::confidential_transfer_create_context_state_account().
/// Instead of sending transactions internally or calculating rent via RPC, this function now accepts
/// the rent value from the caller and returns the instructions to be used externally.
pub fn get_zk_proof_context_state_account_creation_instructions<
    ZK: bytemuck::Pod + zk_elgamal_proof_program::proof_data::ZkProofData<U>,
    U: bytemuck::Pod,
>(
    fee_payer_pubkey: &Pubkey,
    context_state_account_pubkey: &Pubkey,
    context_state_authority_pubkey: &Pubkey,
    proof_data: &ZK,
    rent: u64,
) -> anyhow::Result<(
    solana_sdk::instruction::Instruction,
    solana_sdk::instruction::Instruction,
)> {
    let space = size_of::<zk_elgamal_proof_program::state::ProofContextState<U>>();
    println!("📊 Context state account space required: {} bytes", space);
    println!(
        "💰 Using provided rent for context state account: {} lamports",
        rent
    );

    let context_state_info = ContextStateInfo {
        context_state_account: context_state_account_pubkey,
        context_state_authority: context_state_authority_pubkey,
    };

    let instruction_type = zk_proof_type_to_instruction(ZK::PROOF_TYPE)?;

    println!("🔧 Creating context state account with inputs: fee_payer={}, context_state_account={}, rent={}, space={}", 
        fee_payer_pubkey, context_state_account_pubkey, rent, space);
    let create_account_ix = system_instruction::create_account(
        fee_payer_pubkey,
        context_state_account_pubkey,
        rent,
        space as u64,
        &zk_elgamal_proof_program::id(),
    );

    let verify_proof_ix =
        instruction_type.encode_verify_proof(Some(context_state_info), proof_data);

    // Return a tuple containing the create account instruction and verify proof instruction.
    Ok((create_account_ix, verify_proof_ix))
}
