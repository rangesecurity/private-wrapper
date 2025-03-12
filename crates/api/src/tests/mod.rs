use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{extension::ExtensionType, state::Mint};
use spl_token_client::token::ExtensionInitializationParams;
use std::sync::Arc;

use crate::{
    router,
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
use tower::ServiceExt;

pub mod test_deposit;
pub mod test_initialize;

/// 100.0 with 9 decimals
pub const MINT_AMOUNT: u64 = 100000000000;

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
        let response: ApiResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_deposit(&mut self, key: &Keypair, mint: &Keypair) {
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let deposit = DepositOrWithdraw {
            authority: key.pubkey(),
            token_mint: mint.pubkey(),
            amount: 100_000,
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
    async fn create_confidential_mint(&mut self, key: &Keypair, mint: &Keypair) {
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
                    9,
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
    async fn send_tx(&mut self, key: &Keypair, res: ApiResponse) {
        let transactions = res.decode_transactions().unwrap();
        for mut tx in transactions {
            tx.sign(&vec![&key], self.rpc.get_latest_blockhash().await.unwrap());

            self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
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
