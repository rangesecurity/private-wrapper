use {
    crate::tests::BlinkTestClient, common::test_helpers::test_key,
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::{signature::Keypair, signer::Signer},
    std::sync::Arc,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_initialize() {
    let key = test_key();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc).await;

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, mint.pubkey()).await;
}
