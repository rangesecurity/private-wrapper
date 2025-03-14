use api::types::{ApiTransactionResponse, Withdraw};
use common::key_generator::KeypairType;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token_wrap::get_wrapped_mint_address;

pub async fn withdraw(
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

    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &key.pubkey(),
        &wrapped_mint,
        &spl_token_2022::id(),
    );
    let elgamal_sig = key.sign_message(&KeypairType::ElGamal.message_to_sign(user_ata));
    let ae_sig = key.sign_message(&KeypairType::Ae.message_to_sign(user_ata));

    let client = reqwest::ClientBuilder::new().build()?;

    let equality_proof_keypair = Keypair::new();
    let range_proof_keypair = Keypair::new();

    let payload = Withdraw {
        authority: key.pubkey(),
        token_mint: wrapped_mint,
        elgamal_signature: elgamal_sig,
        ae_signature: ae_sig,
        equality_proof_keypair: equality_proof_keypair.insecure_clone(),
        range_proof_keypair: range_proof_keypair.insecure_clone(),
        amount,
    };

    log::info!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let req = client
        .post(format!("{api_url}/confidential-balances/withdraw"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .build()?;
    let res = client.execute(req).await?;
    let res: ApiTransactionResponse = res.json().await?;

    log::info!("{}", serde_json::to_string_pretty(&res).unwrap());

    let txs = res.decode_transactions()?;
    for (idx, mut tx) in txs.into_iter().enumerate() {
        loop {
            let Ok(blockhash) = rpc.get_latest_blockhash().await else {
                continue;
            };
            if idx == 0 {
                tx.sign(
                    &vec![&key, &equality_proof_keypair, &range_proof_keypair],
                    blockhash,
                );
            } else {
                tx.sign(&vec![&key], blockhash);
            }
            log::info!("sending transfer tx({idx})");
            let sig = match rpc.send_and_confirm_transaction(&tx).await {
                Ok(sig) => sig,
                Err(err) => {
                    log::error!("failed to send transfer tx({idx}) {err:#?}");
                    continue;
                }
            };
            log::info!("sent transfer tx({idx}) {sig})");
            break;
        }
    }

    Ok(())
}
