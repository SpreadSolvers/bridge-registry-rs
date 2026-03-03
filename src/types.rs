use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChainInfo {
    /// CAIP-2 chain ID (e.g. eip155:1, solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp)
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenInfo {
    /// CAIP-10 account ID (e.g. eip155:1:0x...)
    pub id: String,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
