use {
    crate::{router::AppState, types::{ApiError, Initialize}}, axum::{extract::State, response::IntoResponse, Json}, common::key_generator::KeypairType, http::StatusCode, solana_sdk::transaction::Transaction, std::sync::Arc,
    base64::{prelude::BASE64_STANDARD, Engine},
};

/// Handler which is used to construct the token account initialization transaction
pub async fn initialize(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Initialize>,
) -> impl IntoResponse {
    // verify both signatures
    if !payload.elgamal_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::ElGamal.message_to_sign(payload.authority)
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify elgamal signature".to_string()
            })
        ).into_response()
    }
    if !payload.ae_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::Ae.message_to_sign(payload.authority)
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify ae signature".to_string()
            })
        ).into_response()
    }
    let tx =  Transaction::new_with_payer(&[], Some(&payload.authority));
    let tx = match bincode::serialize(&tx) {
        Ok(tx) => tx,
        Err(err) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: format!("failed to serialize transaction {err:#?}")
            })
        ).into_response()
    };
    (
        StatusCode::OK,
        Json(BASE64_STANDARD.encode(tx))
    ).into_response()
}