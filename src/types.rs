use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChainInfo {
    pub caip2: String,
    pub chain_id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub caip10: String,
    pub chain_id: u64,
    pub chain_key: String,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
