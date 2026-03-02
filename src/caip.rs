//! CAIP-2 and CAIP-10 helpers. See .cursor/rules/caip-standards.mdc.

/// Solana genesis hash refs (first 32 chars of Base58btc). Per solana/caip2 namespace.
pub const SOLANA_MAINNET_REF: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";
pub const SOLANA_DEVNET_REF: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1";
pub const SOLANA_TESTNET_REF: &str = "4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z";

/// Starknet chain ID strings. Per starknet/caip2 namespace.
pub const STARKNET_MAIN_REF: &str = "SN_MAIN";
pub const STARKNET_SEPOLIA_REF: &str = "SN_SEPOLIA";

pub fn caip2_eip155(chain_id: u64) -> String {
    format!("eip155:{chain_id}")
}

pub fn caip2_solana(ref_: &str) -> String {
    format!("solana:{ref_}")
}

pub fn caip2_starknet(ref_: &str) -> String {
    format!("starknet:{ref_}")
}

pub fn caip10(caip2_namespace: &str, address: &str) -> String {
    format!("{caip2_namespace}:{address}")
}
