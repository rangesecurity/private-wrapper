pub mod api;
pub mod create_confidential_wrapped_mint;

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
        #[arg(long, help = "unwrapped token mint")]
        unwrapped_mint: String,
        #[arg(long, help = "token program address for the unwrapped mint")]
        unwrapped_mint_program: String,
    }
}