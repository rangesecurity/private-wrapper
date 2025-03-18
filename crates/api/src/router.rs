use {
    crate::handlers,
    axum::{routing::post, Router},
    solana_client::nonblocking::rpc_client::RpcClient,
    std::sync::Arc,
    tower_http::{
        cors::{Any, CorsLayer},
        trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
        LatencyUnit,
    },
    tracing::Level,
};

pub struct AppState {
    pub rpc: Arc<RpcClient>,
}

pub fn new(rpc: Arc<RpcClient>) -> Router {
    Router::new()
        .route(
            "/confidential-balances/transfer-amount-auditor",
            post(|| async { "hello" }),
        )
        .route(
            "/confidential-balances/transfer-amount-sender-receiver",
            post(|| async { "hello" }),
        )
        .route(
            "/confidential-balances/create-confidential-mint",
            post(|| async { "hello" }),
        )
        .route(
            "/confidential-balances/initialize",
            post(handlers::initialize),
        )
        .route("/confidential-balances/deposit", post(handlers::deposit))
        .route("/confidential-balances/withdraw", post(handlers::withdraw))
        .route("/confidential-balances/transfer", post(handlers::transfer))
        .route("/confidential-balances/apply", post(handlers::apply))
        .route("/confidential-balances/balances", post(handlers::balances))
        .route("/private-wrapper/wrap", post(handlers::wrap_tokens))
        .route("/private-wrapper/unwrap", post(handlers::unwrap_tokens))
        .with_state(Arc::new(AppState { rpc }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(
            CorsLayer::default()
                .allow_headers(Any)
                .allow_methods(Any)
                .allow_origin(Any),
        )
}
