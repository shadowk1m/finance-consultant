#![allow(dead_code)]

use crate::llm::{LlmClient, Message};

/// Runs the market / technical analyst for the given ticker.
///
/// `price_data` is a pre-fetched string of OHLCV or indicator data.
pub async fn run_market_analyst(
    llm: &LlmClient,
    model: &str,
    price_data: &str,
    ticker: &str,
    date: &str,
    language: &str,
) -> anyhow::Result<String> {
    let messages = vec![
        Message::system(format!(
            "You are a market analyst. Analyze the price/volume data for {ticker} as of {date}. \
             Identify trends, support/resistance levels, and technical signals. \
             Write your report in {language}."
        )),
        Message::user(format!(
            "Price data:\n{price_data}\n\nWrite a comprehensive market analysis report."
        )),
    ];

    llm.chat(model, &messages, 0.3).await
}

/// Runs the news & macro analyst for the given ticker.
///
/// `news_data` is a pre-fetched news string; `macro_data` is macro-economic context.
pub async fn run_news_analyst(
    llm: &LlmClient,
    model: &str,
    news_data: &str,
    macro_data: &str,
    ticker: &str,
    date: &str,
    language: &str,
) -> anyhow::Result<String> {
    let messages = vec![
        Message::system(format!(
            "You are a financial news and macroeconomic analyst. \
             Analyze the latest news and macro environment for {ticker} as of {date}. \
             Assess how current events, central bank policy, geopolitical risks, and sector \
             developments may affect the asset. Write your report in {language}."
        )),
        Message::user(format!(
            "News data:\n{news_data}\n\nMacro data:\n{macro_data}\n\n\
             Write a comprehensive news and macro analysis report."
        )),
    ];

    llm.chat(model, &messages, 0.3).await
}

/// Runs the fundamentals analyst for the given ticker.
///
/// `fundamentals_data` is a pre-fetched string of financial-statement or valuation data.
pub async fn run_fundamentals_analyst(
    llm: &LlmClient,
    model: &str,
    fundamentals_data: &str,
    ticker: &str,
    date: &str,
    language: &str,
) -> anyhow::Result<String> {
    let messages = vec![
        Message::system(format!(
            "You are a fundamental analyst. Analyze the financial statements and valuation \
             metrics for {ticker} as of {date}. Cover revenue growth, profitability, balance \
             sheet strength, cash flow, and relative valuation (P/E, P/B, EV/EBITDA). \
             Highlight key risks and catalysts. Write your report in {language}."
        )),
        Message::user(format!(
            "Fundamentals data:\n{fundamentals_data}\n\n\
             Write a comprehensive fundamental analysis report."
        )),
    ];

    llm.chat(model, &messages, 0.3).await
}

/// Runs the sentiment analyst for the given ticker.
///
/// `news_data` is used to gauge social and media tone rather than hard macro facts.
pub async fn run_sentiment_analyst(
    llm: &LlmClient,
    model: &str,
    news_data: &str,
    ticker: &str,
    date: &str,
    language: &str,
) -> anyhow::Result<String> {
    let messages = vec![
        Message::system(format!(
            "You are a market sentiment analyst. Read the news and social-media excerpts for \
             {ticker} as of {date} and gauge the prevailing investor mood. Classify overall \
             sentiment (bullish / neutral / bearish), identify dominant narratives, note any \
             extreme fear or greed signals, and flag sentiment divergences from price action. \
             Write your report in {language}."
        )),
        Message::user(format!(
            "News / social data:\n{news_data}\n\n\
             Write a comprehensive sentiment analysis report."
        )),
    ];

    llm.chat(model, &messages, 0.3).await
}

/// Runs the China real-estate specialist analyst.
///
/// `re_data` is a pre-fetched string covering price indices, land supply, policy updates,
/// and developer financials relevant to the ticker (city / developer / REIT).
pub async fn run_real_estate_analyst(
    llm: &LlmClient,
    model: &str,
    re_data: &str,
    ticker: &str,
    date: &str,
    language: &str,
) -> anyhow::Result<String> {
    let messages = vec![
        Message::system(format!(
            "You are a China real estate market analyst. Analyze the following real estate data \
             for {ticker} (city/developer/REIT) as of {date}. Cover: price trends, land supply, \
             policy (限购/限贷), developer fundamentals, macro environment. \
             Write in {language}."
        )),
        Message::user(format!(
            "Real estate data:\n{re_data}\n\n\
             Write a comprehensive China real estate analysis report."
        )),
    ];

    llm.chat(model, &messages, 0.3).await
}
