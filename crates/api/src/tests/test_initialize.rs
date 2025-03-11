use std::sync::Arc;

use crate::{
    router,
    tests::create_confidential_mint,
    types::{Deposit, Initialize},
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

struct BlinkTestClient {
    rpc: Arc<RpcClient>,
    server: TestServer,
}

impl BlinkTestClient {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self {
            rpc: rpc.clone(),
            server: TestServer::new(router::new(rpc)).unwrap(),
        }
    }
    async fn test_initialize(&mut self, key: &Keypair, mint: &Keypair) {
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let init = Initialize {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/initialize")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&init).unwrap().into())
            .await;

        self.send_tx(key, res).await;
    }
    async fn test_deposit(&mut self, key: &Keypair, mint: &Keypair) {
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let deposit = Deposit {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            deposit_amount: 100_000,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/deposit")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&deposit).unwrap().into())
            .await;

        self.send_tx(key, res).await;
    }
    async fn create_confidential_mint(&mut self, key: &Keypair, mint: &Keypair) {
        create_confidential_mint(&self.rpc, &mint, &key).await;
    }
    async fn mint_tokens(&mut self, key: &Keypair, mint: &Keypair) {
        let mut tx = Transaction::new_with_payer(
            &[spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &mint.pubkey(),
                &get_user_ata(key, mint),
                &key.pubkey(),
                &[&key.pubkey()],
                MINT_AMOUNT,
            )
            .unwrap()],
            Some(&key.pubkey()),
        );
        tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
    async fn send_tx(&mut self, key: &Keypair, res: TestResponse) {
        let body_str: String = serde_json::from_slice(res.as_bytes()).unwrap();
        let mut tx: Transaction =
            bincode::deserialize(&BASE64_STANDARD.decode(body_str).unwrap()).unwrap();

        tx.sign(&vec![&key], self.rpc.get_latest_blockhash().await.unwrap());

        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_initialize() {
    let key = test_key();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new_with_commitment(
        "http://localhost:8899".to_string(),
        CommitmentConfig::confirmed(),
    ));

    let mut test_client = BlinkTestClient::new(rpc);

    test_client
        .rpc
        .request_airdrop(&key.pubkey(), 100_000_000_000)
        .await
        .unwrap();

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, &mint).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_deposit() {
    let key = test_key();
    let mint = Keypair::new();
    let rpc = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

    let mut test_client = BlinkTestClient::new(rpc);

    test_client.create_confidential_mint(&key, &mint).await;

    test_client.test_initialize(&key, &mint).await;

    test_client.mint_tokens(&key, &mint).await;

    assert_eq!(
        test_client
            .rpc
            .get_token_account_balance(&get_user_ata(&key, &mint))
            .await
            .unwrap()
            .ui_amount
            .unwrap(),
        100.0
    );

    test_client.test_deposit(&key, &mint).await;
}
