use std::sync::Arc;

use common::test_helpers::test_key;
use http::Request;
use solana_client::nonblocking::rpc_client::RpcClient;
use tower::ServiceExt;

use crate::router;

#[tokio::test]
async fn test_initialize() {
    let key = test_key();

    let router = router::new(Arc::new(RpcClient::new(
        "https://api.devnet.solana.com".to_string(),
    )));

    /*router.oneshot(
        Request::builder()
        .method("POST")
        .uri("/confidential-balances/initialize")
        .body
    );*/
}
