use {
    super::get_user_ata,
    crate::tests::BlinkTestClient,
    common::test_helpers::test_key,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{signature::Keypair, signer::Signer},
    std::sync::Arc,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_transfer() {
    let key = test_key();
    let key2 = Keypair::new();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc).await;

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, mint.pubkey()).await;

    test_client
        .mint_tokens(&key, mint.pubkey(), 1_000_000)
        .await;

    assert_eq!(
        test_client
            .rpc
            .get_token_account_balance(&get_user_ata(&key, mint.pubkey()))
            .await
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap(),
        1_000_000
    );

    // delay the initialization of key2 token account to allow airdrop request to bul confirmed
    test_client.test_initialize(&key2, mint.pubkey()).await;

    test_client.test_deposit(&key, mint.pubkey(), 100).await;
    test_client.test_apply(&key, mint.pubkey()).await;

    test_client
        .test_transfer(&key, mint.pubkey(), &key2, 10)
        .await;

    test_client.test_apply(&key2, mint.pubkey()).await;
}
