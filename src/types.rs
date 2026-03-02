use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChainInfo {
    pub caip2: String,
    pub chain_id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub caip10: String,
    pub chain_id: u64,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
