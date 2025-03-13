use {
    crate::{
        router::AppState,
        types::{ApiError, ApiTransactionResponse, Deposit},
    },
    axum::{extract::State, response::IntoResponse, Json},
    base64::{prelude::BASE64_STANDARD, Engine},
    common::accounts::token_account_already_configured,
    http::StatusCode,
    solana_sdk::transaction::Transaction,
    spl_token_2022::extension::StateWithExtensions,
    std::sync::Arc,
};

/// Handler which is used to deposit into the confidential pending balance
pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Deposit>,
) -> impl IntoResponse {
    // derive the ATA for the authority + token_mint
    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payload.authority,
        &payload.token_mint,
        &spl_token_2022::id(),
    );

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
    if !common::accounts::is_valid_mint(&token_mint) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not support confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    let tx = Transaction::new_with_payer(
        &[
            // deposit can only fail if the incorrect token program is provided
            spl_token_2022::extension::confidential_transfer::instruction::deposit(
                &spl_token_2022::id(),
                &user_ata,
                &payload.token_mint,
                payload.amount,
                decimals,
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
        Json(ApiTransactionResponse {
            transactions: vec![BASE64_STANDARD.encode(tx)],
        }),
    )
        .into_response()
}
