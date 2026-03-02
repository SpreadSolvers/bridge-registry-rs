//! Gas.zip bridge integration.
//!
//! Gas.zip is an instant liquidity bridge for gas refuel, supporting 350+ chains.
//! API docs: https://dev.gas.zip/gas/api/overview

use reqwest::Client;
use serde::Deserialize;

use crate::BridgeError;
use crate::types::{ChainInfo, TokenInfo};

const BASE_URL: &str = "https://backend.gas.zip/v2";

/// Native token placeholder used by many DeFi protocols.
const NATIVE_TOKEN_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

fn client() -> Result<Client, BridgeError> {
    Client::builder()
        .user_agent("bridge-registry/0.1.0")
        .build()
        .map_err(BridgeError::Http)
}

#[derive(Deserialize)]
struct ChainsResponse {
    chains: Vec<ApiChain>,
}

#[derive(Deserialize)]
struct ApiChain {
    name: String,
    chain: u64,
    symbol: Option<String>,
}

async fn fetch_chains_raw() -> Result<Vec<ApiChain>, BridgeError> {
    let url = format!("{BASE_URL}/chains");
    let resp: ChainsResponse = client()?.get(&url).send().await?.json().await?;
    Ok(resp.chains)
}

pub async fn chains() -> Result<Vec<ChainInfo>, BridgeError> {
    let api_chains = fetch_chains_raw().await?;
    Ok(api_chains
        .into_iter()
        .map(|c| ChainInfo {
            caip2: format!("eip155:{}", c.chain),
            chain_id: c.chain,
            name: c.name.clone(),
        })
        .collect())
}

pub async fn tokens() -> Result<Vec<TokenInfo>, BridgeError> {
    let api_chains = fetch_chains_raw().await?;
    Ok(api_chains
        .into_iter()
        .map(|c| {
            let symbol = c.symbol.clone().unwrap_or_else(|| "NATIVE".to_string());
            TokenInfo {
                caip10: format!("eip155:{}:{}", c.chain, NATIVE_TOKEN_ADDRESS),
                chain_id: c.chain,
                chain_key: c.chain.to_string(),
                address: NATIVE_TOKEN_ADDRESS.to_string(),
                symbol: symbol.clone(),
                name: format!("{} Native", c.name),
                decimals: 18,
            }
        })
        .collect())
}
