use {
    crate::{
        router::AppState,
        types::{ApiError, ApiResponse, Transfer, Withdraw},
    },
    axum::{extract::State, response::IntoResponse, Json},
    base64::{prelude::BASE64_STANDARD, Engine},
    common::{
        accounts::token_account_already_configured,
        key_generator::{derive_ae_key, derive_elgamal_key, KeypairType},
        proofs::get_zk_proof_context_state_account_creation_instructions,
    },
    http::StatusCode,
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    spl_token_2022::{
        extension::{
            confidential_transfer::{
                account_info::{
                    ApplyPendingBalanceAccountInfo, TransferAccountInfo, WithdrawAccountInfo,
                },
                instruction::{
                    BatchedRangeProofContext, CiphertextCommitmentEqualityProofContext,
                    ProofContextState,
                },
                ConfidentialTransferAccount, ConfidentialTransferMint,
            },
            BaseStateWithExtensions, StateWithExtensions,
        },
        solana_zk_sdk::{
            encryption::{elgamal::ElGamalPubkey, pod::elgamal::PodElGamalPubkey},
            zk_elgamal_proof_program::instruction::{close_context_state, ContextStateInfo},
        },
    },
    spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation,
    spl_token_confidential_transfer_proof_generation::{
        transfer::TransferProofData, withdraw::WithdrawProofData,
    },
    std::sync::Arc,
};

pub async fn transfer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Transfer>,
) -> impl IntoResponse {
    // derive the ATA for the authority + token_mint
    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payload.authority,
        &payload.token_mint,
        &spl_token_2022::id(),
    );

    // verify elgamal signature
    if !payload.elgamal_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::ElGamal.message_to_sign(user_ata),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify elgamal signature".to_string(),
            }),
        )
            .into_response();
    }

    // verify ae signature
    if !payload.ae_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::Ae.message_to_sign(user_ata),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify ae signature".to_string(),
            }),
        )
            .into_response();
    }

    // derive the elgamal keypair
    let Ok(elgamal_key) = derive_elgamal_key(payload.elgamal_signature) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to derive elgamal keypair".to_string(),
            }),
        )
            .into_response();
    };

    // derive the ae keypair
    let Ok(ae_key) = derive_ae_key(payload.ae_signature) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to derive ae keypair".to_string(),
            }),
        )
            .into_response();
    };

    // lookup both the token mint, and ata accounts
    let mut accounts = state
        .rpc
        .get_multiple_accounts(&[
            payload.token_mint,
            user_ata,
            payload.receiving_token_account,
        ])
        .await
        .unwrap_or_default();

    // if less than 3 accounts is returned, this means the rpc call failed
    // if the mint does not exist then `accounts[0] == None`
    // if the user_ata does not exist then `accounts[1] == None`
    // if the receiving ata does not exist then  `accounts[2] == None`
    if accounts.len() < 3 {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to query accounts".to_string(),
            }),
        )
            .into_response();
    };

    // ensure the token mint account exists
    let Some(token_mint) = std::mem::take(&mut accounts[0]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // ensure token account exists
    let Some(sender_token_account) = std::mem::take(&mut accounts[1]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    let Some(receiving_token_account) = std::mem::take(&mut accounts[2]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "receiving token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // optimization note: provide an unpack token account
    // ensure token account is configured for confidential transfers
    if !token_account_already_configured(&sender_token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account is not configured for confidential transfers"
                    .to_string(),
            }),
        )
            .into_response();
    }

    if !token_account_already_configured(&receiving_token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account is not configured for confidential transfers"
                    .to_string(),
            }),
        )
            .into_response();
    }

    // ensure the token mint is valid for confidential transfers
    if !common::accounts::is_valid_mint(&token_mint) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not support confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    // get the token mint decimals
    let mint_account =
        match StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint.data) {
            Ok(mint) => mint,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack token mint {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    let mint_extension = match mint_account.get_extension::<ConfidentialTransferMint>() {
        Ok(mint) => mint,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get confidential transfer mint extension {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let sender_token_account = match StateWithExtensions::<spl_token_2022::state::Account>::unpack(
        &sender_token_account.data,
    ) {
        Ok(token_account) => token_account,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to unpack authority token account {err:#?}"),
                }),
            )
                .into_response()
        }
    };
    let receiving_token_account =
        match StateWithExtensions::<spl_token_2022::state::Account>::unpack(
            &receiving_token_account.data,
        ) {
            Ok(token_account) => token_account,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack receiving token account {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };
    let sender_confidential_transfer_account = match sender_token_account
        .get_extension::<ConfidentialTransferAccount>()
    {
        Ok(confidential_token_account) => confidential_token_account,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get confidential transfer account extension {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let receiving_confidential_transfer_account = match receiving_token_account
        .get_extension::<ConfidentialTransferAccount>(
    ) {
        Ok(confidential_token_account) => confidential_token_account,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get confidential transfer account extension {err:#?}"),
                }),
            )
                .into_response()
        }
    };
    let auditor_pubkey = if let Some(auditor_pubkey) =
        Option::<PodElGamalPubkey>::from(mint_extension.auditor_elgamal_pubkey)
    {
        let Ok(auditor_pubkey) = TryInto::<ElGamalPubkey>::try_into(auditor_pubkey) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: "failed to parse auditor pubkey".to_string(),
                }),
            )
                .into_response();
        };
        Some(&auditor_pubkey)
    } else {
        None
    };

    let Ok(destination_pubkey) =
        TryInto::<ElGamalPubkey>::try_into(receiving_confidential_transfer_account.elgamal_pubkey)
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to parse destiantion pubkey".to_string(),
            }),
        )
            .into_response();
    };

    let sender_transfer_account = TransferAccountInfo::new(sender_confidential_transfer_account);

    let Ok(TransferProofData {
        equality_proof_data,
        ciphertext_validity_proof_data_with_ciphertext,
        range_proof_data,
    }) = sender_transfer_account.generate_split_transfer_proof_data(
        payload.amount,
        &elgamal_key,
        &ae_key,
        &destination_pubkey,
        auditor_pubkey,
    )
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to generate split transfer proof".to_string(),
            }),
        )
            .into_response();
    };

    // Parse rent values for proof account creation
    let equality_proof_rent = request
        .equality_proof_rent
        .parse::<u64>()
        .map_err(|_| AppError::SerializationError)?;
    let ciphertext_validity_proof_rent = request
        .ciphertext_validity_proof_rent
        .parse::<u64>()
        .map_err(|_| AppError::SerializationError)?;
    let range_proof_rent = request
        .range_proof_rent
        .parse::<u64>()
        .map_err(|_| AppError::SerializationError)?;

    // Decode sender token account data from request
    println!("📦 Decoding sender token account data from request");
    let sender_token_account_info = {
        let sender_token_account_data = BASE64_STANDARD.decode(&request.sender_token_account)?;
        StateWithExtensionsOwned::<spl_token_2022::state::Account>::unpack(
            sender_token_account_data,
        )?
    };
    println!(
        "✅ Successfully decoded sender token account data from owner {}",
        sender_token_account_info.base.owner.to_string()
    );

    // Decode recipient token account data from request
    println!("📦 Decoding recipient token account data from request");
    let recipient_token_account_info = {
        let recipient_token_account_data =
            BASE64_STANDARD.decode(&request.recipient_token_account)?;
        StateWithExtensionsOwned::<spl_token_2022::state::Account>::unpack(
            recipient_token_account_data,
        )?
    };
    println!(
        "✅ Successfully decoded recipient token account data from owner {}",
        recipient_token_account_info.base.owner.to_string()
    );

    // Verify that both accounts reference the same mint
    let mint = {
        let sender_mint = sender_token_account_info.base.mint;
        let recipient_mint = recipient_token_account_info.base.mint;

        if sender_token_account_info.base.mint != recipient_token_account_info.base.mint {
            println!(
                "❌ Mint mismatch: sender mint {} does not match recipient mint {}",
                sender_mint.to_string(),
                recipient_mint.to_string()
            );
            return Err(AppError::MintMismatch);
        }

        sender_mint
    };

    // Get the sender token account pubkey
    let sender_ata_authority = sender_token_account_info.base.owner;
    let sender_token_account = get_associated_token_address_with_program_id(
        &sender_ata_authority,
        &mint,
        &spl_token_2022::id(),
    );
    println!(
        "✅ Calculated sender token account address: {}",
        sender_token_account
    );

    // Get the recipient token account address
    let recipient_ata_authority = recipient_token_account_info.base.owner;
    let recipient_token_account = get_associated_token_address_with_program_id(
        &recipient_ata_authority,
        &mint,
        &spl_token_2022::id(),
    );
    println!(
        "✅ Calculated recipient token account address: {}",
        recipient_token_account
    );

    // Must first create 3 accounts to store proofs before transferring tokens
    // This must be done in a separate transactions because the proofs are too large for single transaction:
    // Equality Proof - prove that two ciphertexts encrypt the same value
    // Ciphertext Validity Proof - prove that ciphertexts are properly generated
    // Range Proof - prove that ciphertexts encrypt a value in a specified range (0, u64::MAX)

    // "Authority" for the proof accounts (to close the accounts after the transfer)
    let context_state_authority = &sender_ata_authority;

    // Generate addresses for proof accounts
    let equality_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let range_proof_context_state_account = Keypair::new();

    // ConfidentialTransferAccount extension information needed to create proof data
    let sender_transfer_account_info = {
        let sender_account_extension_data =
            sender_token_account_info.get_extension::<ConfidentialTransferAccount>()?;

        TransferAccountInfo::new(sender_account_extension_data)
    };

    let recipient_elgamal_pubkey: solana_zk_sdk::encryption::elgamal::ElGamalPubkey =
        recipient_token_account_info
            .get_extension::<ConfidentialTransferAccount>()?
            .elgamal_pubkey
            .try_into()?;

    // Get auditor ElGamal pubkey from the mint account data
    let auditor_elgamal_pubkey_option = {
        let mint_account_data = BASE64_STANDARD.decode(&request.mint_token_account)?;

        Option::<solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey>::from(
            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(mint_account_data)?
                .get_extension::<spl_token_2022::extension::confidential_transfer::ConfidentialTransferMint>()?
                .auditor_elgamal_pubkey,
        )
        .map(|pod| pod.try_into())
        .transpose()?
    };

    // Create the ElGamal keypair and AES key for the sender token account
    // Create the sender's ElGamal keypair in a temporary scope
    let sender_elgamal_keypair = {
        println!(
            "🔐 Decoding ElGamal signature: {}",
            request.elgamal_signature
        );
        let decoded_elgamal_signature = BASE64_STANDARD.decode(&request.elgamal_signature)?;
        println!(
            "✅ ElGamal signature base64 decoded, got {} bytes",
            decoded_elgamal_signature.len()
        );

        // Create signature directly from bytes
        let elgamal_signature = Signature::try_from(decoded_elgamal_signature.as_slice())
            .map_err(|_| AppError::SerializationError)?;
        println!("✅ ElGamal signature created successfully");

        ElGamalKeypair::new_from_signature(&elgamal_signature)
            .map_err(|_| AppError::SerializationError)?
    };
    println!("✅ ElGamal keypair created successfully");

    // Create the sender's AES key in a temporary scope
    let sender_aes_key = {
        println!("🔐 Decoding AES signature: {}", request.aes_signature);
        let decoded_aes_signature = BASE64_STANDARD.decode(&request.aes_signature)?;
        println!(
            "✅ AES signature base64 decoded, got {} bytes",
            decoded_aes_signature.len()
        );

        // Create signature directly from bytes
        let aes_signature = Signature::try_from(decoded_aes_signature.as_slice())
            .map_err(|_| AppError::SerializationError)?;
        println!("✅ AES signature created successfully");

        AeKey::new_from_signature(&aes_signature).map_err(|_| AppError::SerializationError)?
    };
    println!("✅ AES key created successfully");

    // Generate proof data
    let TransferProofData {
        equality_proof_data,
        ciphertext_validity_proof_data_with_ciphertext,
        range_proof_data,
    } = sender_transfer_account_info.generate_split_transfer_proof_data(
        transfer_amount_lamports,
        &sender_elgamal_keypair,
        &sender_aes_key,
        &recipient_elgamal_pubkey,
        auditor_elgamal_pubkey_option.as_ref(),
    )?;

    // Create 3 proofs ------------------------------------------------------

    // Range Proof Instructions------------------------------------------------------------------------------
    let (range_create_ix, range_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &sender_ata_authority,
            &range_proof_context_state_account.pubkey(),
            &context_state_authority,
            &range_proof_data,
            range_proof_rent,
        )?;

    // Equality Proof Instructions---------------------------------------------------------------------------
    let (equality_create_ix, equality_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &sender_ata_authority,
            &equality_proof_context_state_account.pubkey(),
            &context_state_authority,
            &equality_proof_data,
            equality_proof_rent,
        )?;

    // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
    let (cv_create_ix, cv_verify_ix) = get_zk_proof_context_state_account_creation_instructions(
        &sender_ata_authority,
        &ciphertext_validity_proof_context_state_account.pubkey(),
        &context_state_authority,
        &ciphertext_validity_proof_data_with_ciphertext.proof_data,
        ciphertext_validity_proof_rent,
    )?;

    // Transact Proofs ------------------------------------------------------------------------------------
    let client_blockhash =
        Hash::from_str(&request.latest_blockhash).map_err(|_| AppError::SerializationError)?;

    // Parse priority fee
    let priority_fee = match request.priority_fee.parse::<u64>() {
        Ok(fee) => fee,
        Err(_) => {
            println!(
                "⚠️ Invalid priority fee format: {}, defaulting to 0",
                request.priority_fee
            );
            0
        }
    };

    // Transaction 1: Allocate all proof accounts at once.
    let tx1 = {
        // Create instructions vector
        let mut instructions = Vec::new();

        // Add priority fee instructions if the fee is greater than 0
        if priority_fee > 0 {
            // Convert lamports to micro-lamports per compute unit
            // For example, 10,000,000 lamports with 200,000 compute units = 50,000 micro-lamports per CU
            let micro_lamports = priority_fee * 1_000_000 / 200_000;

            // Add compute budget program instructions
            let compute_budget_program_id = solana_sdk::compute_budget::id();

            // Set compute unit limit (optional but recommended)
            instructions.push(solana_sdk::instruction::Instruction::new_with_borsh(
                compute_budget_program_id,
                &solana_sdk::compute_budget::ComputeBudgetInstruction::SetComputeUnitLimit(200_000),
                vec![],
            ));

            // Set compute unit price (priority fee)
            instructions.push(solana_sdk::instruction::Instruction::new_with_borsh(
                compute_budget_program_id,
                &solana_sdk::compute_budget::ComputeBudgetInstruction::SetComputeUnitPrice(
                    micro_lamports,
                ),
                vec![],
            ));
        }

        // Add the original instructions
        instructions.push(range_create_ix.clone());
        instructions.push(equality_create_ix.clone());
        instructions.push(cv_create_ix.clone());

        // Rest of the code remains the same...
        let message =
            v0::Message::try_compile(&sender_ata_authority, &instructions, &[], client_blockhash)?;

        // Create a versioned message
        let versioned_message = VersionedMessage::V0(message.clone());

        VersionedTransaction::try_new(
            versioned_message,
            &[
                &NullSigner::new(&sender_ata_authority) as &dyn Signer,
                &range_proof_context_state_account,
                &equality_proof_context_state_account,
                &ciphertext_validity_proof_context_state_account,
            ],
        )?
    };

    // Transaction 2: Encode Range Proof on its own (because it's the largest).
    let tx2 = {
        let message = v0::Message::try_compile(
            &sender_ata_authority,
            &[range_verify_ix],
            &[],
            client_blockhash,
        )?;

        // Create a versioned transaction with a placeholder signature for the sender
        VersionedTransaction {
            // Single placeholder signature for the sender as the fee payer.
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(message),
        }
    };

    // Transaction 3: Encode all remaining proofs.
    let tx3 = {
        let message = v0::Message::try_compile(
            &sender_ata_authority,
            &[equality_verify_ix, cv_verify_ix],
            &[],
            client_blockhash,
        )?;

        // Create a versioned transaction with a placeholder signature for the sender
        VersionedTransaction {
            // Single placeholder signature for the sender
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(message),
        }
    };

    // Transaction 4: Execute transfer (below)
    // Transfer with Split Proofs -------------------------------------------
    let tx4 = {
        let new_decryptable_available_balance = sender_transfer_account_info
            .new_decryptable_available_balance(transfer_amount_lamports, &sender_aes_key)
            .map_err(|_| TokenError::AccountDecryption)?
            .into();

        let instructions = transfer(
            &spl_token_2022::id(),
            &sender_token_account,
            &mint,
            &recipient_token_account,
            &new_decryptable_available_balance,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
            &sender_ata_authority,
            &vec![],
            ProofLocation::ContextStateAccount(&equality_proof_context_state_account.pubkey()),
            ProofLocation::ContextStateAccount(
                &ciphertext_validity_proof_context_state_account.pubkey(),
            ),
            ProofLocation::ContextStateAccount(&range_proof_context_state_account.pubkey()),
        )?;

        let message =
            v0::Message::try_compile(&sender_ata_authority, &instructions, &[], client_blockhash)?;

        // Create a versioned transaction with a placeholder signature for the sender
        VersionedTransaction {
            // Single placeholder signature for the sender
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(message),
        }
    };

    // Transaction 5: (below)
    // Close Proof Accounts --------------------------------------------------
    let tx5 = {
        // Lamports from the closed proof accounts will be sent to this account
        let destination_account = &sender_ata_authority;

        // Close the equality proof account
        let close_equality_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &equality_proof_context_state_account.pubkey(),
                context_state_authority: &context_state_authority,
            },
            &destination_account,
        );

        // Close the ciphertext validity proof account
        let close_ciphertext_validity_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &ciphertext_validity_proof_context_state_account.pubkey(),
                context_state_authority: &context_state_authority,
            },
            &destination_account,
        );

        // Close the range proof account
        let close_range_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &range_proof_context_state_account.pubkey(),
                context_state_authority: &context_state_authority,
            },
            &destination_account,
        );

        let message = v0::Message::try_compile(
            &sender_ata_authority,
            &[
                close_equality_proof_instruction,
                close_ciphertext_validity_proof_instruction,
                close_range_proof_instruction,
            ],
            &[],
            client_blockhash,
        )?;

        // Create a versioned transaction with a placeholder signature for the sender
        VersionedTransaction {
            // Single placeholder signature for the sender
            signatures: vec![Signature::default()],
            message: VersionedMessage::V0(message),
        }
    };

    // Return all transactions
    let transactions = vec![tx1, tx2, tx3, tx4, tx5];
    let response = MultiTransactionResponse {
        transactions: transactions
            .into_iter()
            .enumerate()
            .map(|(i, tx)| {
                let serialized_transaction = match bincode::serialize(&tx) {
                    Ok(bytes) => BASE64_STANDARD.encode(bytes),
                    Err(_) => return Err(AppError::SerializationError),
                };
                println!("✅ Successfully serialized transaction {}", i + 1);

                Ok(serialized_transaction)
            })
            .collect::<Result<Vec<String>, AppError>>()?,
        message: "MultiTransaction for confidential transfer created successfully".to_string(),
    };

    Ok(Json(response))
}
