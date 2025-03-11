pub mod types;
pub mod handlers;
pub mod router;
pub mod serde_utils;

#[cfg(test)]
mod tests;

use {
    anyhow::{Context, Result}, solana_client::nonblocking::rpc_client::RpcClient, std::sync::Arc
};

pub async fn start_api(
    listen_url: &str,
    rpc_url: String
) -> Result<()> {
    let rpc = RpcClient::new(rpc_url);
    let router = router::new(Arc::new(rpc));
    Ok(axum::serve(
        tokio::net::TcpListener::bind(listen_url).await.with_context(|| "failed to create listener")?,
        router
    ).await?)
}