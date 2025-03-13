use {
    super::get_user_ata, crate::tests::BlinkTestClient, common::test_helpers::test_key,
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::signature::Keypair,
    std::sync::Arc,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_deposit() {
    let key = test_key();
    let unwrapped_mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc).await;

    test_client.create_mint(&key, &unwrapped_mint).await;

    let wrapped_mint = test_client.create_confidential_wrapped_mint(&key, &unwrapped_mint).await;

    // create unwrapped token account
    test_client.create_token_account(&key, &unwrapped_mint).await;

    // mint unwrapped tokens
    test_client.mint_tokens(
        &key,
        &unwrapped_mint,
        spl_token_2022::ui_amount_to_amount(100.0, 6)
    ).await;
}
