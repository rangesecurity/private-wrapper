use {
    crate::{
        router,
        types::{
            ApiBalancesResponse, ApiTransactionResponse, Balances, Deposit, InitializeOrApply,
            Transfer, Withdraw, WrapTokens,
        },
    },
    axum_test::TestServer,
    common::{key_generator::KeypairType, test_helpers::test_key},
    solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig},
    solana_sdk::{
        program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
        transaction::Transaction,
    },
    solana_transaction_status_client_types::UiTransactionEncoding,
    spl_token_2022::{extension::ExtensionType, state::Mint},
    spl_token_client::token::ExtensionInitializationParams,
    spl_token_wrap::{
        get_wrapped_mint_address, get_wrapped_mint_authority, get_wrapped_mint_backpointer_address,
        state::Backpointer,
    },
    std::sync::Arc,
};

pub mod test_deposit;
pub mod test_initialize;
pub mod test_private_wrapper;
pub mod test_transfer;
pub mod test_withdraw;

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
    async fn test_initialize(&mut self, key: &Keypair, mint: Pubkey) {
        println!("initializing confidential token account");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let init = InitializeOrApply {
            authority: key.pubkey(),
            token_mint: mint,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/initialize")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&init).unwrap().into())
            .await;
        let response: ApiTransactionResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_deposit(&mut self, key: &Keypair, mint: Pubkey, amount: u64) {
        println!("depositing to pending balance");

        let deposit = Deposit {
            authority: key.pubkey(),
            token_mint: mint,
            amount,
        };
        let res = self
            .server
            .post("/confidential-balances/deposit")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&deposit).unwrap().into())
            .await;
        let response: ApiTransactionResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_wrap_tokens(
        &mut self,
        key: &Keypair,
        unwrapped_mint: &Keypair,
        wrapped_mint: Pubkey,
        amount: u64,
    ) {
        println!("privately wrapping tokens");

        let wrap = WrapTokens {
            authority: key.pubkey(),
            unwrapped_token_mint: unwrapped_mint.pubkey(),
            unwrapped_token_program: spl_token_2022::id(),
            wrapped_token_mint: wrapped_mint,
            amount,
        };
        let res = self
            .server
            .post("/private-wrapper/wrap")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&wrap).unwrap().into())
            .await;
        let res = String::from_utf8(res.as_bytes().to_vec()).unwrap();
        let response: ApiTransactionResponse = serde_json::from_str(&res).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_unwrap_tokens(
        &mut self,
        key: &Keypair,
        unwrapped_mint: &Keypair,
        wrapped_mint: Pubkey,
        amount: u64,
    ) {
        println!("privately wrapping tokens");

        let wrap = WrapTokens {
            authority: key.pubkey(),
            unwrapped_token_mint: unwrapped_mint.pubkey(),
            unwrapped_token_program: spl_token_2022::id(),
            wrapped_token_mint: wrapped_mint,
            amount,
        };
        let res = self
            .server
            .post("/private-wrapper/unwrap")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&wrap).unwrap().into())
            .await;
        let res = String::from_utf8(res.as_bytes().to_vec()).unwrap();
        let response: ApiTransactionResponse = serde_json::from_str(&res).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_apply(&mut self, key: &Keypair, mint: Pubkey) {
        println!("applying pending balance");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let deposit = InitializeOrApply {
            authority: key.pubkey(),
            token_mint: mint,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/apply")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&deposit).unwrap().into())
            .await;
        let response: ApiTransactionResponse = serde_json::from_slice(res.as_bytes()).unwrap();
        self.send_tx(key, response).await;
    }
    async fn test_withdraw(&mut self, key: &Keypair, mint: Pubkey, amount: u64) {
        println!("withdrawing confidential tokens");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let equality_proof_keypair = Keypair::new();
        let range_proof_keypair = Keypair::new();

        let withdraw = Withdraw {
            authority: key.pubkey(),
            token_mint: mint,
            amount,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
            equality_proof_keypair: equality_proof_keypair.insecure_clone(),
            range_proof_keypair: range_proof_keypair.insecure_clone(),
        };
        let res = self
            .server
            .post("/confidential-balances/withdraw")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&withdraw).unwrap().into())
            .await;
        let res = String::from_utf8(res.as_bytes().to_vec()).unwrap();
        let response: ApiTransactionResponse = serde_json::from_str(&res).unwrap();
        let txs = response.decode_transactions().unwrap();
        // we cant use the send_tx helper here as we need to sign with equality + range proofs
        for (idx, mut tx) in txs.into_iter().enumerate() {
            if idx == 0 {
                tx.sign(
                    &vec![key, &equality_proof_keypair, &range_proof_keypair],
                    self.rpc.get_latest_blockhash().await.unwrap(),
                );
            } else {
                tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
            }
            self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
        }
    }
    async fn test_transfer(
        &mut self,
        key: &Keypair,
        mint: Pubkey,
        receipient: &Keypair,
        amount: u64,
    ) {
        println!("transferring confidential tokens");
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let equality_proof_keypair = Keypair::new();
        let range_proof_keypair = Keypair::new();
        let ciphertext_proof_keypair = Keypair::new();

        let withdraw = Transfer {
            authority: key.pubkey(),
            token_mint: mint,
            amount,
            receiving_token_account:
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &receipient.pubkey(),
                    &mint,
                    &spl_token_2022::id(),
                ),
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
            equality_proof_keypair: equality_proof_keypair.insecure_clone(),
            range_proof_keypair: range_proof_keypair.insecure_clone(),
            ciphertext_validity_proof_keypair: ciphertext_proof_keypair.insecure_clone(),
        };
        let res = self
            .server
            .post("/confidential-balances/transfer")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&withdraw).unwrap().into())
            .await;
        let res = String::from_utf8(res.as_bytes().to_vec()).unwrap();
        let response: ApiTransactionResponse = serde_json::from_str(&res).unwrap();
        let txs = response.decode_transactions().unwrap();
        // we cant use the send_tx helper here as we need to sign with equality + range proofs
        for (idx, mut tx) in txs.into_iter().enumerate() {
            if idx == 0 {
                tx.sign(
                    &vec![
                        key,
                        &equality_proof_keypair,
                        &range_proof_keypair,
                        &ciphertext_proof_keypair,
                    ],
                    self.rpc.get_latest_blockhash().await.unwrap(),
                );
            } else {
                tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
            }
            self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
        }
    }
    async fn get_balances(&mut self, key: &Keypair, mint: Pubkey) -> ApiBalancesResponse {
        let user_ata = get_user_ata(key, mint);
        let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
        let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

        let balances = Balances {
            authority: key.pubkey(),
            token_mint: mint,
            elgamal_signature: elgamal_sig,
            ae_signature: ae_sig,
        };
        let res = self
            .server
            .post("/confidential-balances/balances")
            .add_header("Content-Type", "application/json")
            .bytes(serde_json::to_string(&balances).unwrap().into())
            .await;
        serde_json::from_slice(res.as_bytes()).unwrap()
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
    async fn create_mint(&mut self, key: &Keypair, mint: &Keypair) {
        let create_mint_ix = system_instruction::create_account(
            &key.pubkey(),
            &mint.pubkey(),
            self.rpc
                .get_minimum_balance_for_rent_exemption(spl_token_2022::state::Mint::LEN)
                .await
                .unwrap(),
            spl_token_2022::state::Mint::LEN as u64,
            &spl_token_2022::id(),
        );
        let init_mint_ix = spl_token_2022::instruction::initialize_mint2(
            &spl_token_2022::id(),
            &mint.pubkey(),
            &key.pubkey(),
            None,
            6,
        )
        .unwrap();
        let mut tx =
            Transaction::new_with_payer(&[create_mint_ix, init_mint_ix], Some(&key.pubkey()));
        tx.sign(
            &vec![key, mint],
            self.rpc.get_latest_blockhash().await.unwrap(),
        );
        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
    // returns the address of the wrapped mint
    async fn create_confidential_wrapped_mint(
        &mut self,
        key: &Keypair,
        unwrapped_mint: &Keypair,
    ) -> Pubkey {
        println!("creating wrapped confidential mint");

        let wrapped_mint_address =
            get_wrapped_mint_address(&unwrapped_mint.pubkey(), &spl_token_2022::id());

        let backpoint_rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(std::mem::size_of::<Backpointer>())
            .await
            .unwrap();
        let mint_size = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
            ExtensionType::ConfidentialTransferMint,
        ])
        .unwrap();
        let mint_rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(mint_size)
            .await
            .unwrap();

        let ix = spl_token_wrap::instruction::create_confidential_mint(
            &spl_token_wrap::ID,
            &wrapped_mint_address,
            &get_wrapped_mint_backpointer_address(&wrapped_mint_address),
            &unwrapped_mint.pubkey(),
            &spl_token_2022::id(),
            true,
            true,
            [0u8; 32],
            [0u8; 32],
        );
        let mut ixs = vec![];
        ixs.append(&mut system_instruction::transfer_many(
            &key.pubkey(),
            &[
                (
                    get_wrapped_mint_backpointer_address(&wrapped_mint_address),
                    backpoint_rent,
                ),
                (wrapped_mint_address, mint_rent),
            ],
        ));
        ixs.push(ix);
        let mut tx = Transaction::new_with_payer(&ixs, Some(&key.pubkey()));
        tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());

        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();

        // create the escrow account
        let ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &key.pubkey(),
                &get_wrapped_mint_authority(&wrapped_mint_address),
                &unwrapped_mint.pubkey(),
                &spl_token_2022::id(),
            );
        let mut tx = Transaction::new_with_payer(&[ix], Some(&key.pubkey()));
        tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
        wrapped_mint_address
    }
    async fn mint_tokens(&mut self, key: &Keypair, mint: Pubkey, amount: u64) {
        let mut tx = Transaction::new_with_payer(
            &[spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &mint,
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
    async fn create_token_account(&mut self, key: &Keypair, mint: &Keypair) {
        let ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &key.pubkey(),
                &key.pubkey(),
                &mint.pubkey(),
                &spl_token_2022::id(),
            );
        let mut tx = Transaction::new_with_payer(&[ix], Some(&key.pubkey()));
        tx.sign(&vec![key], self.rpc.get_latest_blockhash().await.unwrap());
        self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
    }
    async fn send_tx(&mut self, key: &Keypair, res: ApiTransactionResponse) {
        let transactions = res.decode_transactions().unwrap();
        for mut tx in transactions {
            tx.sign(&vec![&key], self.rpc.get_latest_blockhash().await.unwrap());

            let sig = self.rpc.send_and_confirm_transaction(&tx).await.unwrap();
            // ensure the tx was confirmed
            let _ = self
                .rpc
                .get_transaction_with_config(
                    &sig,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::JsonParsed),
                        max_supported_transaction_version: Some(1),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
    }
}

pub fn get_user_ata(key: &Keypair, mint: Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &mint,
        &spl_token_2022::id(),
    )
}
