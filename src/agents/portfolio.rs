#![allow(dead_code)]

use crate::llm::{LlmClient, Message};
use super::{AnalystReports, DebateState, RiskState};

/// Runs the portfolio manager to produce a final trading decision.
pub async fn run_portfolio_manager(
    llm: &LlmClient,
    model: &str,
    reports: &AnalystReports,
    debate: &DebateState,
    risk: &RiskState,
    ticker: &str,
    language: &str,
) -> anyhow::Result<(String, String)> {
    let context = format!(
        "## Analyst Reports\n\
         Market: {}\nFundamentals: {}\nNews: {}\nSentiment: {}\n\n\
         ## Debate Investment Plan\n{}\n\n\
         ## Risk Assessment\n{}",
        reports.market,
        reports.fundamentals,
        reports.news,
        reports.sentiment,
        debate.investment_plan,
        risk.risk_assessment,
    );

    let system = crate::llm::prompts::system_portfolio_manager(language);
    let messages = vec![
        Message::system(system),
        Message::user(format!(
            "Here is the full research context for {ticker}:\n\n{context}\n\n\
             Render your final investment decision."
        )),
    ];

    let decision_text = llm.chat(model, &messages, 0.2).await?;

    // Derive a concise rating label from the decision text.
    let rating = derive_rating(&decision_text);

    Ok((decision_text, rating))
}

fn derive_rating(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("strong buy") {
        "Strong Buy".into()
    } else if lower.contains("strong sell") {
        "Strong Sell".into()
    } else if lower.contains("**buy**") || lower.contains("buy") {
        "Buy".into()
    } else if lower.contains("**sell**") || lower.contains("sell") {
        "Sell".into()
    } else {
        "Hold".into()
    }
}
