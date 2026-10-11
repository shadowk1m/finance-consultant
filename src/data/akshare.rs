//! AKShare vendor — wraps the Python AKShare library via subprocess.
//!
//! AKShare is Python-only, so every API call spawns a short-lived `python3 -c`
//! process, captures its stdout as a JSON string, and returns it to the caller.

#![allow(dead_code)]

use anyhow::{bail, Context};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for the AKShare data vendor.
///
/// All methods are `async` and spawn a subprocess under the hood, so they
/// should not be called in a tight loop without back-pressure.
pub struct AkshareClient {
    /// Path to the Python 3 interpreter, e.g. `"python3"` or
    /// `"/usr/local/bin/python3"`.
    pub python_bin: String,
}

impl AkshareClient {
    /// Create a new client.
    ///
    /// `python_bin` is the interpreter used for every subprocess call.  Pass
    /// `"python3"` to rely on `$PATH`, or an absolute path for a specific
    /// virtual-environment interpreter.
    pub fn new(python_bin: &str) -> Self {
        Self {
            python_bin: python_bin.to_string(),
        }
    }

    // ── Internal helper ──────────────────────────────────────────────────────

    /// Execute an inline Python script and return its stdout as a `String`.
    ///
    /// If the process exits with a non-zero status code the stderr output is
    /// wrapped in an [`anyhow::Error`] and returned.
    pub(crate) async fn run_python(&self, script: &str) -> anyhow::Result<String> {
        let output = Command::new(&self.python_bin)
            .arg("-c")
            .arg(script)
            .output()
            .await
            .context("failed to spawn Python subprocess")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            bail!(
                "Python subprocess exited with {}: {}",
                output.status,
                stderr.trim()
            );
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Python subprocess produced non-UTF-8 stdout")?;

        Ok(stdout.trim().to_string())
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Fetch daily OHLCV history for an A-share stock.
    ///
    /// * `symbol`     — bare ticker, e.g. `"600519"` (no exchange suffix).
    /// * `start_date` — `"YYYYMMDD"`.
    /// * `end_date`   — `"YYYYMMDD"`.
    ///
    /// Returns a JSON array string (up to 30 most-recent rows) produced by
    /// `ak.stock_zh_a_hist` with forward-adjusted prices (`qfq`).
    pub async fn get_stock_daily(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        let script = format!(
            r#"
import akshare as ak, json
df = ak.stock_zh_a_hist(
    symbol="{symbol}",
    period="daily",
    start_date="{start_date}",
    end_date="{end_date}",
    adjust="qfq",
)
print(df.tail(30).to_json(orient="records", force_ascii=False))
"#,
            symbol = symbol,
            start_date = start_date,
            end_date = end_date,
        );

        self.run_python(&script)
            .await
            .with_context(|| format!("akshare get_stock_daily failed for {symbol}"))
    }

    /// Fetch the latest news headlines for a stock.
    ///
    /// Returns a JSON array of the 20 most-recent items from
    /// `ak.stock_news_em`.
    pub async fn get_stock_news(&self, symbol: &str) -> anyhow::Result<String> {
        let script = format!(
            r#"
import akshare as ak, json
df = ak.stock_news_em(stock="{symbol}")
print(df.tail(20).to_json(orient="records", force_ascii=False))
"#,
            symbol = symbol,
        );

        self.run_python(&script)
            .await
            .with_context(|| format!("akshare get_stock_news failed for {symbol}"))
    }

    /// Fetch a Chinese macroeconomic indicator time-series.
    ///
    /// `indicator` must be one of `"cpi"`, `"ppi"`, or `"gdp"`.  The
    /// corresponding AKShare function `ak.macro_china_{indicator}()` is
    /// called and the result is returned as a JSON array string.
    pub async fn get_macro_data(&self, indicator: &str) -> anyhow::Result<String> {
        let script = format!(
            r#"
import akshare as ak, json
fn = getattr(ak, "macro_china_{indicator}")
df = fn()
print(df.to_json(orient="records", force_ascii=False))
"#,
            indicator = indicator,
        );

        self.run_python(&script)
            .await
            .with_context(|| format!("akshare get_macro_data failed for indicator={indicator}"))
    }

    /// Fetch the latest CCTV news headlines for global macro context.
    ///
    /// Uses `ak.news_cctv()` and returns the 10 most-recent items as a JSON
    /// array string.
    pub async fn get_global_news(&self) -> anyhow::Result<String> {
        let script = r#"
import akshare as ak, json
df = ak.news_cctv(date=None)
print(df.tail(10).to_json(orient="records", force_ascii=False))
"#;

        self.run_python(script)
            .await
            .context("akshare get_global_news failed")
    }
}
