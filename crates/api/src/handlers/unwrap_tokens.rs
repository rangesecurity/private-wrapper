use {
    crate::{
        router::AppState,
        types::{ApiError, ApiTransactionResponse, InitializeOrApply, WrapTokens},
    },
    axum::{extract::State, response::IntoResponse, Extension, Json},
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
    spl_token_wrap::get_wrapped_mint_authority,
    std::sync::Arc,
};

pub async fn unwrap_tokens(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WrapTokens>,
) -> impl IntoResponse {
    let unwrapped_user_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payload.authority,
            &payload.unwrapped_token_mint,
            &payload.unwrapped_token_program,
        );
    let wrapped_user_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payload.authority,
            &payload.wrapped_token_mint,
            &spl_token_2022::id(),
        );
    let wrapped_mint_authority = get_wrapped_mint_authority(&payload.wrapped_token_mint);
    let ix = spl_token_wrap::instruction::unwrap(
        &spl_token_wrap::id(),
        &spl_associated_token_account::get_associated_token_address_with_program_id(
            &wrapped_mint_authority,
            &payload.unwrapped_token_mint,
            &payload.unwrapped_token_program,
        ),
        &unwrapped_user_ata,
        &wrapped_mint_authority,
        &payload.unwrapped_token_mint,
        &spl_token_2022::id(),
        &payload.unwrapped_token_program,
        &wrapped_user_ata,
        &payload.wrapped_token_mint,
        &payload.authority,
        &[],
        payload.amount
    );

    let tx = match bincode::serialize(&Transaction::new_with_payer(
        &[ix],
        Some(&payload.authority),
    )) {
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
