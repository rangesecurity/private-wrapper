use api::types::{ApiBalancesResponse, Balances};
use common::key_generator::KeypairType;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token_wrap::get_wrapped_mint_address;

pub async fn balances(
    api_url: String,
    keypair: String,
    unwrapped_mint: String,
) -> anyhow::Result<()> {
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

    let payload = Balances {
        authority: key.pubkey(),
        token_mint: wrapped_mint,
        elgamal_signature: elgamal_sig,
        ae_signature: ae_sig,
    };

    log::info!("{}", serde_json::to_string_pretty(&payload).unwrap());

    let req = client
        .post(format!("{api_url}/confidential-balances/balances"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .build()?;
    let res = client.execute(req).await?;
    let res: ApiBalancesResponse = res.json().await?;

    log::info!("{}", serde_json::to_string_pretty(&res).unwrap());

    Ok(())
}
