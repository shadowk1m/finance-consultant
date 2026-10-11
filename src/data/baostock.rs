//! BaoStock vendor — free A-share historical data via Python subprocess.
//!
//! BaoStock is a Python library that provides free, open historical data for
//! Chinese A-share markets.  Like AKShare it is Python-only, so every call
//! spawns a `python3 -c` subprocess.

#![allow(dead_code)]

use anyhow::{bail, Context};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for the BaoStock data vendor.
pub struct BaostockClient {
    /// Path to the Python 3 interpreter.
    pub python_bin: String,
}

impl BaostockClient {
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
    async fn run_python(&self, script: &str) -> anyhow::Result<String> {
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
    /// * `code`       — BaoStock ticker, e.g. `"sh.600519"` or `"sz.000001"`.
    /// * `start_date` — `"YYYY-MM-DD"`.
    /// * `end_date`   — `"YYYY-MM-DD"`.
    ///
    /// Queries `bs.query_history_k_data_plus` with dividend-adjusted prices
    /// (`adjustflag="3"` = forward adjust) and returns the result as a JSON
    /// array string.
    pub async fn get_stock_history(
        &self,
        code: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        let script = format!(
            r#"
import baostock as bs, pandas as pd, json

lg = bs.login()
rs = bs.query_history_k_data_plus(
    "{code}",
    "date,code,open,high,low,close,volume,amount,adjustflag",
    start_date="{start_date}",
    end_date="{end_date}",
    frequency="d",
    adjustflag="3",
)

data = []
while rs.error_code == '0' and rs.next():
    data.append(rs.get_row_data())

df = pd.DataFrame(data, columns=rs.fields)
print(df.to_json(orient="records", force_ascii=False))

bs.logout()
"#,
            code = code,
            start_date = start_date,
            end_date = end_date,
        );

        self.run_python(&script)
            .await
            .with_context(|| format!("baostock get_stock_history failed for {code}"))
    }
}
