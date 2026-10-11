use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// AssetType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Stock,
    RealEstate,
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetType::Stock => write!(f, "stock"),
            AssetType::RealEstate => write!(f, "real_estate"),
        }
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Stock
    }
}

// ---------------------------------------------------------------------------
// FileConfig — mirrors the TOML schema; every field is optional so a partial
// config file is accepted without error.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    llm_provider: Option<String>,
    llm_base_url: Option<String>,
    deep_think_model: Option<String>,
    quick_think_model: Option<String>,
    api_key: Option<String>,
    tushare_token: Option<String>,
    akshare_python: Option<String>,
    output_language: Option<String>,
    max_debate_rounds: Option<usize>,
    max_risk_rounds: Option<usize>,
    results_dir: Option<PathBuf>,
    data_cache_dir: Option<PathBuf>,
    asset_type: Option<AssetType>,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Config {
    /// LLM provider name, e.g. "openai" or "deepseek".
    pub llm_provider: String,

    /// Base URL for an OpenAI-compatible endpoint (e.g. DeepSeek).
    /// When `None` the default OpenAI endpoint is used.
    pub llm_base_url: Option<String>,

    /// Model used for deep / slow reasoning tasks.
    pub deep_think_model: String,

    /// Model used for quick / cheap tasks.
    pub quick_think_model: String,

    /// LLM API key.  Loaded from `LLM_API_KEY`, `OPENAI_API_KEY`, or
    /// `DEEPSEEK_API_KEY` (checked in that order).
    pub api_key: String,

    /// Tushare data API token.
    pub tushare_token: Option<String>,

    /// Path to the Python 3 interpreter used to run AKShare scripts.
    pub akshare_python: String,

    /// Language for agent-generated reports ("Chinese" or "English").
    pub output_language: String,

    /// Maximum number of bull/bear debate rounds.
    pub max_debate_rounds: usize,

    /// Maximum number of risk-assessment rounds.
    pub max_risk_rounds: usize,

    /// Root directory for persisted analysis results.
    pub results_dir: PathBuf,

    /// Root directory for cached vendor data.
    pub data_cache_dir: PathBuf,

    /// Type of asset being analysed.
    pub asset_type: AssetType,
}

impl Config {
    /// Load configuration.
    ///
    /// Priority (highest → lowest):
    /// 1. Environment variables.
    /// 2. `~/.finance-consultant/config.toml` (if present).
    /// 3. Hard-coded defaults.
    pub fn load() -> anyhow::Result<Self> {
        // Load a `.env` file from the current directory if one exists.
        // `dotenvy::dotenv()` returns an error when the file is absent, which
        // is fine — we simply ignore it.
        let _ = dotenvy::dotenv();

        // Try to read the TOML config file.
        let file_cfg = Self::load_file_config();

        // ── API key ─────────────────────────────────────────────────────────
        let api_key = env::var("LLM_API_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .or_else(|_| env::var("DEEPSEEK_API_KEY"))
            .ok()
            .or_else(|| file_cfg.as_ref().ok().and_then(|c| c.api_key.clone()))
            .unwrap_or_else(|| {
                tracing::warn!(
                    "No LLM API key found. Set LLM_API_KEY, OPENAI_API_KEY, or DEEPSEEK_API_KEY."
                );
                String::new()
            });

        // ── Helper: env wins over file, file wins over default ───────────────
        let file = file_cfg.unwrap_or_default();

        let llm_provider = env::var("LLM_PROVIDER")
            .ok()
            .or(file.llm_provider)
            .unwrap_or_else(|| "openai".to_string());

        let llm_base_url = env::var("LLM_BASE_URL").ok().or(file.llm_base_url);

        let deep_think_model = env::var("DEEP_THINK_MODEL")
            .ok()
            .or(file.deep_think_model)
            .unwrap_or_else(|| "deepseek-reasoner".to_string());

        let quick_think_model = env::var("QUICK_THINK_MODEL")
            .ok()
            .or(file.quick_think_model)
            .unwrap_or_else(|| "deepseek-chat".to_string());

        let tushare_token = env::var("TUSHARE_TOKEN").ok().or(file.tushare_token);

        let akshare_python = env::var("AKSHARE_PYTHON")
            .ok()
            .or(file.akshare_python)
            .unwrap_or_else(|| "python3".to_string());

        let output_language = env::var("OUTPUT_LANGUAGE")
            .ok()
            .or(file.output_language)
            .unwrap_or_else(|| "Chinese".to_string());

        let max_debate_rounds = env::var("MAX_DEBATE_ROUNDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or(file.max_debate_rounds)
            .unwrap_or(1);

        let max_risk_rounds = env::var("MAX_RISK_ROUNDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or(file.max_risk_rounds)
            .unwrap_or(1);

        // ── Directories ──────────────────────────────────────────────────────
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".finance-consultant");

        let results_dir = env::var("RESULTS_DIR")
            .ok()
            .map(PathBuf::from)
            .or(file.results_dir)
            .unwrap_or_else(|| base_dir.join("results"));

        let data_cache_dir = env::var("DATA_CACHE_DIR")
            .ok()
            .map(PathBuf::from)
            .or(file.data_cache_dir)
            .unwrap_or_else(|| base_dir.join("cache"));

        // ── Asset type ───────────────────────────────────────────────────────
        let asset_type = env::var("ASSET_TYPE")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "stock" => Some(AssetType::Stock),
                "real_estate" | "realestate" => Some(AssetType::RealEstate),
                _ => None,
            })
            .or(file.asset_type)
            .unwrap_or_default();

        Ok(Config {
            llm_provider,
            llm_base_url,
            deep_think_model,
            quick_think_model,
            api_key,
            tushare_token,
            akshare_python,
            output_language,
            max_debate_rounds,
            max_risk_rounds,
            results_dir,
            data_cache_dir,
            asset_type,
        })
    }

    /// Return the directory where results for `ticker` on `date` are stored.
    ///
    /// Example: `results_dir/600519/2024-01-15`
    pub fn results_path(&self, ticker: &str, date: &str) -> PathBuf {
        self.results_dir.join(ticker).join(date)
    }

    /// Return the path for a cached vendor response.
    ///
    /// Example: `data_cache_dir/akshare/600519_daily_20240101.json`
    pub fn cache_path(&self, vendor: &str, key: &str) -> PathBuf {
        self.data_cache_dir.join(vendor).join(format!("{key}.json"))
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Attempt to parse `~/.finance-consultant/config.toml`.
    /// Returns a default `FileConfig` (all `None`) if the file does not exist.
    fn load_file_config() -> anyhow::Result<FileConfig> {
        let path = dirs::home_dir()
            .context("cannot determine home directory")?
            .join(".finance-consultant")
            .join("config.toml");

        if !path.exists() {
            return Ok(FileConfig::default());
        }

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;

        toml::from_str(&raw)
            .with_context(|| format!("failed to parse config file: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_type_display() {
        assert_eq!(AssetType::Stock.to_string(), "stock");
        assert_eq!(AssetType::RealEstate.to_string(), "real_estate");
    }

    #[test]
    fn results_path_structure() {
        let cfg = Config {
            llm_provider: "openai".into(),
            llm_base_url: None,
            deep_think_model: "m".into(),
            quick_think_model: "m".into(),
            api_key: "k".into(),
            tushare_token: None,
            akshare_python: "python3".into(),
            output_language: "Chinese".into(),
            max_debate_rounds: 1,
            max_risk_rounds: 1,
            results_dir: PathBuf::from("/tmp/results"),
            data_cache_dir: PathBuf::from("/tmp/cache"),
            asset_type: AssetType::Stock,
        };

        assert_eq!(
            cfg.results_path("600519", "2024-01-15"),
            PathBuf::from("/tmp/results/600519/2024-01-15")
        );
        assert_eq!(
            cfg.cache_path("akshare", "600519_daily"),
            PathBuf::from("/tmp/cache/akshare/600519_daily.json")
        );
    }
}
