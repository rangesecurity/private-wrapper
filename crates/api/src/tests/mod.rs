use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use solana_transaction_status_client_types::UiTransactionEncoding;
use spl_token_2022::{extension::ExtensionType, state::Mint};
use spl_token_client::token::ExtensionInitializationParams;
use std::sync::Arc;

use crate::{
    router,
    types::{ApiResponse, Deposit, InitializeOrApply, Withdraw},
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
use tower::ServiceExt;

pub mod test_deposit;
pub mod test_initialize;
pub mod test_withdraw;

/// 100.0 with 9 decimals
pub const MINT_AMOUNT: u64 = 100000000000;

struct BlinkTestClient {
    rpc: Arc<RpcClient>,
    server: TestServer,
}

impl BlinkTestClient {
    pub async fn new(rpc: Arc<RpcClient>) -> Self {
        // seed the test key with SOL
        {
            let test_key = test_key().pubkey();
            let balance = rpc.get_balance(&test_key).await.unwrap();
            rpc.request_airdrop(&test_key, spl_token_2022::ui_amount_to_amount(100.0, 9))
                .await
                .unwrap();
            loop {
                let new_balance = rpc.get_balance(&test_key).await.unwrap();
                if new_balance > balance {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        Self {
            rpc: rpc.clone(),
            server: TestServer::new(router::new(rpc)).unwrap(),
        }
    }
    async fn test_initialize(&mut self, key: &Keypair, mint: &Keypair) {
        println!("initializing confidential token account");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let init = InitializeOrApply {
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
        let response: ApiResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_deposit(&mut self, key: &Keypair, mint: &Keypair, amount: u64) {
        println!("depositing to pending balance");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let deposit = Deposit {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            amount,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/deposit")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&deposit).unwrap().into())
            .await;
        let response: ApiResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_apply(&mut self, key: &Keypair, mint: &Keypair) {
        println!("applying pending balance");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let deposit = InitializeOrApply {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/apply")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&deposit).unwrap().into())
            .await;
        let response: ApiResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_withdraw(&mut self, key: &Keypair, mint: &Keypair, amount: u64) {
        println!("withdrawing confidential tokens");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let equality_proof_keypair = Keypair::new();
        let range_proof_keypair = Keypair::new();

        let withdraw = Withdraw {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            amount,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
            equality_proof_keypair: equality_proof_keypair.to_base58_string(),
            range_proof_keypair: range_proof_keypair.to_base58_string(),
        };
        let res = self
            .server
            .post("/confidential-balances/withdraw")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&withdraw).unwrap().into())
            .await;
        let res = String::from_utf8(res.as_bytes().to_vec()).unwrap();
        let response: ApiResponse = serde_json::from_str(&res).unwrap();
        let txs = response.decode_transactions().unwrap();
        // we cant use the send_tx helper here as we need to sign with equality + range proofs
        for (idx, mut tx) in txs.into_iter().enumerate() {
            if idx == 0 {
                println!("signing tx0");
                tx.sign(
                    &vec![key, &equality_proof_keypair, &range_proof_keypair],
                    self.rpc.get_latest_blockhash().await.unwrap(),
                );
            } else if idx == 1 {
                println!("signing tx1");
                tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
            } else if idx == 2 {
                println!("signing tx2");
                tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
            } else if idx == 3 {
                println!("signing tx3");
                tx.sign(
                    &vec![key],
                    self.rpc.get_latest_blockhash().await.unwrap(),
                );
            }
            println!("sending tx({idx})");
            self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
        }
    }
    async fn create_confidential_mint(&mut self, key: &Keypair, mint: &Keypair) {
        println!("creating confidential mint");
        let space = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::ConfidentialTransferMint,
        ])
        .unwrap();
        let rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(space)
            .await
            .unwrap();

        let create_account_ix = system_instruction::create_account(
            &key.pubkey(),
            &mint.pubkey(),
            rent,
            space as u64,
            &spl_token_2022::id(),
        );

        let extension_init_ix = ExtensionInitializationParams::ConfidentialTransferMint {
            authority: Some(key.pubkey()),
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
                    &key.pubkey(),
                    None,
                    6,
                )
                .unwrap(),
            ],
            Some(&key.pubkey()),
        );
        tx.sign(
            &vec![key, mint],
            self.rpc.get_latest_blockhash().await.unwrap(),
        );

        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
    async fn mint_tokens(&mut self, key: &Keypair, mint: &Keypair, amount: u64) {
        let mut tx = Transaction::new_with_payer(
            &[spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &mint.pubkey(),
                &get_user_ata(key, mint),
                &key.pubkey(),
                &[&key.pubkey()],
                amount,
            )
            .unwrap()],
            Some(&key.pubkey()),
        );
        tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
    async fn send_tx(&mut self, key: &Keypair, res: ApiResponse) {
        let transactions = res.decode_transactions().unwrap();
        for mut tx in transactions {
            tx.sign(&vec![&key], self.rpc.get_latest_blockhash().await.unwrap());

            let sig = self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
            // ensure the tx was confirmed
            let _ = self.rpc
                    .get_transaction_with_config(
                        &sig,
                        RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::JsonParsed),
                            max_supported_transaction_version: Some(1),
                            ..Default::default()
                        }
                    )
                    .await
                    .unwrap();
        }
    }
}

pub fn get_user_ata(key: &Keypair, mint: &Keypair) -> Pubkey {
    spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    )
}
