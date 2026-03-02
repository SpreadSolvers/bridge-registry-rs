pub mod gaszip;
pub mod hyperlane;
pub mod stargate;
pub mod types;

use types::{ChainInfo, TokenInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bridge {
    GasZip,
    Hyperlane,
    Stargate,
}

pub const ALL_BRIDGES: &[Bridge] = &[Bridge::GasZip, Bridge::Hyperlane, Bridge::Stargate];

impl std::fmt::Display for Bridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl serde::Serialize for Bridge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unknown bridge: {0}")]
    UnknownBridge(String),

    #[error("API error ({status}): {body}. Set GITHUB_TOKEN for higher rate limits.")]
    ApiError { status: u16, body: String },
}

impl Bridge {
    pub fn name(&self) -> &'static str {
        match self {
            Bridge::GasZip => "gaszip",
            Bridge::Hyperlane => "hyperlane",
            Bridge::Stargate => "stargate",
        }
    }

    pub fn from_name(name: &str) -> Option<Bridge> {
        match name.to_lowercase().as_str() {
            "gaszip" | "gas.zip" => Some(Bridge::GasZip),
            "hyperlane" => Some(Bridge::Hyperlane),
            "stargate" => Some(Bridge::Stargate),
            _ => None,
        }
    }

    pub async fn chains(&self) -> Result<Vec<ChainInfo>, BridgeError> {
        match self {
            Bridge::GasZip => gaszip::chains().await,
            Bridge::Hyperlane => hyperlane::chains().await,
            Bridge::Stargate => stargate::chains().await,
        }
    }

    pub async fn tokens(&self) -> Result<Vec<TokenInfo>, BridgeError> {
        match self {
            Bridge::GasZip => gaszip::tokens().await,
            Bridge::Hyperlane => hyperlane::tokens().await,
            Bridge::Stargate => stargate::tokens().await,
        }
    }
}
