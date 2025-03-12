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
        confidential_transfer::{
            account_info::ApplyPendingBalanceAccountInfo, ConfidentialTransferAccount,
        },
        BaseStateWithExtensions, StateWithExtensions,
    },
    std::sync::Arc,
};

/// Handler which is used to construct the token account initialization transaction
///
/// # Errors
///
/// * Token acount does not exist and/or not configured for confidential transfers
/// * Insufficient token amount
pub async fn apply(
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

    // ensure token account exists
    let Some(token_account) = std::mem::take(&mut accounts[1]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // optimization note: provide an unpack token account
    // ensure token account is configured for confidential transfers
    if !token_account_already_configured(&token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token account is not configured for confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    // get the token mint decimals
    let decimals =
        match StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint.data) {
            Ok(mint) => mint.base.decimals,
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

    // unpack token account
    let token_account =
        match StateWithExtensions::<spl_token_2022::state::Account>::unpack(&token_account.data) {
            Ok(token_account) => token_account,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack token account {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    // retrieve the confidential transfer account extension
    let confidential_transfer_account = match token_account
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

    // get the pending balance data
    let apply_pending_balance_info =
        ApplyPendingBalanceAccountInfo::new(confidential_transfer_account);

    // get the current pending balance counter
    let pending_balance_credit_counter =
        apply_pending_balance_info.pending_balance_credit_counter();

    // get new available balance
    let new_decryptable_available_balance = match apply_pending_balance_info
        .new_decryptable_available_balance(&elgamal_key.secret(), &ae_key)
    {
        Ok(new_balance) => new_balance,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    msg: format!("failed to encrypt new available balance {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let tx = Transaction::new_with_payer(
        &[
            // can only fail if incorrect token program is provided
            spl_token_2022::extension::confidential_transfer::instruction::apply_pending_balance(
                &spl_token_2022::id(),
                &user_ata,
                pending_balance_credit_counter,
                &new_decryptable_available_balance.into(),
                &payload.authority,
                &[&payload.authority],
            )
            .unwrap(),
        ],
        Some(&payload.authority),
    );

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
