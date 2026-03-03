use std::collections::{HashMap, HashSet};

use reqwest::Client;
use serde::Deserialize;

use crate::BridgeError;
use crate::caip;
use crate::types::{ChainInfo, TokenInfo};

const REGISTRY_BASE: &str =
    "https://raw.githubusercontent.com/hyperlane-xyz/hyperlane-registry/main";
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
    #[serde(rename = "chainId", deserialize_with = "deser_chain_id")]
    chain_id: Option<ChainId>,
    #[serde(rename = "domainId")]
    domain_id: Option<u64>,
    name: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    protocol: Option<String>,
}

/// ChainId from metadata: u64 for EVM, or hex string for Starknet (e.g. "0x534e5f4d41494e").
#[derive(Clone)]
enum ChainId {
    U64(u64),
    HexString(String),
}

fn deser_chain_id<'de, D>(deserializer: D) -> Result<Option<ChainId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ChainIdValue {
        U64(u64),
        String(String),
    }
    let opt = Option::<ChainIdValue>::deserialize(deserializer)?;
    Ok(opt.map(|v| match v {
        ChainIdValue::U64(n) => ChainId::U64(n),
        ChainIdValue::String(s) => ChainId::HexString(s),
    }))
}

/// Decode Starknet hex chainId (e.g. "0x534e5f4d41494e") to string ("SN_MAIN").
fn starknet_hex_to_ref(hex: &str) -> Option<String> {
    let s = hex.strip_prefix("0x")?;
    let bytes = (0..s.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect::<Vec<_>>();
    String::from_utf8(bytes).ok()
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

fn chain_id_u64(meta: &ChainMetadata) -> Option<u64> {
    match &meta.chain_id {
        Some(ChainId::U64(n)) => Some(*n),
        _ => None,
    }
}

/// Build CAIP-2 from chain name and metadata. Per namespace spec.
fn caip2_for_chain(chain_name: &str, meta: &ChainMetadata) -> Option<String> {
    let protocol = meta
        .protocol
        .as_deref()
        .unwrap_or("ethereum")
        .to_lowercase();
    match protocol.as_str() {
        "ethereum" | "evm" => {
            let id = chain_id_u64(meta)?;
            Some(caip::caip2_eip155(id))
        }
        "sealevel" => {
            let ref_ = match chain_name.to_lowercase().as_str() {
                "solanamainnet" => caip::SOLANA_MAINNET_REF,
                "solanadevnet" => caip::SOLANA_DEVNET_REF,
                "solanatestnet" => caip::SOLANA_TESTNET_REF,
                _ => return None,
            };
            Some(caip::caip2_solana(ref_))
        }
        "starknet" => {
            let ref_ = match &meta.chain_id {
                Some(ChainId::HexString(hex)) => starknet_hex_to_ref(hex)?.into(),
                _ => match chain_name.to_lowercase().as_str() {
                    "starknet" => caip::STARKNET_MAIN_REF.to_string(),
                    "starknetsepolia" => caip::STARKNET_SEPOLIA_REF.to_string(),
                    _ => return None,
                },
            };
            Some(caip::caip2_starknet(&ref_))
        }
        _ => None,
    }
}

async fn fetch_chains_raw() -> Result<(Vec<ChainInfo>, HashMap<String, String>), BridgeError> {
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
    let chain_dirs: Vec<serde_json::Value> =
        serde_json::from_str(&chain_text).map_err(|_| BridgeError::ApiError {
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
                let id = caip2_for_chain(&name, &meta)?;
                let display = meta
                    .display_name
                    .or(meta.name)
                    .unwrap_or_else(|| name.clone());
                Some((
                    ChainInfo {
                        id: id.clone(),
                        name: display,
                    },
                    name,
                    id,
                ))
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    let chains_with_keys: Vec<(ChainInfo, String, String)> =
        results.into_iter().filter_map(|r| r).collect();
    let chains: Vec<ChainInfo> = chains_with_keys.iter().map(|(c, _, _)| c.clone()).collect();
    let chain_map: HashMap<String, String> = chains_with_keys
        .iter()
        .map(|(_, key, id)| (key.to_lowercase(), id.clone()))
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
    let route_dirs: Vec<serde_json::Value> =
        serde_json::from_str(&text).map_err(|_| BridgeError::ApiError {
            status: status.as_u16(),
            body: text.chars().take(500).collect(),
        })?;

    let route_names: Vec<String> = route_dirs
        .into_iter()
        .filter_map(|v| {
            let obj: &serde_json::Map<String, serde_json::Value> = v.as_object()?;
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

    let route_files: Vec<(String, Vec<String>)> = futures::future::join_all(config_fetches)
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
                let url =
                    format!("{REGISTRY_BASE}/deployments/warp_routes/{route_name}/{config_file}");
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
        let Some((route_name, warp_tokens)) = opt else {
            continue;
        };
        for t in warp_tokens {
            let key = (t.chain_name.to_lowercase(), t.address.to_lowercase());
            if !seen.insert(key) {
                continue;
            }
            let Some(chain_id) = chain_map.get(&t.chain_name.to_lowercase()) else {
                continue;
            };
            tokens.push(TokenInfo {
                id: caip::caip10(chain_id, &t.address),
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
