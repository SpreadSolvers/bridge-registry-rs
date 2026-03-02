use std::collections::{HashMap, HashSet};

use reqwest::Client;
use serde::Deserialize;

use crate::BridgeError;
use crate::types::{ChainInfo, TokenInfo};

const REGISTRY_BASE: &str = "https://raw.githubusercontent.com/hyperlane-xyz/hyperlane-registry/main";
const GITHUB_API: &str = "https://api.github.com/repos/hyperlane-xyz/hyperlane-registry";

fn client() -> Result<Client, BridgeError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github.v3+json".parse().unwrap(),
    );
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if let Ok(auth_value) = format!("Bearer {token}").parse() {
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
        }
    }
    Client::builder()
        .user_agent("bridge-registry/0.1.0")
        .default_headers(headers)
        .build()
        .map_err(BridgeError::Http)
}

#[derive(Deserialize)]
struct ChainMetadata {
    chain_id: Option<u64>,
    #[serde(rename = "chainId")]
    chain_id_alt: Option<u64>,
    name: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    protocol: Option<String>,
}

#[derive(Deserialize)]
struct WarpConfig {
    tokens: Option<Vec<WarpToken>>,
}

#[derive(Deserialize, Clone)]
struct WarpToken {
    #[serde(rename = "addressOrDenom")]
    address: String,
    #[serde(rename = "chainName")]
    chain_name: String,
    decimals: Option<u8>,
    symbol: Option<String>,
    name: Option<String>,
}

fn chain_id(meta: &ChainMetadata) -> Option<u64> {
    meta.chain_id.or(meta.chain_id_alt)
}

fn caip2_namespace(protocol: &str) -> &str {
    match protocol.to_lowercase().as_str() {
        "ethereum" | "evm" => "eip155",
        "solana" => "solana",
        "starknet" => "starknet",
        _ => "eip155",
    }
}

async fn fetch_chains_raw(
) -> Result<(Vec<ChainInfo>, HashMap<String, (u64, &'static str)>), BridgeError> {
    let client = client()?;

    let chain_resp = client
        .get(format!("{GITHUB_API}/contents/chains"))
        .send()
        .await?;
    let chain_status = chain_resp.status();
    let chain_text = chain_resp.text().await?;
    if !chain_status.is_success() {
        return Err(BridgeError::ApiError {
            status: chain_status.as_u16(),
            body: chain_text.chars().take(500).collect(),
        });
    }
    let chain_dirs: Vec<serde_json::Value> = serde_json::from_str(&chain_text)
        .map_err(|_| BridgeError::ApiError {
            status: chain_status.as_u16(),
            body: chain_text.chars().take(500).collect(),
        })?;

    let mut chain_names: Vec<String> = chain_dirs
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            if obj.get("type")?.as_str()? == "dir" {
                obj.get("name")?.as_str().map(String::from)
            } else {
                None
            }
        })
        .filter(|n| n != "addresses.yaml" && !n.starts_with('.'))
        .collect();

    chain_names.sort();

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20));
    let client = std::sync::Arc::new(client);

    let futures: Vec<_> = chain_names
        .into_iter()
        .map(|name| {
            let client = client.clone();
            let sem = semaphore.clone();
            async move {
                let _permit = sem.acquire().await.ok()?;
                let url = format!("{REGISTRY_BASE}/chains/{name}/metadata.yaml");
                let text = client.get(&url).send().await.ok()?.text().await.ok()?;
                let meta: ChainMetadata = serde_yaml::from_str(&text).ok()?;
                let chain_id = chain_id(&meta)?;
                let protocol = meta.protocol.unwrap_or_else(|| "ethereum".to_string());
                let ns = caip2_namespace(&protocol);
                let display = meta
                    .display_name
                    .or(meta.name)
                    .unwrap_or_else(|| name.clone());
                Some((ChainInfo {
                    caip2: format!("{ns}:{chain_id}"),
                    chain_id,
                    name: display,
                }, name))
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    let chains_with_keys: Vec<(ChainInfo, String)> =
        results.into_iter().filter_map(|r| r).collect();
    let chains: Vec<ChainInfo> = chains_with_keys.iter().map(|(c, _)| c.clone()).collect();
    let chain_map: HashMap<String, (u64, &'static str)> = chains_with_keys
        .iter()
        .map(|(c, key)| {
            let ns = if c.caip2.starts_with("eip155") {
                "eip155"
            } else if c.caip2.starts_with("solana") {
                "solana"
            } else {
                "unknown"
            };
            (key.to_lowercase(), (c.chain_id, ns))
        })
        .collect();
    Ok((chains, chain_map))
}

async fn fetch_tokens_raw() -> Result<Vec<TokenInfo>, BridgeError> {
    let client = std::sync::Arc::new(client()?);
    let (_chains, chain_map) = fetch_chains_raw().await?;

    let resp = client
        .get(format!("{GITHUB_API}/contents/deployments/warp_routes"))
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(BridgeError::ApiError {
            status: status.as_u16(),
            body: text.chars().take(500).collect(),
        });
    }
    let route_dirs: Vec<serde_json::Value> = serde_json::from_str(&text)
        .map_err(|_| BridgeError::ApiError {
            status: status.as_u16(),
            body: text.chars().take(500).collect(),
        })?;

    let route_names: Vec<String> = route_dirs
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            if obj.get("type")?.as_str()? == "dir" {
                obj.get("name")?.as_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    let config_fetches: Vec<_> = route_names
        .into_iter()
        .map(|route_name| {
            let client = client.clone();
            async move {
                let resp = client
                    .get(format!(
                        "{GITHUB_API}/contents/deployments/warp_routes/{route_name}"
                    ))
                    .send()
                    .await
                    .ok()?;
                let text = resp.text().await.ok()?;
                let files: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
                let config_files: Vec<String> = files
                    .into_iter()
                    .filter_map(|v| {
                        let obj = v.as_object()?;
                        let name = obj.get("name")?.as_str()?;
                        if name.ends_with("-config.yaml") {
                            Some(name.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                Some((route_name, config_files))
            }
        })
        .collect();

    let route_files: Vec<(String, Vec<String>)> =
        futures::future::join_all(config_fetches)
            .await
            .into_iter()
            .filter_map(|r| r)
            .collect();

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
    let config_fetches: Vec<_> = route_files
        .into_iter()
        .flat_map(|(route_name, config_files)| {
            config_files
                .into_iter()
                .map(move |config_file| (route_name.clone(), config_file))
        })
        .map(|(route_name, config_file)| {
            let client = client.clone();
            let sem = semaphore.clone();
            async move {
                let _permit = sem.acquire().await.ok()?;
                let url = format!(
                    "{REGISTRY_BASE}/deployments/warp_routes/{route_name}/{config_file}"
                );
                let text = client.get(&url).send().await.ok()?.text().await.ok()?;
                let config: WarpConfig = serde_yaml::from_str(&text).ok()?;
                Some((route_name, config.tokens?))
            }
        })
        .collect();

    let configs = futures::future::join_all(config_fetches).await;

    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut tokens = Vec::new();
    for opt in configs {
        let Some((route_name, warp_tokens)) = opt else { continue };
        for t in warp_tokens {
            let key = (t.chain_name.to_lowercase(), t.address.to_lowercase());
            if !seen.insert(key) {
                continue;
            }
            let Some(&(chain_id, ns)) = chain_map.get(&t.chain_name.to_lowercase()) else {
                continue;
            };
            tokens.push(TokenInfo {
                caip10: format!("{ns}:{chain_id}:{}", t.address),
                chain_id,
                chain_key: t.chain_name.clone(),
                address: t.address,
                symbol: t.symbol.unwrap_or_else(|| route_name.clone()),
                name: t.name.unwrap_or_else(|| route_name.clone()),
                decimals: t.decimals.unwrap_or(18),
            });
        }
    }
    Ok(tokens)
}

pub async fn chains() -> Result<Vec<ChainInfo>, BridgeError> {
    let (chains, _) = fetch_chains_raw().await?;
    Ok(chains)
}

pub async fn tokens() -> Result<Vec<TokenInfo>, BridgeError> {
    fetch_tokens_raw().await
}
