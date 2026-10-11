//! Data router — selects the appropriate vendor for each request type.
//!
//! The router applies a priority order at runtime so callers never need to
//! know which vendor is actually used:
//!
//! | Data kind      | Priority                              |
//! |----------------|---------------------------------------|
//! | Price data     | Tushare → AKShare → BaoStock          |
//! | Fundamentals   | Tushare → AKShare (no fundamentals)   |
//! | News           | Tushare → AKShare                     |
//! | Macro          | AKShare (CPI + PPI + GDP)             |
//! | Real estate    | RealEstateClient (AKShare-backed)     |

#![allow(dead_code)]

use anyhow::Context;
use serde_json::{json, Value};

use crate::config::{AssetType, Config};

use super::{
    akshare::AkshareClient,
    baostock::BaostockClient,
    real_estate::RealEstateClient,
    tushare::TushareClient,
};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Unified data gateway that dispatches requests to the correct vendor.
pub struct DataRouter {
    /// AKShare client (always available — no token required).
    pub akshare: AkshareClient,
    /// Tushare client — only present when a token is configured.
    pub tushare: Option<TushareClient>,
    /// BaoStock client (always available — no token required).
    pub baostock: BaostockClient,
    /// Real-estate data client (backed by AKShare).
    pub real_estate: RealEstateClient,
    /// A snapshot of the application configuration used for routing decisions.
    config: Config,
}

impl DataRouter {
    /// Build a router from the application [`Config`].
    pub fn new(config: &Config) -> Self {
        let akshare = AkshareClient::new(&config.akshare_python);
        let baostock = BaostockClient::new(&config.akshare_python);
        let real_estate = RealEstateClient::new(&config.akshare_python);

        let tushare = config
            .tushare_token
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(TushareClient::new);

        Self {
            akshare,
            tushare,
            baostock,
            real_estate,
            config: config.clone(),
        }
    }

    // ── Public methods ───────────────────────────────────────────────────────

    /// Fetch OHLCV price data for `symbol` between `start_date` and
    /// `end_date` (both `"YYYYMMDD"`).
    ///
    /// Priority: **Tushare → AKShare → BaoStock**.
    ///
    /// Tushare expects the ticker in `"600519.SH"` format; AKShare and
    /// BaoStock use the bare `"600519"` and `"sh.600519"` formats
    /// respectively — the router converts automatically.
    pub async fn get_price_data(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        // ── 1. Tushare ───────────────────────────────────────────────────────
        if let Some(ts) = &self.tushare {
            let ts_code = bare_to_tushare(symbol);
            match ts.get_daily(&ts_code, start_date, end_date).await {
                Ok(data) => {
                    tracing::debug!("price data for {symbol} served by Tushare");
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Tushare get_daily failed, falling back: {e:#}");
                }
            }
        }

        // ── 2. AKShare ───────────────────────────────────────────────────────
        match self
            .akshare
            .get_stock_daily(symbol, start_date, end_date)
            .await
        {
            Ok(data) => {
                tracing::debug!("price data for {symbol} served by AKShare");
                return Ok(data);
            }
            Err(e) => {
                tracing::warn!("AKShare get_stock_daily failed, falling back: {e:#}");
            }
        }

        // ── 3. BaoStock ──────────────────────────────────────────────────────
        let bs_code = bare_to_baostock(symbol);
        // BaoStock uses "YYYY-MM-DD" date format.
        let bs_start = reformat_date(start_date);
        let bs_end = reformat_date(end_date);
        self.baostock
            .get_stock_history(&bs_code, &bs_start, &bs_end)
            .await
            .with_context(|| format!("all price-data vendors failed for {symbol}"))
    }

    /// Fetch fundamental data (income statement + balance sheet) for
    /// `symbol`.
    ///
    /// Priority: **Tushare → AKShare fallback (empty JSON)**.
    ///
    /// When Tushare is available both statements are fetched and merged into a
    /// single JSON object `{ "income": [...], "balance_sheet": [...] }`.
    pub async fn get_fundamentals(&self, symbol: &str) -> anyhow::Result<String> {
        if let Some(ts) = &self.tushare {
            let ts_code = bare_to_tushare(symbol);

            let income_fut = ts.get_income(&ts_code);
            let balance_fut = ts.get_balancesheet(&ts_code);

            let (income_res, balance_res) = tokio::join!(income_fut, balance_fut);

            let income: Value = income_res
                .and_then(|s| {
                    serde_json::from_str(&s).context("failed to parse income JSON")
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("tushare income failed: {e:#}");
                    Value::Array(vec![])
                });

            let balance_sheet: Value = balance_res
                .and_then(|s| {
                    serde_json::from_str(&s).context("failed to parse balance sheet JSON")
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("tushare balancesheet failed: {e:#}");
                    Value::Array(vec![])
                });

            let combined = json!({
                "income":        income,
                "balance_sheet": balance_sheet,
            });

            tracing::debug!("fundamentals for {symbol} served by Tushare");
            return serde_json::to_string(&combined)
                .context("failed to serialise fundamentals");
        }

        // AKShare does not expose structured financials, so return an empty
        // object rather than an error — callers can handle the empty case.
        tracing::warn!(
            "no Tushare token configured; fundamentals unavailable for {symbol}"
        );
        Ok(r#"{"income":[],"balance_sheet":[]}"#.to_string())
    }

    /// Fetch news articles for `symbol` within the given date range.
    ///
    /// Priority: **Tushare → AKShare**.
    pub async fn get_news(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        // ── 1. Tushare ───────────────────────────────────────────────────────
        if let Some(ts) = &self.tushare {
            let ts_code = bare_to_tushare(symbol);
            match ts.get_news(&ts_code, start_date, end_date).await {
                Ok(data) => {
                    tracing::debug!("news for {symbol} served by Tushare");
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!("Tushare get_news failed, falling back: {e:#}");
                }
            }
        }

        // ── 2. AKShare ───────────────────────────────────────────────────────
        self.akshare
            .get_stock_news(symbol)
            .await
            .with_context(|| format!("all news vendors failed for {symbol}"))
    }

    /// Fetch combined Chinese macro data: CPI + PPI + GDP.
    ///
    /// All three indicators are requested concurrently from AKShare.  Results
    /// are merged into a single JSON object:
    ///
    /// ```json
    /// { "cpi": [...], "ppi": [...], "gdp": [...] }
    /// ```
    ///
    /// Individual failures are logged but do not abort the whole request —
    /// the failed indicator is returned as an empty array.
    pub async fn get_macro_data(&self) -> anyhow::Result<String> {
        let (cpi_res, ppi_res, gdp_res) = tokio::join!(
            self.akshare.get_macro_data("cpi"),
            self.akshare.get_macro_data("ppi"),
            self.akshare.get_macro_data("gdp"),
        );

        let parse_or_empty = |res: anyhow::Result<String>, label: &str| -> Value {
            res.and_then(|s| {
                    serde_json::from_str(&s).context("JSON parse failed")
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("macro {label} failed: {e:#}");
                    Value::Array(vec![])
                })
        };

        let combined = json!({
            "cpi": parse_or_empty(cpi_res, "cpi"),
            "ppi": parse_or_empty(ppi_res, "ppi"),
            "gdp": parse_or_empty(gdp_res, "gdp"),
        });

        serde_json::to_string(&combined).context("failed to serialise macro data")
    }

    /// Fetch China real-estate data.
    ///
    /// `query` is a hint for what kind of data is needed:
    ///
    /// * `"news"` — returns the latest real-estate news headlines.
    /// * `"land"` — returns land-supply statistics.
    /// * anything else (including `""`) — returns new-area statistics.
    ///
    /// All data comes from the [`RealEstateClient`] (AKShare-backed).
    pub async fn get_real_estate_data(&self, query: &str) -> anyhow::Result<String> {
        match query {
            "news" => self.real_estate.get_real_estate_news().await,
            "land" => self.real_estate.get_land_price_data().await,
            city => self.real_estate.get_house_price_index(city).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Ticker format helpers
// ---------------------------------------------------------------------------

/// Convert a bare A-share ticker to Tushare format.
///
/// * Shanghai exchange stocks start with `6` → append `.SH`.
/// * Everything else (Shenzhen / ChiNext / STAR) → append `.SZ`.
fn bare_to_tushare(symbol: &str) -> String {
    // Handle already-formatted codes (e.g. "600519.SH").
    if symbol.contains('.') {
        return symbol.to_string();
    }
    let suffix = if symbol.starts_with('6') { "SH" } else { "SZ" };
    format!("{symbol}.{suffix}")
}

/// Convert a bare ticker to BaoStock format.
///
/// * Shanghai → `"sh.{symbol}"`.
/// * Shenzhen → `"sz.{symbol}"`.
fn bare_to_baostock(symbol: &str) -> String {
    // Handle already-formatted codes (e.g. "sh.600519").
    if symbol.contains('.') {
        return symbol.to_string();
    }
    let prefix = if symbol.starts_with('6') { "sh" } else { "sz" };
    format!("{prefix}.{symbol}")
}

/// Reformat a `"YYYYMMDD"` date string to `"YYYY-MM-DD"` for BaoStock.
///
/// If the input does not look like an 8-digit date string it is returned
/// unchanged so the caller's error surfaces from BaoStock itself.
fn reformat_date(date: &str) -> String {
    if date.len() == 8 && date.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..])
    } else {
        date.to_string()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tushare_format_sh() {
        assert_eq!(bare_to_tushare("600519"), "600519.SH");
    }

    #[test]
    fn tushare_format_sz() {
        assert_eq!(bare_to_tushare("000001"), "000001.SZ");
    }

    #[test]
    fn tushare_already_formatted() {
        assert_eq!(bare_to_tushare("600519.SH"), "600519.SH");
    }

    #[test]
    fn baostock_format_sh() {
        assert_eq!(bare_to_baostock("600519"), "sh.600519");
    }

    #[test]
    fn baostock_format_sz() {
        assert_eq!(bare_to_baostock("000001"), "sz.000001");
    }

    #[test]
    fn date_reformat() {
        assert_eq!(reformat_date("20240115"), "2024-01-15");
        assert_eq!(reformat_date("2024-01-15"), "2024-01-15");
    }
}
