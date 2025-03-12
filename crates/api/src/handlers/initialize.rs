use {
    crate::{
        router::AppState,
        types::{ApiError, ApiResponse, InitializeOrApply},
    },
    axum::{extract::State, response::IntoResponse, Json},
    base64::{prelude::BASE64_STANDARD, Engine},
    common::{
        accounts::token_account_already_configured,
        key_generator::{derive_ae_key, derive_elgamal_key, KeypairType},
    },
    http::StatusCode,
    solana_sdk::transaction::Transaction,
    spl_token_2022::extension::{
        confidential_transfer::instruction::{configure_account, PubkeyValidityProofData},
        ExtensionType,
    },
    spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation},
    std::sync::Arc,
};

/// Handler which is used to construct the deposit + aply balance instructions
///
/// # Errors
///
/// * Token account is initialized with extension already
/// * Mint account does not support ConfidentialTransferMint
pub async fn initialize(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InitializeOrApply>,
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

    // lookup both the token mint, and ata accounts
    let mut accounts = state
        .rpc
        .get_multiple_accounts(&[payload.token_mint, user_ata])
        .await
        .unwrap_or_default();

    // if less than 2 accounts is returned, this means the rpc call failed
    // if the mint does not exist then `accounts[0] == None`
    // if the user_ata does not exist then `accounts[1] == None`
    if accounts.len() < 2 {
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

    // check to see if the ata already exists
    if let Some(token_account) = std::mem::take(&mut accounts[1]) {
        // token account already exists, check to see if its already configured for confidential transfers
        if token_account_already_configured(&token_account) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    msg: "token account already configured for confidential transfers".to_string(),
                }),
            )
                .into_response();
        }
    }

    // ensure the token mint is valid for confidential transfers
    if !common::accounts::is_valid_mint(token_mint) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not support confidential transfers".to_string(),
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

    // generate proof data
    let Ok(proof_data) = PubkeyValidityProofData::new(&elgamal_key) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to generate proof data".to_string(),
            }),
        )
            .into_response();
    };

    // generate the account configuration instructions
    let Ok(mut configure_instructions) = configure_account(
        &spl_token_2022::id(),
        &user_ata,
        &payload.token_mint,
        &ae_key.encrypt(0).into(),
        65536,
        &payload.authority,
        &[],
        ProofLocation::InstructionOffset(
            1.try_into().unwrap(),
            ProofData::InstructionData(&proof_data),
        ),
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to generate configure instructions".to_string(),
            }),
        )
            .into_response();
    };

    // create the instructions to initialize the ata, and reallocate for confidential transfers
    let mut instructions = vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payload.authority,
            &payload.authority,
            &payload.token_mint,
            &spl_token_2022::id(),
        ),
        // the only possible error for rellocate is if the token program is not spl_token_2022
        spl_token_2022::instruction::reallocate(
            &spl_token_2022::id(),
            &user_ata,
            &payload.authority,
            &payload.authority,
            &[],
            &[ExtensionType::ConfidentialTransferAccount],
        )
        .unwrap(),
    ];

    // update the instructions with account configuration
    instructions.append(&mut configure_instructions);

    // create the transaction, bincode serialize it, and return it as a base64 encoded string

    let tx = Transaction::new_with_payer(&instructions, Some(&payload.authority));

    let tx = match bincode::serialize(&tx) {
        Ok(tx) => tx,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to serialize transaction {err:#?}"),
                }),
            )
                .into_response()
        }
    };
    (
        StatusCode::OK,
        Json(ApiResponse {
            transactions: vec![BASE64_STANDARD.encode(tx)],
        }),
    )
        .into_response()
}
