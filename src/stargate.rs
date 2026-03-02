use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::BridgeError;
use crate::types::{ChainInfo, TokenInfo};

const BASE_URL: &str = "https://transfer.layerzero-api.com/v1";

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
    #[serde(rename = "chainKey")]
    chain_key: String,
    #[serde(rename = "chainType")]
    chain_type: String,
    #[serde(rename = "chainId")]
    chain_id: u64,
}

#[derive(Deserialize)]
struct TokensResponse {
    tokens: Vec<ApiToken>,
}

#[derive(Deserialize)]
struct ApiToken {
    #[serde(rename = "isSupported")]
    is_supported: bool,
    #[serde(rename = "chainKey")]
    chain_key: String,
    address: String,
    decimals: u8,
    symbol: String,
    name: String,
}

fn caip2_namespace(chain_type: &str) -> &str {
    match chain_type {
        "EVM" => "eip155",
        "SOLANA" => "solana",
        "STARKNET" => "starknet",
        _ => "unknown",
    }
}

async fn fetch_chains_raw() -> Result<Vec<ApiChain>, BridgeError> {
    let url = format!("{BASE_URL}/chains");
    let resp: ChainsResponse = client()?.get(&url).send().await?.json().await?;
    Ok(resp.chains)
}

async fn fetch_tokens_raw() -> Result<Vec<ApiToken>, BridgeError> {
    let url = format!("{BASE_URL}/tokens");
    let resp: TokensResponse = client()?.get(&url).send().await?.json().await?;
    Ok(resp.tokens)
}

pub async fn chains() -> Result<Vec<ChainInfo>, BridgeError> {
    let api_chains = fetch_chains_raw().await?;
    Ok(api_chains
        .into_iter()
        .map(|c| {
            let ns = caip2_namespace(&c.chain_type);
            ChainInfo {
                caip2: format!("{ns}:{}", c.chain_id),
                chain_id: c.chain_id,
                name: c.name,
            }
        })
        .collect())
}

pub async fn tokens() -> Result<Vec<TokenInfo>, BridgeError> {
    let (api_chains, api_tokens) =
        tokio::try_join!(fetch_chains_raw(), fetch_tokens_raw())?;

    let chain_map: HashMap<&str, &ApiChain> = api_chains
        .iter()
        .map(|c| (c.chain_key.as_str(), c))
        .collect();

    Ok(api_tokens
        .into_iter()
        .filter(|t| t.is_supported)
        .filter_map(|t| {
            let chain = chain_map.get(t.chain_key.as_str())?;
            let ns = caip2_namespace(&chain.chain_type);
            Some(TokenInfo {
                caip10: format!("{ns}:{}:{}", chain.chain_id, t.address),
                chain_id: chain.chain_id,
                chain_key: t.chain_key,
                address: t.address,
                symbol: t.symbol,
                name: t.name,
                decimals: t.decimals,
            })
        })
        .collect())
}
