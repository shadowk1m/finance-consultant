//! Tushare vendor — HTTP REST API for A-share financial data.
//!
//! All requests are POST calls to `https://api.tushare.pro`.  The Tushare
//! token is sent with every request; obtain one at <https://tushare.pro>.

#![allow(dead_code)]

use anyhow::{bail, Context};
use serde_json::{json, Value};

const TUSHARE_API_URL: &str = "https://api.tushare.pro";

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for the Tushare financial data API.
pub struct TushareClient {
    /// Tushare API token.
    pub token: String,
    /// Shared HTTP client (keep-alive, connection pooling).
    client: reqwest::Client,
}

impl TushareClient {
    /// Create a new client using the supplied API `token`.
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    // ── Internal helper ──────────────────────────────────────────────────────

    /// Call a Tushare API endpoint.
    ///
    /// * `api_name` — Tushare interface name, e.g. `"daily"`.
    /// * `params`   — Request parameters as a JSON object.
    /// * `fields`   — Comma-separated list of fields to return (empty string
    ///   means "all fields").
    ///
    /// Returns the `data` field of the Tushare response on success, or an
    /// error if the HTTP call fails or the API returns a non-zero code.
    async fn call(
        &self,
        api_name: &str,
        params: Value,
        fields: &str,
    ) -> anyhow::Result<Value> {
        let body = json!({
            "api_name": api_name,
            "token":    self.token,
            "params":   params,
            "fields":   fields,
        });

        let resp = self
            .client
            .post(TUSHARE_API_URL)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("HTTP request to Tushare ({api_name}) failed"))?;

        let status = resp.status();
        let json: Value = resp
            .json()
            .await
            .with_context(|| format!("failed to parse Tushare response for {api_name}"))?;

        if !status.is_success() {
            bail!(
                "Tushare HTTP error {status} for api_name={api_name}: {json}"
            );
        }

        // Tushare signals API-level errors with a non-zero `code` field.
        let code = json.get("code").and_then(Value::as_i64).unwrap_or(-1);
        if code != 0 {
            let msg = json
                .get("msg")
                .and_then(Value::as_str)
                .unwrap_or("(no message)");
            bail!("Tushare API error (code={code}) for {api_name}: {msg}");
        }

        json.get("data")
            .cloned()
            .with_context(|| format!("Tushare response for {api_name} has no `data` field"))
    }

    /// Convert a Tushare `data` object (which uses columnar `fields` /
    /// `items` arrays) into a JSON array of record objects, then serialise it
    /// to a `String`.
    fn data_to_records_string(data: &Value) -> anyhow::Result<String> {
        let fields = data
            .get("fields")
            .and_then(Value::as_array)
            .with_context(|| "Tushare data missing `fields`")?;

        let items = data
            .get("items")
            .and_then(Value::as_array)
            .with_context(|| "Tushare data missing `items`")?;

        let records: Vec<Value> = items
            .iter()
            .map(|row| {
                let obj: serde_json::Map<String, Value> = fields
                    .iter()
                    .zip(row.as_array().unwrap_or(&vec![]))
                    .map(|(k, v)| {
                        (
                            k.as_str().unwrap_or("").to_string(),
                            v.clone(),
                        )
                    })
                    .collect();
                Value::Object(obj)
            })
            .collect();

        serde_json::to_string(&records).context("failed to serialise Tushare records")
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Fetch daily OHLCV data for a stock.
    ///
    /// * `ts_code`    — Tushare ticker, e.g. `"600519.SH"`.
    /// * `start_date` — `"YYYYMMDD"`.
    /// * `end_date`   — `"YYYYMMDD"`.
    ///
    /// Returns a JSON array string.
    pub async fn get_daily(
        &self,
        ts_code: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        let params = json!({
            "ts_code":    ts_code,
            "start_date": start_date,
            "end_date":   end_date,
        });

        let data = self
            .call("daily", params, "ts_code,trade_date,open,high,low,close,vol,amount")
            .await
            .with_context(|| format!("tushare get_daily failed for {ts_code}"))?;

        Self::data_to_records_string(&data)
    }

    /// Fetch the income statement for a stock.
    ///
    /// `ts_code` — Tushare ticker, e.g. `"600519.SH"`.
    ///
    /// Returns a JSON array string.
    pub async fn get_income(&self, ts_code: &str) -> anyhow::Result<String> {
        let params = json!({
            "ts_code": ts_code,
        });

        let data = self
            .call(
                "income",
                params,
                "ts_code,ann_date,f_ann_date,end_date,report_type,\
                 total_revenue,revenue,total_cogs,oper_cost,\
                 operate_profit,total_profit,n_income",
            )
            .await
            .with_context(|| format!("tushare get_income failed for {ts_code}"))?;

        Self::data_to_records_string(&data)
    }

    /// Fetch the balance sheet for a stock.
    ///
    /// `ts_code` — Tushare ticker, e.g. `"600519.SH"`.
    ///
    /// Returns a JSON array string.
    pub async fn get_balancesheet(&self, ts_code: &str) -> anyhow::Result<String> {
        let params = json!({
            "ts_code": ts_code,
        });

        let data = self
            .call(
                "balancesheet",
                params,
                "ts_code,ann_date,f_ann_date,end_date,report_type,\
                 total_assets,total_liab,total_hldr_eqy_inc_min_int,\
                 money_cap,accounts_receiv,inventories,lt_borr,st_borr",
            )
            .await
            .with_context(|| format!("tushare get_balancesheet failed for {ts_code}"))?;

        Self::data_to_records_string(&data)
    }

    /// Fetch news articles mentioning a stock within a date range.
    ///
    /// * `ts_code`    — Tushare ticker, e.g. `"600519.SH"`.
    /// * `start_date` — `"YYYYMMDD"`.
    /// * `end_date`   — `"YYYYMMDD"`.
    ///
    /// Filters to a curated list of reliable Chinese financial news sources.
    /// Returns a JSON array string.
    pub async fn get_news(
        &self,
        ts_code: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<String> {
        // Reliable Chinese financial news sources supported by Tushare.
        let src = "sina,wallstreetcn,10jqka,eastmoney,yuncaijing,finet,rss";

        let params = json!({
            "src":        src,
            "start_date": format!("{} 09:00:00", start_date),
            "end_date":   format!("{} 23:59:59", end_date),
        });

        let data = self
            .call("news", params, "datetime,content,title,channels,src")
            .await
            .with_context(|| format!("tushare get_news failed for {ts_code}"))?;

        Self::data_to_records_string(&data)
    }
}
