pub mod api;
pub mod apply;
pub mod balances;
pub mod create_confidential_wrapped_mint;
pub mod deposit;
pub mod initialize;
pub mod transfer;
pub mod unwrap;
pub mod withdraw;
pub mod wrap;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "private token wrapper cli")]
pub struct Cli {
    #[arg(long, help = "log level to set", default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "start the api")]
    StartAPI {
        #[arg(
            long,
            help = "ip + port to listen on",
            default_value = "127.0.0.1:1337"
        )]
        listen_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
    },
    #[command(
        about = "initialize a wrapped mint with the spl token wrap program that supports confidential transfers"
    )]
    CreateConfidentialWrappedMint {
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(
            long,
            help = "token program address for the unwrapped mint",
            default_value = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        )]
        unwrapped_mint_program: String,
    },
    #[command(
        about = "wrap tokens with the spl token wrap program, the wrapped tokens will support confidential transfers"
    )]
    WrapTokens {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(
            long,
            help = "token program address for the unwrapped mint",
            default_value = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        )]
        unwrapped_mint_program: String,
        #[arg(long, help = "amount of tokens to wrap in lamports")]
        amount: u64,
    },
    #[command(about = "initialize a confidential transfer account")]
    Initialize {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
    },
    #[command(about = "deposit tokens from the non confidential balance into pending balance")]
    Deposit {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(long, help = "amount of tokens to deposit in lamports")]
        amount: u64,
    },
    #[command(about = "apply tokens from the pending balance into confidential available balance")]
    Apply {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
    },
    #[command(about = "display confidential and non confidential balances")]
    Balances {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
    },
    #[command(about = "confidentially transfers tokens")]
    Transfer {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(
            long,
            help = "public key of user to transfer funds to",
            default_value = "BYuf1dG4YecRxCzkykK5tgBnNJo2SVdbedAzuFXgWy9y"
        )]
        recipient: String,
        #[arg(long, help = "amount of tokens to transfer in lamports")]
        amount: u64,
    },
    #[command(about = "withdraw tokens from the confidential balance to non confidential balance")]
    Withdraw {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(long, help = "amount of tokens to withdraw in lamports")]
        amount: u64,
    },
    #[command(about = "unwrap tokens with the spl token wrap program")]
    UnwrapTokens {
        #[arg(
            long,
            help = "api endpoint for the private wrapper",
            default_value = "http://127.0.0.1:1337"
        )]
        api_url: String,
        #[arg(
            long,
            help = "solana rpc url",
            default_value = "https://api.devnet.solana.com/"
        )]
        rpc_url: String,
        #[arg(long, help = "path to a json keypair")]
        keypair: String,
        #[arg(
            long,
            help = "unwrapped token mint",
            default_value = "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv"
        )]
        unwrapped_mint: String,
        #[arg(
            long,
            help = "token program address for the unwrapped mint",
            default_value = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        )]
        unwrapped_mint_program: String,
        #[arg(long, help = "amount of tokens to wrap in lamports")]
        amount: u64,
    },
}
