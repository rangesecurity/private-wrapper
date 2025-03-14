pub mod api;
pub mod create_confidential_wrapped_mint;
pub mod wrap;
pub mod initialize;
pub mod deposit;
pub mod apply;
pub mod balances;
pub mod transfer;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "private token wrapper cli")]
pub struct Cli {
    #[arg(long, help = "log level to set", default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    StartAPI {
        #[arg(long, help = "ip + port to listen on", default_value = "127.0.0.1:1337")]
        listen_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
    },
    CreateConfidentialWrappedMint {
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
        #[arg(long, help = "token program address for the unwrapped mint", default_value = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
        unwrapped_mint_program: String,
    },
    WrapTokens {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
        #[arg(long, help = "token program address for the unwrapped mint", default_value = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
        unwrapped_mint_program: String,
        #[arg(long, help = "amount of tokens to wrap in lamports")]
        amount: u64,
    },
    Initialize {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
    },
    Deposit {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
        #[arg(long, help = "amount of tokens to deposit in lamports")]
        amount: u64,
    },
    Apply {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
    },
    Balances {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
    },
    Transfer {
        #[arg(long, help = "api endpoint for the private wrapper", default_value = "http://127.0.0.1:1337")]
        api_url: String,
        #[arg(long, help = "solana rpc url", default_value = "https://api.devnet.solana.com/")]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(long, help = "unwrapped token mint", default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv")]
        unwrapped_mint: String,
        #[arg(long, help = "public key of user to transfer funds to", default_value = "BYuf1dG4YecRxCzkykK5tgBnNJo2SVdbedAzuFXgWy9y")]
        recipient: String,
        #[arg(long, help = "amount of tokens to transfer in lamports")]
        amount: u64,
    },
}