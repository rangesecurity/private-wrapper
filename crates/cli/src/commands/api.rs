pub async fn start_api(
    listen_url: String,
    rpc_endpoint: String,
) -> anyhow::Result<()> {
    log::info!("starting api");
    api::start_api(&listen_url, rpc_endpoint).await
}