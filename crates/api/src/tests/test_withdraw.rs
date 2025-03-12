use std::sync::Arc;

use crate::{
    router,
    tests::BlinkTestClient,
    types::{ApiResponse, DepositOrWithdraw, Initialize},
};
use axum::body::{Body, Bytes};
use axum_test::{TestResponse, TestServer};
use base64::{prelude::BASE64_STANDARD, Engine};
use common::{
    key_generator::{derive_ae_key, derive_elgamal_key, KeypairType},
    test_helpers::test_key,
};
use http::Request;
use http_body_util::BodyExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use tower::ServiceExt;

use super::{get_user_ata, MINT_AMOUNT};

#[tokio::test(flavor = "multi_thread")]
async fn test_withdraw() {
    let key = test_key();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));
    let balance = rpc.get_balance(&key.pubkey()).await.unwrap();
    rpc.request_airdrop(&key.pubkey(), spl_token_2022::ui_amount_to_amount(10000.0, 9)).await.unwrap();
    // running into issues with the test validator not confirming fast enough
    loop {
        let new_balance = rpc.get_balance(&key.pubkey()).await.unwrap();
        if new_balance > balance {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    let mut test_client = BlinkTestClient::new(rpc);

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, &mint).await;
    
    test_client.mint_tokens(&key, &mint, 1_000_000).await;

    assert_eq!(
        test_client
            .rpc
            .get_token_account_balance(&get_user_ata(&key, &mint))
            .await
            .unwrap()
            .amount.parse::<u64>()
            .unwrap(),
        1_000_000
    );

    test_client.test_deposit(&key, &mint, 200).await;
    test_client.test_withdraw(&key, &mint, 100).await;
}
