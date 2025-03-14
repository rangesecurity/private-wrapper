use api::types::{ApiTransactionResponse, WrapTokens};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token_wrap::get_wrapped_mint_address;

pub async fn wrap(
    api_url: String,
    rpc_url: String,
    keypair: String,
    unwrapped_mint: String,
    unwrapped_mint_program: String,
    amount: u64,
) -> anyhow::Result<()> {
    let unwrapped_mint: Pubkey = unwrapped_mint.parse().unwrap();
    let unwrapped_mint_program: Pubkey = unwrapped_mint_program.parse().unwrap();
    let rpc = RpcClient::new(rpc_url);
    let key = Keypair::read_from_file(keypair).unwrap();

    let client = reqwest::ClientBuilder::new().build()?;

    let payload = WrapTokens {
        authority: key.pubkey(),
        unwrapped_token_mint: unwrapped_mint,
        wrapped_token_mint: get_wrapped_mint_address(&unwrapped_mint, &spl_token_2022::id()),
        unwrapped_token_program: unwrapped_mint_program,
        amount,
    };

    log::info!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let req = client
        .post(format!("{api_url}/private-wrapper/wrap"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .build()?;
    let res = client.execute(req).await?;
    let res: ApiTransactionResponse = res.json().await?;

    log::info!("{}", serde_json::to_string_pretty(&res).unwrap());

    let txs = res.decode_transactions()?;
    for mut tx in txs {
        tx.sign(&vec![&key], rpc.get_latest_blockhash().await?);
        log::info!("sending wrap tx");
        let sig = rpc.send_and_confirm_transaction(&tx).await?;
        log::info!("sent wrap tx {sig}");
    }

    Ok(())
}
