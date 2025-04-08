use {
    crate::{
        router::AppState,
        types::{ApiError, ApiTransactionResponse, WrapTokens},
    },
    axum::{extract::State, response::IntoResponse, Json},
    base64::{prelude::BASE64_STANDARD, Engine},
    http::StatusCode,
    solana_sdk::transaction::Transaction,
    spl_token_wrap::get_wrapped_mint_authority,
    std::sync::Arc,
};

pub async fn unwrap_tokens(
    State(_state): State<Arc<AppState>>,
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

    let mut ixs = vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payload.authority,
            &payload.authority,
            &payload.unwrapped_token_mint,
            &payload.unwrapped_token_program
        ),
        spl_token_wrap::instruction::unwrap(
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
            payload.amount,
        )
    ];

    let tx = match bincode::serialize(&Transaction::new_with_payer(
        &ixs,
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
