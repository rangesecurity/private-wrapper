
use {
    axum::{routing::post, Router}, solana_client::nonblocking::rpc_client::RpcClient, std::sync::Arc, tower_http::{cors::{Any, CorsLayer}, trace::{DefaultOnResponse, TraceLayer}, LatencyUnit}, tracing::Level,
    crate::handlers,
};

pub struct AppState {
    rpc: Arc<RpcClient>
}

pub fn new(rpc: Arc<RpcClient>) -> Router {
    Router::new()
    .route("/confidential-balances/transfer-amount-auditor", post(|| async {
        "hello"
    }))
    .route("/confidential-balances/transfer-amount-sender-receiver",post(|| async {
        "hello"
    }))
    .route("/confidential-balances/create-confidential-mint", post(|| async {
        "hello"
    }))
    .route("/confidential-balances/initialize", post(handlers::initialize))
    .route("/confidential-balances/deposit", post(|| async {
        "hello"
    }))
    .route("/confidential-balances/withdraw", post(|| async {
        "hello"
    }))
    .route("/confidential-balances/transfer", post(|| async {
        "hello"
    }))
    .with_state(Arc::new(AppState {
        rpc
    }))
    .layer(
        TraceLayer::new_for_http()
        .on_response(
            DefaultOnResponse::new()
            .level(Level::INFO)
            .latency_unit(LatencyUnit::Millis)
        )
    )
    .layer(
        CorsLayer::default()
        .allow_headers(Any)
        .allow_methods(Any)
        .allow_origin(Any)
    )
}