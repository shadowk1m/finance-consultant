use crate::agents::RunState;
use anyhow::Context;
use chrono::Local;
use std::fs;
use std::path::PathBuf;

/// Write both a human-readable Markdown report and a machine-readable JSON
/// state dump for the given `RunState`.
///
/// # Files produced
/// * `{output_dir}/{ticker}_{date}_report.md`  — the Markdown narrative
/// * `{output_dir}/{ticker}_{date}_state.json` — the full `RunState` as JSON
///
/// # Returns
/// The path to the `.md` file.
pub fn write_report(state: &RunState, output_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    // Ensure the output directory exists.
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output directory: {}", output_dir.display()))?;

    let ticker = &state.ticker;
    let date = &state.trade_date;

    // ── Markdown report ──────────────────────────────────────────────────────
    let md_path = output_dir.join(format!("{ticker}_{date}_report.md"));
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S %z").to_string();
    let badge = format_rating_badge(&state.final_rating);

    let mut md = String::new();
    md.push_str(&format!(
        "# Financial Analysis Report — {ticker} ({date})\n\n"
    ));

    md.push_str(&format!("**Asset type:** {}  \n", state.asset_type));
    md.push_str(&format!("**Rating:** {badge}  \n\n"));
    md.push_str("---\n\n");

    md.push_str("## Market Analysis\n\n");
    md.push_str(&state.analyst_reports.market);
    md.push_str("\n\n");

    md.push_str("## News & Macro Analysis\n\n");
    md.push_str(&state.analyst_reports.news);
    md.push_str("\n\n");

    md.push_str("## Fundamentals Analysis\n\n");
    md.push_str(&state.analyst_reports.fundamentals);
    md.push_str("\n\n");

    md.push_str("## Sentiment Analysis\n\n");
    md.push_str(&state.analyst_reports.sentiment);
    md.push_str("\n\n");

    if let Some(re) = &state.analyst_reports.real_estate {
        if !re.is_empty() {
            md.push_str("## Real Estate Analysis\n\n");
            md.push_str(re);
            md.push_str("\n\n");
        }
    }

    md.push_str("## Investment Debate\n\n");
    md.push_str("### Bull Case\n\n");
    md.push_str(&state.debate_state.bull_history);
    md.push_str("\n\n");
    md.push_str("### Bear Case\n\n");
    md.push_str(&state.debate_state.bear_history);
    md.push_str("\n\n");
    md.push_str("### Investment Plan\n\n");
    md.push_str(&state.debate_state.investment_plan);
    md.push_str("\n\n");

    md.push_str("## Risk Assessment\n\n");
    md.push_str(&state.risk_state.risk_assessment);
    md.push_str("\n\n");

    md.push_str("## Trader Plan\n\n");
    md.push_str(&state.trader_plan);
    md.push_str("\n\n");

    md.push_str("## Final Decision\n\n");
    md.push_str(&state.final_decision);
    md.push_str("\n\n");
    md.push_str(&format!("**Rating: {badge}**\n\n"));
    md.push_str(&format!("---\n\n*Generated: {timestamp}*\n"));

    fs::write(&md_path, &md)
        .with_context(|| format!("failed to write report: {}", md_path.display()))?;

    // ── JSON state dump ──────────────────────────────────────────────────────
    let json_path = output_dir.join(format!("{ticker}_{date}_state.json"));
    let json = serde_json::to_string_pretty(state).context("failed to serialise RunState")?;

    fs::write(&json_path, &json)
        .with_context(|| format!("failed to write state JSON: {}", json_path.display()))?;

    tracing::info!(
        "Report written to {} (JSON: {})",
        md_path.display(),
        json_path.display()
    );

    Ok(md_path)
}

/// Return an ASCII/emoji badge for a rating string.
///
/// Recognised values (case-insensitive):
/// `"Strong Buy"` → 🟢 Strong Buy  
/// `"Buy"`        → 🟩 Buy  
/// `"Hold"`       → 🟡 Hold  
/// `"Sell"`       → 🟧 Sell  
/// `"Strong Sell"`→ 🔴 Strong Sell  
pub fn format_rating_badge(rating: &str) -> &'static str {
    match rating.to_lowercase().trim() {
        "strong buy" => "🟢 Strong Buy",
        "buy" => "🟩 Buy",
        "hold" => "🟡 Hold",
        "sell" => "🟧 Sell",
        "strong sell" => "🔴 Strong Sell",
        _ => "⚪ Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_badges_are_correct() {
        assert_eq!(format_rating_badge("Strong Buy"), "🟢 Strong Buy");
        assert_eq!(format_rating_badge("Buy"), "🟩 Buy");
        assert_eq!(format_rating_badge("Hold"), "🟡 Hold");
        assert_eq!(format_rating_badge("Sell"), "🟧 Sell");
        assert_eq!(format_rating_badge("Strong Sell"), "🔴 Strong Sell");
        assert_eq!(format_rating_badge("unknown"), "⚪ Unknown");
    }
}
