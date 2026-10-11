pub mod analysts;
pub mod debate;
pub mod risk;
pub mod portfolio;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalystReports {
    pub market: String,
    pub news: String,
    pub fundamentals: String,
    pub sentiment: String,
    pub real_estate: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebateState {
    pub bull_history: String,
    pub bear_history: String,
    pub investment_plan: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskState {
    pub aggressive_history: String,
    pub conservative_history: String,
    pub neutral_history: String,
    pub risk_assessment: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunState {
    pub ticker: String,
    pub trade_date: String,
    pub asset_type: String,
    pub analyst_reports: AnalystReports,
    pub debate_state: DebateState,
    pub risk_state: RiskState,
    pub trader_plan: String,
    pub final_decision: String,
    /// One of: "Strong Buy" | "Buy" | "Hold" | "Sell" | "Strong Sell"
    pub final_rating: String,
}
