use api::types::{ApiTransactionResponse, Deposit};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token_wrap::get_wrapped_mint_address;
pub async fn deposit(
    api_url: String,
    rpc_url: String,
    keypair: String,
    unwrapped_mint: String,
    amount: u64,
) -> anyhow::Result<()> {
    let rpc = RpcClient::new(rpc_url);
    let unwrapped_mint: Pubkey = unwrapped_mint.parse().unwrap();
    let wrapped_mint = get_wrapped_mint_address(&unwrapped_mint, &spl_token_2022::id());
    let key = Keypair::read_from_file(keypair).unwrap();
    let client = reqwest::ClientBuilder::new().build()?;

    let payload = Deposit {
        authority: key.pubkey(),
        token_mint: wrapped_mint,
        amount,
    };

    log::info!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let req = client
        .post(format!("{api_url}/confidential-balances/deposit"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .build()?;
    let res = client.execute(req).await?;
    let res: ApiTransactionResponse = res.json().await?;

    log::info!("{}", serde_json::to_string_pretty(&res).unwrap());

    let txs = res.decode_transactions()?;
    for mut tx in txs {
        tx.sign(&vec![&key], rpc.get_latest_blockhash().await?);
        log::info!("sending deposit tx");
        let sig = rpc.send_and_confirm_transaction(&tx).await?;
        log::info!("sent deposit tx {sig}");
    }

    Ok(())
}
