//! China real-estate data via AKShare Python subprocess.
//!
//! AKShare exposes several real-estate endpoints under the `real_estate_*`
//! namespace.  This module wraps the most useful ones:
//!
//! * New-area statistics  — `ak.real_estate_new_area_em()`
//! * News feed            — `ak.real_estate_news_em()`
//! * Land-supply data     — `ak.real_estate_land_supply_em()`

#![allow(dead_code)]

use anyhow::Context;

use super::akshare::AkshareClient;

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for China real-estate data, backed by AKShare.
pub struct RealEstateClient {
    /// Underlying AKShare client used to execute Python scripts.
    pub akshare: AkshareClient,
}

impl RealEstateClient {
    /// Create a new client.
    ///
    /// `python_bin` is forwarded to the inner [`AkshareClient`].
    pub fn new(python_bin: &str) -> Self {
        Self {
            akshare: AkshareClient::new(python_bin),
        }
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Fetch new residential construction area statistics.
    ///
    /// `city` is accepted as a parameter for forward-compatibility but the
    /// `ak.real_estate_new_area_em()` endpoint returns national aggregate
    /// data; city-level filtering can be applied downstream.
    ///
    /// Returns a JSON array string from `ak.real_estate_new_area_em()`, or
    /// `"[]"` if the endpoint is unavailable.
    pub async fn get_house_price_index(&self, city: &str) -> anyhow::Result<String> {
        // `city` is kept for API compatibility; AKShare returns national data.
        let _ = city;

        let script = r#"
try:
    import akshare as ak, json
    df = ak.real_estate_new_area_em()
    print(df.to_json(orient="records", force_ascii=False))
except Exception as e:
    import sys, json
    print("[]")
    print(f"WARNING: real_estate_new_area_em failed: {e}", file=sys.stderr)
"#;

        self.akshare
            .run_python(script)
            .await
            .context("real_estate get_house_price_index failed")
    }

    /// Fetch real-estate news headlines.
    ///
    /// Returns a JSON array string of the 20 most-recent items from
    /// `ak.real_estate_news_em()`.
    pub async fn get_real_estate_news(&self) -> anyhow::Result<String> {
        let script = r#"
import akshare as ak, json
df = ak.real_estate_news_em()
print(df.tail(20).to_json(orient="records", force_ascii=False))
"#;

        self.akshare
            .run_python(script)
            .await
            .context("real_estate get_real_estate_news failed")
    }

    /// Fetch land-supply statistics.
    ///
    /// Uses `ak.real_estate_land_supply_em()` which provides city-level
    /// land-supply data from the East Money platform.
    ///
    /// Returns a JSON array string, or `"[]"` if the endpoint is unavailable.
    pub async fn get_land_price_data(&self) -> anyhow::Result<String> {
        let script = r#"
try:
    import akshare as ak, json
    df = ak.real_estate_land_supply_em()
    print(df.to_json(orient="records", force_ascii=False))
except Exception as e:
    import sys, json
    print("[]")
    print(f"WARNING: real_estate_land_supply_em failed: {e}", file=sys.stderr)
"#;

        self.akshare
            .run_python(script)
            .await
            .context("real_estate get_land_price_data failed")
    }
}
