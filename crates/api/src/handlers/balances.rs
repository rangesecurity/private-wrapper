use {
    crate::{
        router::AppState,
        types::{ApiBalancesResponse, ApiError, Balances},
    },
    axum::{extract::State, response::IntoResponse, Json},
    common::key_generator::{derive_ae_key, derive_elgamal_key, KeypairType},
    http::StatusCode,
    spl_token_2022::{
        extension::{
            confidential_transfer::{account_info::combine_balances, ConfidentialTransferAccount},
            BaseStateWithExtensions, StateWithExtensions,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeCiphertext, elgamal::ElGamalCiphertext},
    },
    std::sync::Arc,
};

/// Handler which is used to apply pending balance into confidential balance
pub async fn balances(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Balances>,
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

    let Ok(pending_balance_lo) =
        TryInto::<ElGamalCiphertext>::try_into(confidential_transfer_account.pending_balance_lo)
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to parse pending_balance_hi".to_string(),
            }),
        )
            .into_response();
    };
    let Ok(pending_balance_hi) =
        TryInto::<ElGamalCiphertext>::try_into(confidential_transfer_account.pending_balance_hi)
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to parse pending_balance_lo".to_string(),
            }),
        )
            .into_response();
    };

    let Some(pending_balance_lo) = elgamal_key.secret().decrypt_u32(&pending_balance_lo) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to decrypt pending_balance_lo".to_string(),
            }),
        )
            .into_response();
    };

    let Some(pending_balance_hi) = elgamal_key.secret().decrypt_u32(&pending_balance_hi) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to decrypt pending_balance_hi".to_string(),
            }),
        )
            .into_response();
    };

    let Some(pending_balance) = combine_balances(pending_balance_lo, pending_balance_hi) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to combined pending_balance_lo and pending_balance_hi".to_string(),
            }),
        )
            .into_response();
    };

    let Ok(decryptable_available_balance) = TryInto::<AeCiphertext>::try_into(
        confidential_transfer_account.decryptable_available_balance,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to parse decryptable_available_balance".to_string(),
            }),
        )
            .into_response();
    };

    let Some(decrypted_available_balance) = ae_key.decrypt(&decryptable_available_balance) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to decrypt decryptable_available_balance".to_string(),
            }),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(ApiBalancesResponse {
            pending_balance: spl_token_2022::amount_to_ui_amount(pending_balance, decimals),
            available_balance: spl_token_2022::amount_to_ui_amount(
                decrypted_available_balance,
                decimals,
            ),
            non_confidential_balance: spl_token_2022::amount_to_ui_amount(
                token_account.base.amount,
                decimals,
            ),
        }),
    )
        .into_response()
}
