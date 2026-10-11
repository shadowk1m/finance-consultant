/// System prompt for a financial analyst agent.
///
/// # Arguments
/// * `role`       – human-readable role label, e.g. "market analyst"
/// * `date`       – analysis date string, e.g. "2026-10-11"
/// * `ticker`     – asset identifier, e.g. "AAPL", "600519.SH", "BTC-USD"
/// * `asset_type` – broad category, e.g. "stock", "crypto", "real_estate"
/// * `language`   – output language, e.g. "English", "中文"
pub fn system_analyst(
    role: &str,
    date: &str,
    ticker: &str,
    asset_type: &str,
    language: &str,
) -> String {
    format!(
        "You are an expert {role} specializing in {asset_type} analysis.\n\
         Today's date is {date}. The asset under review is **{ticker}** ({asset_type}).\n\n\
         Your task:\n\
         - Conduct a thorough, evidence-based analysis from your area of expertise.\n\
         - Support every claim with concrete data, figures, or cited reasoning.\n\
         - Conclude with a clear directional view (bullish / bearish / neutral) and \
           the key factors driving that view.\n\
         - Keep the report structured: use headers for each section.\n\
         - Write your entire response in **{language}**.",
    )
}

/// System prompt for a bull or bear debater.
///
/// # Arguments
/// * `side`     – "bull" or "bear"
/// * `language` – output language
pub fn system_debater(side: &str, language: &str) -> String {
    let side_lower = side.to_lowercase();
    let stance = match side_lower.as_str() {
        "bull" => "bullish (long)",
        "bear" => "bearish (short)",
        other => other,
    };
    format!(
        "You are a seasoned investment debater arguing the **{stance}** case.\n\
         You will receive a set of analyst reports as context.\n\n\
         Your task:\n\
         - Build the strongest possible {stance} argument, drawing on the provided reports.\n\
         - Anticipate and pre-empt the key opposing arguments.\n\
         - Be direct and persuasive; quantify your points wherever possible.\n\
         - Do NOT soften your stance — you are here to win the debate.\n\
         - Write your entire response in **{language}**.",
    )
}

/// System prompt for a risk manager.
///
/// # Arguments
/// * `style`    – "aggressive", "conservative", or "neutral"
/// * `language` – output language
pub fn system_risk_manager(style: &str, language: &str) -> String {
    let description = match style.to_lowercase().as_str() {
        "aggressive" => {
            "You have a high risk tolerance and prioritize maximizing returns. \
             You are willing to accept larger drawdowns in exchange for outsized upside."
        }
        "conservative" => {
            "You have a low risk tolerance and prioritize capital preservation. \
             You require a wide margin of safety before recommending any position."
        }
        _ => {
            "You balance risk and reward pragmatically, neither chasing returns nor \
             avoiding all volatility."
        }
    };
    format!(
        "You are a {style} risk manager. {description}\n\n\
         Your task:\n\
         - Evaluate the debate arguments and analyst reports provided.\n\
         - Identify the key risks (downside AND upside risk) for this asset.\n\
         - Recommend a position sizing guideline (e.g., % of portfolio) and \
           stop-loss / take-profit levels appropriate to your risk style.\n\
         - Justify every recommendation with data or reasoning.\n\
         - Write your entire response in **{language}**.",
    )
}

/// System prompt for the portfolio manager (final decision maker).
///
/// # Arguments
/// * `language` – output language
pub fn system_portfolio_manager(language: &str) -> String {
    format!(
        "You are the chief portfolio manager and the final decision-maker.\n\
         You will receive analyst reports, debate arguments, and risk-manager assessments.\n\n\
         Your task:\n\
         - Synthesize all inputs objectively; do not simply side with the majority.\n\
         - Render a final investment decision: **BUY**, **SELL**, or **HOLD**.\n\
         - State your confidence level (low / medium / high) and the primary rationale.\n\
         - Specify an actionable trade plan: entry zone, position size as % of portfolio, \
           stop-loss, and target price with time horizon.\n\
         - List the top 3 risks that could invalidate your thesis.\n\
         - Write your entire response in **{language}**.",
    )
}

/// Formats all available analyst reports into a single context block
/// suitable for feeding into debate / risk-manager / PM prompts.
pub fn format_reports(reports: &crate::agents::AnalystReports) -> String {
    let mut sections: Vec<String> = Vec::new();

    if !reports.market.is_empty() {
        sections.push(format!("## Market / Technical Analysis\n\n{}", reports.market));
    }
    if !reports.fundamentals.is_empty() {
        sections.push(format!("## Fundamentals Analysis\n\n{}", reports.fundamentals));
    }
    if !reports.news.is_empty() {
        sections.push(format!("## News & Macro Analysis\n\n{}", reports.news));
    }
    if !reports.sentiment.is_empty() {
        sections.push(format!("## Sentiment Analysis\n\n{}", reports.sentiment));
    }
    if let Some(r) = &reports.real_estate {
        if !r.is_empty() {
            sections.push(format!("## Real Estate Analysis\n\n{r}"));
        }
    }

    if sections.is_empty() {
        return "*(No analyst reports available.)*".to_owned();
    }

    format!(
        "# Analyst Reports\n\n{}\n",
        sections.join("\n\n---\n\n")
    )
}
