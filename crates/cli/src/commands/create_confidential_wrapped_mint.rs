use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token_2022::extension::ExtensionType;
use spl_token_wrap::{
    get_wrapped_mint_address, get_wrapped_mint_authority, get_wrapped_mint_backpointer_address, state::Backpointer
};

pub async fn create_token_mint(
    rpc_url: String,
    keypair: String,
    unwrapped_mint: String,
    unwrapped_mint_program: String,
) -> anyhow::Result<()> {
    let unwrapped_mint: Pubkey = unwrapped_mint.parse().unwrap();
    let unwrapped_mint_program: Pubkey = unwrapped_mint_program.parse().unwrap();
    let rpc = RpcClient::new(rpc_url);
    let key = Keypair::read_from_file(keypair).unwrap();
    let wrapped_mint_address = get_wrapped_mint_address(&unwrapped_mint, &spl_token_2022::id());

    let backpoint_rent = rpc
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<Backpointer>())
        .await
        .unwrap();
    let mint_size = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
        ExtensionType::ConfidentialTransferMint,
    ])
    .unwrap();
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_size)
        .await
        .unwrap();

    let ix = spl_token_wrap::instruction::create_confidential_mint(
        &spl_token_wrap::ID,
        &wrapped_mint_address,
        &get_wrapped_mint_backpointer_address(&wrapped_mint_address),
        &unwrapped_mint,
        &spl_token_2022::id(),
        true,
        true,
        [0u8; 32],
        [0u8; 32],
    );
    let mut ixs = vec![];
    ixs.append(&mut system_instruction::transfer_many(
        &key.pubkey(),
        &[
            (
                get_wrapped_mint_backpointer_address(&wrapped_mint_address),
                backpoint_rent,
            ),
            (wrapped_mint_address, mint_rent),
        ],
    ));
    ixs.push(spl_associated_token_account::instruction::create_associated_token_account(
        &key.pubkey(),
        &get_wrapped_mint_authority(&wrapped_mint_address),
        &unwrapped_mint,
        &unwrapped_mint_program
    ));
    ixs.push(ix);
    let mut tx = Transaction::new_with_payer(&ixs, Some(&key.pubkey()));
    tx.sign(&vec![key], rpc.get_latest_blockhash().await.unwrap());
    
    log::info!("creating confidential wrapped mint");
    let sig = rpc.send_and_confirm_transaction(&tx).await.unwrap();
    log::info!("sent tx {sig}");
    Ok(())
}
