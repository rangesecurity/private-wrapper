use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{extension::ExtensionType, state::Mint};
use spl_token_client::token::ExtensionInitializationParams;

pub mod test_initialize;

/// 100.0 with 9 decimals
pub const MINT_AMOUNT: u64 = 100000000000;

/// Creates a token mint with the ConfidentialTransferMint extension
pub async fn create_confidential_mint(rpc: &RpcClient, mint: &Keypair, authority: &Keypair) {
    let space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::ConfidentialTransferMint,
    ])
    .unwrap();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(space)
        .await
        .unwrap();

    let create_account_ix = system_instruction::create_account(
        &authority.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token_2022::id(),
    );

    let extension_init_ix = ExtensionInitializationParams::ConfidentialTransferMint {
        authority: Some(authority.pubkey()),
        auto_approve_new_accounts: true,
        auditor_elgamal_pubkey: None,
    }
    .instruction(&spl_token_2022::id(), &mint.pubkey())
    .unwrap();

    let mut tx = Transaction::new_with_payer(
        &[
            create_account_ix,
            extension_init_ix,
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::id(),
                &mint.pubkey(),
                &authority.pubkey(),
                None,
                9,
            )
            .unwrap(),
        ],
        Some(&authority.pubkey()),
    );
    tx.sign(
        &vec![authority, mint],
        rpc.get_latest_blockhash().await.unwrap(),
    );

    rpc.send_and_confirm_transaction(&tx).await.unwrap();
}

pub fn get_user_ata(key: &Keypair, mint: &Keypair) -> Pubkey {
    spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    )
}
