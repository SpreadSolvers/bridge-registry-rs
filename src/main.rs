use clap::{Parser, Subcommand};

use bridge_registry::{ALL_BRIDGES, Bridge, BridgeError};

#[derive(Parser)]
#[command(name = "bridge-registry")]
#[command(about = "Registry of supported cross-chain bridges")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List supported chains for a bridge
    Chains {
        /// Bridge name (e.g. stargate)
        bridge: String,
    },
    /// List supported tokens for a bridge
    Tokens {
        /// Bridge name (e.g. stargate)
        bridge: String,
    },
    /// List all known bridges
    List,
    /// List bridges that support a given chain ID
    Bridges {
        /// EVM chain ID (e.g. 42161)
        chain_id: u64,
        /// CAIP-2 namespace (default: eip155). Other namespaces not supported yet.
        #[arg(long, default_value = "eip155")]
        namespace: String,
    },
}

fn resolve_bridge(name: &str) -> Result<Bridge, BridgeError> {
    Bridge::from_name(name).ok_or_else(|| BridgeError::UnknownBridge(name.to_string()))
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Chains { bridge } => {
            let b = resolve_bridge(&bridge)?;
            let chains = b.chains().await?;
            println!("{}", serde_json::to_string_pretty(&chains)?);
        }
        Commands::Tokens { bridge } => {
            let b = resolve_bridge(&bridge)?;
            let tokens = b.tokens().await?;
            println!("{}", serde_json::to_string_pretty(&tokens)?);
        }
        Commands::List => {
            let names: Vec<&str> = ALL_BRIDGES.iter().map(|b| b.name()).collect();
            println!("{}", serde_json::to_string_pretty(&names)?);
        }
        Commands::Bridges {
            chain_id,
            namespace,
        } => {
            if namespace != "eip155" {
                return Err(format!(
                    "namespace '{namespace}' is not supported yet; only eip155 is implemented"
                )
                .into());
            }
            let chain_id_str = format!("eip155:{chain_id}");
            let mut supporting = Vec::new();
            for &bridge in ALL_BRIDGES {
                let chains = bridge.chains().await?;
                if chains.iter().any(|c| c.id == chain_id_str) {
                    supporting.push(bridge.name());
                }
            }
            println!("{}", serde_json::to_string_pretty(&supporting)?);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
