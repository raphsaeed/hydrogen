//! Unified strongly-typed error definitions for Hydrogen.

use thiserror::Error;

/// Canonical result alias across Hydrogen subsystems.
pub type Result<T> = std::result::Result<T, HydrogenError>;

/// Top-level error enum covering configuration, I/O, vault, Theta, and Carbon boundaries.
#[derive(Debug, Error)]
pub enum HydrogenError {
    /// Invalid or missing configuration value or CLI argument.
    #[error("configuration error: {0}")]
    Config(String),

    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML configuration parse error.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Encrypted `memory.raphs` (`memory_core_rs`) vault error.
    #[error("vault error: {0}")]
    Vault(#[from] memory_core_rs::Error),

    /// Deterministic `theta_calc` evaluation or verification failure.
    #[error("Theta calculation error: {0}")]
    Theta(String),

    /// `Carbon Protocol v0.1` wire or command error.
    #[error("Carbon protocol error: {0}")]
    CarbonProtocol(String),

    /// WebSocket transport or timeout error when talking to Carbon.
    #[error("Carbon WebSocket transport error: {0}")]
    CarbonTransport(String),

    /// Domain or clean-room validation failure.
    #[error("validation error: {0}")]
    Validation(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for HydrogenError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::CarbonTransport(err.to_string())
    }
}
