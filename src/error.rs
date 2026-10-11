use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("LLM API error: {0}")]
    Llm(String),

    #[error("Data vendor error ({vendor}): {message}")]
    DataVendor { vendor: String, message: String },

    #[error("No data available for {symbol}: {reason}")]
    NoData { symbol: String, reason: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Asset type {0} is not supported by vendor {1}")]
    UnsupportedAssetType(String, String),
}

pub type Result<T> = std::result::Result<T, AppError>;
