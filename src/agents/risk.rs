#![allow(dead_code)]

use crate::llm::{LlmClient, Message};
use super::{AnalystReports, DebateState, RiskState};

/// Runs all three risk-manager perspectives (aggressive, conservative, neutral)
/// and synthesises a final risk assessment.
pub async fn run_risk_managers(
    llm: &LlmClient,
    model: &str,
    reports: &AnalystReports,
    debate: &DebateState,
    ticker: &str,
    language: &str,
) -> anyhow::Result<RiskState> {
    let context = format!(
        "## Analyst Reports\nMarket: {}\nFundamentals: {}\nNews: {}\nSentiment: {}\n\n\
         ## Debate\nBull: {}\nBear: {}\n\nInvestment Plan: {}",
        reports.market,
        reports.fundamentals,
        reports.news,
        reports.sentiment,
        debate.bull_history,
        debate.bear_history,
        debate.investment_plan,
    );

    let styles = ["aggressive", "conservative", "neutral"];
    let mut histories = ["", "", ""].map(String::from);

    for (i, style) in styles.iter().enumerate() {
        let prompt = crate::llm::prompts::system_risk_manager(style, language);
        let messages = vec![
            Message::system(prompt),
            Message::user(format!(
                "Here is the research context for {ticker}:\n\n{context}\n\n\
                 Provide your {style} risk assessment."
            )),
        ];
        histories[i] = llm.chat(model, &messages, 0.3).await?;
    }

    // Synthesise
    let synthesis_messages = vec![
        Message::system(format!(
            "You are a senior risk officer. Synthesise the three risk perspectives below \
             into a single, actionable risk assessment for {ticker}. Write in {language}."
        )),
        Message::user(format!(
            "Aggressive risk manager:\n{}\n\nConservative risk manager:\n{}\n\n\
             Neutral risk manager:\n{}\n\nWrite the consolidated risk assessment.",
            histories[0], histories[1], histories[2],
        )),
    ];
    let risk_assessment = llm.chat(model, &synthesis_messages, 0.3).await?;

    Ok(RiskState {
        aggressive_history: histories[0].clone(),
        conservative_history: histories[1].clone(),
        neutral_history: histories[2].clone(),
        risk_assessment,
    })
}
