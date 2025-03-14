use std::str::FromStr;

use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use commands::Commands;

mod commands;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let cli = commands::Cli::parse();
    init_log(&cli.log_level);

    match cli.command {
        Commands::StartAPI { listen_url, rpc_url } => {
            commands::api::start_api(listen_url, rpc_url).await
        }
        Commands::CreateConfidentialWrappedMint { rpc_url, keypair, unwrapped_mint, unwrapped_mint_program }=> {
            commands::create_confidential_wrapped_mint::create_token_mint( rpc_url, keypair, unwrapped_mint, unwrapped_mint_program).await
        }
        Commands::WrapTokens { api_url,rpc_url, keypair, unwrapped_mint,  unwrapped_mint_program, amount } => {
            commands::wrap::wrap(api_url, rpc_url,keypair, unwrapped_mint, unwrapped_mint_program, amount).await
        }
        Commands::Initialize { api_url, rpc_url, keypair, unwrapped_mint } => {
            commands::initialize::initialize(
                api_url,
                rpc_url,
                keypair,
                unwrapped_mint
            ).await
        }
        Commands::Deposit { api_url, rpc_url, keypair, unwrapped_mint, amount } => {
            commands::deposit::deposit(
                api_url,
                rpc_url,
                keypair,
                unwrapped_mint,
                amount
            ).await
        }
        Commands::Apply { api_url, rpc_url, keypair, unwrapped_mint } => {
            commands::apply::apply(
                api_url,
                rpc_url,
                keypair,
                unwrapped_mint
            ).await
        }
        Commands::Balances { api_url, keypair, unwrapped_mint } => {
            commands::balances::balances(
                api_url,
                keypair,
                unwrapped_mint
            ).await
        }
        Commands::Transfer { api_url, rpc_url, keypair, unwrapped_mint, recipient, amount } => {
            commands::transfer::transfer(
                api_url,
                rpc_url,
                keypair,
                recipient,
                unwrapped_mint,
                amount
            ).await
        }
    }
}

fn init_log(level: &str) {
    let mut layers = Vec::with_capacity(2);
    let level_filter = LevelFilter::from_level(tracing::Level::from_str(level).unwrap());

    layers.push(
        tracing_subscriber::fmt::layer()
            .with_level(true)
            .with_line_number(true)
            .with_file(true)
            .with_filter(level_filter)
            .boxed(),
    );
    if let Err(err) = tracing_subscriber::registry().with(layers).try_init() {
        log::warn!("global subscriber already registered {err:#?}");
    }
}
