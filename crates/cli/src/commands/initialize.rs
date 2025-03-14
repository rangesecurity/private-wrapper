use api::types::{ApiTransactionResponse, InitializeOrApply};
use common::key_generator::KeypairType;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token_wrap::get_wrapped_mint_address;
pub async fn initialize(
    api_url: String,
    rpc_url: String,
    keypair: String,
    unwrapped_mint: String,
) -> anyhow::Result<()> {
    let rpc = RpcClient::new(rpc_url);
    let unwrapped_mint: Pubkey = unwrapped_mint.parse().unwrap();
    let wrapped_mint = get_wrapped_mint_address(&unwrapped_mint, &spl_token_2022::id());
    let key = Keypair::read_from_file(keypair).unwrap();

    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &wrapped_mint,
        &spl_token_2022::id(),
    );
    let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
    let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

    let client = reqwest::ClientBuilder::new().build()?;

    let payload = InitializeOrApply {
        authority: key.pubkey(),
        token_mint: wrapped_mint,
        elgamal_signature: elgamal_sig,
        ae_signature: ae_sig,
    };

    log::info!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let req = client
        .post(format!("{api_url}/confidential-balances/initialize"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .build()?;
    let res = client.execute(req).await?;
    let res: ApiTransactionResponse = res.json().await?;

    log::info!("{}", serde_json::to_string_pretty(&res).unwrap());

    let txs = res.decode_transactions()?;
    for mut tx in txs {
        tx.sign(&vec![&key], rpc.get_latest_blockhash().await?);
        log::info!("sending initialize tx");
        let sig = rpc.send_and_confirm_transaction(&tx).await?;
        log::info!("sent initialize tx {sig}");
    }

    Ok(())
}
