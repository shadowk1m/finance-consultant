#![allow(dead_code)]

use crate::llm::{LlmClient, Message};
use super::AnalystReports;

/// Formats all available analyst reports into a single context block for the debaters.
fn format_reports(reports: &AnalystReports, ticker: &str) -> String {
    let mut out = format!("=== Analyst Reports for {ticker} ===\n\n");

    out.push_str("## Market Analysis\n");
    out.push_str(&reports.market);
    out.push_str("\n\n");

    out.push_str("## News & Macro Analysis\n");
    out.push_str(&reports.news);
    out.push_str("\n\n");

    out.push_str("## Fundamentals Analysis\n");
    out.push_str(&reports.fundamentals);
    out.push_str("\n\n");

    out.push_str("## Sentiment Analysis\n");
    out.push_str(&reports.sentiment);

    if let Some(re) = &reports.real_estate {
        out.push_str("\n\n## Real Estate Analysis\n");
        out.push_str(re);
    }

    out
}

/// Runs a multi-round bull-vs-bear debate grounded in the analyst reports,
/// then synthesises the debate into an investment plan.
///
/// Each round the bull responds first, then the bear.  Both agents receive
/// the full conversation history so arguments build on each other.
/// After all rounds a "Research Manager" synthesises the debate.
pub async fn run_debate(
    llm: &LlmClient,
    model: &str,
    reports: &AnalystReports,
    ticker: &str,
    rounds: usize,
    language: &str,
) -> anyhow::Result<super::DebateState> {
    let context = format_reports(reports, ticker);

    // Separate accumulators for the readable history strings returned to callers.
    let mut bull_history = String::new();
    let mut bear_history = String::new();

    // Shared conversation that both agents append to, giving each debater full
    // visibility of the opposing arguments.
    let mut conversation: Vec<Message> = Vec::new();

    let bull_system = format!(
        "You are a bullish investment analyst debating whether to invest in {ticker}. \
         Draw on the provided research reports to make the strongest possible case FOR \
         investing. Respond concisely and persuasively. Write in {language}."
    );

    let bear_system = format!(
        "You are a bearish investment analyst debating whether to invest in {ticker}. \
         Draw on the provided research reports to make the strongest possible case AGAINST \
         investing. Respond concisely and persuasively. Write in {language}."
    );

    // Prime both debaters with the research context as the opening user turn.
    let opening = format!(
        "Here are the analyst reports:\n\n{context}\n\n\
         Begin the investment debate for {ticker}."
    );

    for round in 1..=rounds {
        // ── Bull turn ──────────────────────────────────────────────────────────
        let mut bull_messages = vec![Message::system(&bull_system)];
        // Give the bull the full shared conversation so far, plus this round's prompt.
        bull_messages.extend(conversation.clone());
        if round == 1 {
            bull_messages.push(Message::user(&opening));
        } else {
            bull_messages.push(Message::user(format!(
                "Round {round}: Continue your bullish argument, addressing the bear's \
                 most recent points."
            )));
        }

        let bull_reply = llm.chat(model, &bull_messages, 0.7).await?;

        let bull_turn = format!("[Bull – Round {round}]: {bull_reply}");
        bull_history.push_str(&bull_turn);
        bull_history.push('\n');

        // Append to shared conversation so the bear can see it.
        conversation.push(Message::user(format!(
            "Bull analyst (round {round}):\n{bull_reply}"
        )));

        // ── Bear turn ──────────────────────────────────────────────────────────
        let mut bear_messages = vec![Message::system(&bear_system)];
        bear_messages.extend(conversation.clone());
        bear_messages.push(Message::user(format!(
            "Round {round}: Respond to the bull's argument and strengthen the bearish case."
        )));

        let bear_reply = llm.chat(model, &bear_messages, 0.7).await?;

        let bear_turn = format!("[Bear – Round {round}]: {bear_reply}");
        bear_history.push_str(&bear_turn);
        bear_history.push('\n');

        // Append bear reply to shared conversation.
        conversation.push(Message::user(format!(
            "Bear analyst (round {round}):\n{bear_reply}"
        )));
    }

    // ── Research Manager synthesis ─────────────────────────────────────────────
    let synthesis_messages = vec![
        Message::system(format!(
            "You are a research manager synthesising a bull-vs-bear debate about {ticker}. \
             Weigh the arguments from both sides, highlight the strongest points, identify key \
             uncertainties, and produce a balanced investment plan with a clear recommended \
             action (Buy / Hold / Sell) and rationale. Write in {language}."
        )),
        Message::user(format!(
            "Original analyst reports:\n\n{context}\n\n\
             Debate transcript:\n\nBull arguments:\n{bull_history}\n\nBear arguments:\n{bear_history}\n\n\
             Synthesise the debate and write the final investment plan."
        )),
    ];

    let investment_plan = llm.chat(model, &synthesis_messages, 0.3).await?;

    Ok(super::DebateState {
        bull_history,
        bear_history,
        investment_plan,
    })
}
