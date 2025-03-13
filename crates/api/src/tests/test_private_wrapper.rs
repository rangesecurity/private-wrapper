use {
    super::get_user_ata, crate::tests::BlinkTestClient, common::test_helpers::test_key,
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::{signature::Keypair, signer::Signer},
    std::sync::Arc,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_wrap() {
    let key = test_key();
    let unwrapped_mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc).await;

    test_client.create_mint(&key, &unwrapped_mint).await;

    let wrapped_mint = test_client.create_confidential_wrapped_mint(&key, &unwrapped_mint).await;

    // create unwrapped token account
    test_client.create_token_account(&key, &unwrapped_mint).await;

    // create wrapped token account
    test_client.test_initialize(&key, wrapped_mint).await;

    // mint unwrapped tokens
    test_client.mint_tokens(
        &key,
        unwrapped_mint.pubkey(),
        spl_token_2022::ui_amount_to_amount(100.0, 6)
    ).await;

    // wrap tokens (this will become a non confidential balance)
    test_client.test_wrap_tokens(
        &key,
        &unwrapped_mint,
        wrapped_mint,
        spl_token_2022::ui_amount_to_amount(1.0, 6),
    ).await;

    // deposit a portion of the non confidential balance
    test_client.test_deposit(
        &key,
        wrapped_mint,
        spl_token_2022::ui_amount_to_amount(0.5, 6),
    ).await;

    // apply the pending balance
    test_client.test_apply(
        &key,
        wrapped_mint
    ).await;

    // unwrap a portio of the non confidential balance
    test_client.test_unwrap_tokens(
        &key,
        &unwrapped_mint,
        wrapped_mint,
        spl_token_2022::ui_amount_to_amount(0.25, 6)
    ).await;
    // deposit a portion of the non confidential balance
    test_client.test_deposit(
        &key,
        wrapped_mint,
        spl_token_2022::ui_amount_to_amount(0.1, 6),
    ).await;

    let balances = test_client.get_balances(&key, wrapped_mint).await;
    assert_eq!(balances.pending_balance, 0.1);
    assert_eq!(balances.available_balance, 0.5);
    assert_eq!(balances.non_confidential_balance, 0.15);
}
