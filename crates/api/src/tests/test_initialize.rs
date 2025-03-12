use std::sync::Arc;

use crate::{
    router,
    tests::BlinkTestClient,
    types::{ApiResponse, InitializeOrApply},
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
async fn test_initialize() {
    let key = test_key();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc).await;

    test_client
        .rpc
        .request_airdrop(&key.pubkey(), 100_000_000_000)
        .await
        .unwrap();

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, &mint).await;
}
