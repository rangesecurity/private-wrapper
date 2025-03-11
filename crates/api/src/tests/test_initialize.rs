use std::sync::Arc;

use crate::{router, tests::create_confidential_mint, types::Initialize};
use axum::body::Body;
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

#[tokio::test]
async fn test_initialize() {
    let key = test_key();
    let mint = Keypair::new();
    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    );
    let rpc = Arc::new(RpcClient::new_with_commitment(
        "http://localhost:8899".to_string(),
        CommitmentConfig::confirmed(),
    ));

    rpc.request_airdrop(&key.pubkey(), 100_000_000_000)
        .await
        .unwrap();

    create_confidential_mint(&rpc, &mint, &key).await;

    let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
    let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));
    let elgamal_key = derive_elgamal_key(elgamal_sig).unwrap();
    let ae_key = derive_ae_key(ae_sig).unwrap();

    let init = Initialize {
        authority: key.pubkey(),
        token_mint: mint.pubkey(),
        elgamal_signature: elgamal_sig,
        ae_signature: ae_sig,
    };

    let router = router::new(rpc.clone());

    let res = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/confidential-balances/initialize")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&init).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let body_str: String = serde_json::from_slice(&body[..]).unwrap();
    let mut tx: Transaction =
        bincode::deserialize(&BASE64_STANDARD.decode(body_str).unwrap()).unwrap();

    tx.sign(&vec![&key], rpc.get_latest_blockhash().await.unwrap());

    rpc.send_and_confirm_transaction(&tx).await.unwrap();
}
