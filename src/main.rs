//! `fc` — AI-powered financial consultant for China markets.
//!
//! Usage:
//! ```
//! fc analyze --ticker 600519 --date 2026-10-01
//! fc config
//! ```

#![allow(unused_imports)]

pub mod agents;
pub mod cli;
pub mod config;
pub mod data;
pub mod error;
pub mod llm;
pub mod report;

use anyhow::Context;
use chrono::{Duration, Local, NaiveDate};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    agents::{
        analysts::{
            run_fundamentals_analyst, run_market_analyst, run_news_analyst,
            run_real_estate_analyst, run_sentiment_analyst,
        },
        debate::run_debate,
        portfolio::run_portfolio_manager,
        risk::run_risk_managers,
        AnalystReports, RunState,
    },
    cli::{chat::ChatSession, display::AppDisplay, input},
    config::{AssetType, Config},
    data::DataRouter,
    llm::LlmClient,
};

// ─── CLI argument structures ─────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "fc",
    about = "AI-powered financial consultant for China markets",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a stock or real estate asset
    Analyze {
        /// Ticker symbol (e.g. 600519 for Moutai, or "上海浦东" for real estate)
        #[arg(short, long)]
        ticker: Option<String>,

        /// Analysis date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        date: Option<String>,

        /// Asset type: stock or real_estate
        #[arg(short = 'a', long, default_value = "stock")]
        asset_type: String,

        /// Skip interactive chat after analysis
        #[arg(long)]
        no_chat: bool,

        /// Output language (Chinese or English)
        #[arg(short, long)]
        language: Option<String>,
    },

    /// Show the current configuration
    Config,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise tracing from `RUST_LOG` (e.g. `RUST_LOG=info fc analyze …`).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::load().context("failed to load configuration")?;
    let display = AppDisplay::new();

    match cli.command {
        // ── fc config ─────────────────────────────────────────────────────────
        Some(Commands::Config) => {
            display.print_header("Finance Consultant — Configuration");
            let cfg_json = serde_json::json!({
                "llm_provider":      config.llm_provider,
                "llm_base_url":      config.llm_base_url,
                "deep_think_model":  config.deep_think_model,
                "quick_think_model": config.quick_think_model,
                "api_key":           if config.api_key.is_empty() { "<not set>" } else { "<set>" },
                "tushare_token":     config.tushare_token.as_deref().map(|_| "<set>").unwrap_or("<not set>"),
                "akshare_python":    config.akshare_python,
                "output_language":   config.output_language,
                "max_debate_rounds": config.max_debate_rounds,
                "max_risk_rounds":   config.max_risk_rounds,
                "results_dir":       config.results_dir.display().to_string(),
                "data_cache_dir":    config.data_cache_dir.display().to_string(),
                "asset_type":        config.asset_type.to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&cfg_json)?);
        }

        // ── fc analyze ────────────────────────────────────────────────────────
        Some(Commands::Analyze {
            ticker,
            date,
            asset_type,
            no_chat,
            language,
        }) => {
            run_analyze(
                ticker,
                date,
                Some(asset_type),
                no_chat,
                language,
                false,
                &config,
                &display,
            )
            .await?;
        }

        // ── no subcommand — interactive setup ────────────────────────────────
        None => {
            run_analyze(None, None, None, false, None, true, &config, &display).await?;
        }
    }

    Ok(())
}

/// Core analysis flow, shared between `fc analyze` and the interactive path.
#[allow(clippy::too_many_arguments)]
async fn run_analyze(
    ticker_arg: Option<String>,
    date_arg: Option<String>,
    asset_type_arg: Option<String>,
    no_chat_arg: bool,
    language_arg: Option<String>,
    interactive: bool,
    config: &Config,
    display: &AppDisplay,
) -> anyhow::Result<()> {
    // ── Collect parameters ─────────────────────────────────────────────────
    let ticker = resolve_ticker(ticker_arg, interactive)?;
    let date = resolve_date(date_arg, interactive)?;
    let asset_type_str = resolve_asset_type(asset_type_arg, interactive)?;
    let language = resolve_language(language_arg, config, interactive)?;
    let no_chat = if interactive {
        !input::confirm("Start interactive chat after analysis?")?
    } else {
        no_chat_arg
    };

    let asset_type = match asset_type_str.to_lowercase().as_str() {
        "real_estate" | "realestate" => AssetType::RealEstate,
        _ => AssetType::Stock,
    };

    display.print_header("Finance Consultant — Analysis");
    display.print_status(&format!(
        "Ticker: {}  |  Date: {}  |  Asset: {}  |  Language: {}",
        ticker, date, asset_type, language
    ));
    println!();

    // ── Set up clients ─────────────────────────────────────────────────────
    let base_url = config
        .llm_base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com".to_string());
    let llm = LlmClient::new(&base_url, &config.api_key);
    let router = DataRouter::new(config);

    let quick_model = &config.quick_think_model;
    let deep_model = &config.deep_think_model;

    // ── Compute date range ─────────────────────────────────────────────────
    let (start_date, end_date) = compute_date_range(&date, &asset_type);

    // ── Step 1: Fetch all data concurrently ────────────────────────────────
    let (price_res, news_res, fundamentals_res, macro_res, re_res) = {
        let sp = display.spinner("Fetching market data…");
        let results = tokio::join!(
            router.get_price_data(&ticker, &start_date, &end_date),
            router.get_news(&ticker, &start_date, &end_date),
            router.get_fundamentals(&ticker),
            router.get_macro_data(),
            fetch_real_estate_if_needed(&router, &asset_type, &ticker),
        );
        sp.finish_and_clear();
        results
    };

    let price_data = unwrap_or_placeholder(price_res, "price_data");
    let news_data = unwrap_or_placeholder(news_res, "news_data");
    let fundamentals_data = unwrap_or_placeholder(fundamentals_res, "fundamentals_data");
    let macro_data = unwrap_or_placeholder(macro_res, "macro_data");
    let re_data: Option<String> = re_res.map(|r| unwrap_or_placeholder(r, "real_estate_data"));

    println!("✅  Market data fetched");

    // ── Step 2: Run analyst team concurrently ──────────────────────────────
    let analyst_steps: u64 = if asset_type == AssetType::RealEstate { 5 } else { 4 };
    let pb = display.progress_bar(analyst_steps, "Running analyst team");

    let (market_res, news_analyst_res, fundamentals_res2, sentiment_res) = tokio::join!(
        run_market_analyst(&llm, quick_model, &price_data, &ticker, &date, &language),
        run_news_analyst(
            &llm,
            quick_model,
            &news_data,
            &macro_data,
            &ticker,
            &date,
            &language
        ),
        run_fundamentals_analyst(
            &llm,
            quick_model,
            &fundamentals_data,
            &ticker,
            &date,
            &language
        ),
        run_sentiment_analyst(&llm, quick_model, &news_data, &ticker, &date, &language),
    );

    let market_report = unwrap_or_placeholder(market_res, "market_analyst");
    pb.inc(1);
    println!("✅  Market Analysis complete");

    let news_report = unwrap_or_placeholder(news_analyst_res, "news_analyst");
    pb.inc(1);
    println!("✅  News & Macro Analysis complete");

    let fundamentals_report = unwrap_or_placeholder(fundamentals_res2, "fundamentals_analyst");
    pb.inc(1);
    println!("✅  Fundamentals Analysis complete");

    let sentiment_report = unwrap_or_placeholder(sentiment_res, "sentiment_analyst");
    pb.inc(1);
    println!("✅  Sentiment Analysis complete");

    // Real-estate analyst (optional)
    let real_estate_report = if let Some(re_text) = re_data {
        let re_res = run_real_estate_analyst(
            &llm,
            quick_model,
            &re_text,
            &ticker,
            &date,
            &language,
        )
        .await;
        pb.inc(1);
        let rep = unwrap_or_placeholder(re_res, "real_estate_analyst");
        println!("✅  Real Estate Analysis complete");
        Some(rep)
    } else {
        None
    };

    pb.finish_and_clear();

    let reports = AnalystReports {
        market: market_report,
        news: news_report,
        fundamentals: fundamentals_report,
        sentiment: sentiment_report,
        real_estate: real_estate_report,
    };

    // ── Step 3: Investment debate ──────────────────────────────────────────
    display.print_status("Running investment debate…");
    let debate = run_debate(
        &llm,
        deep_model,
        &reports,
        &ticker,
        config.max_debate_rounds,
        &language,
    )
    .await
    .unwrap_or_else(|e| {
        display.print_error(&format!("Debate failed: {e}"));
        Default::default()
    });
    println!("✅  Investment debate complete");

    // ── Step 4: Risk panel ─────────────────────────────────────────────────
    display.print_status("Running risk assessment panel…");
    let risk = run_risk_managers(&llm, deep_model, &reports, &debate, &ticker, &language)
        .await
        .unwrap_or_else(|e| {
            display.print_error(&format!("Risk panel failed: {e}"));
            Default::default()
        });
    println!("✅  Risk assessment complete");

    // ── Step 5: Portfolio manager final decision ───────────────────────────
    display.print_status("Portfolio Manager making final decision…");
    let (final_decision, final_rating) =
        run_portfolio_manager(&llm, deep_model, &reports, &debate, &risk, &ticker, &language)
            .await
            .unwrap_or_else(|e| {
                display.print_error(&format!("Portfolio manager failed: {e}"));
                (format!("DECISION_UNAVAILABLE: {e}"), "Hold".to_string())
            });
    println!("✅  Final decision made");

    // ── Build and persist RunState ─────────────────────────────────────────
    let state = RunState {
        ticker: ticker.clone(),
        trade_date: date.clone(),
        asset_type: asset_type.to_string(),
        analyst_reports: reports,
        debate_state: debate,
        risk_state: risk,
        trader_plan: String::new(),
        final_decision: final_decision.clone(),
        final_rating: final_rating.clone(),
    };

    let output_dir = config.results_path(&ticker, &date);
    match report::write_report(&state, &output_dir) {
        Ok(path) => println!("📄  Report saved to {}", path.display()),
        Err(e) => display.print_error(&format!("Failed to save report: {e}")),
    }

    // ── Display final rating banner ────────────────────────────────────────
    display.print_rating(&ticker, &final_rating);
    println!();
    display.render_markdown("### Final Decision\n");
    display.render_markdown(&final_decision);
    println!();

    // ── Step 6: Interactive chat (unless --no-chat) ────────────────────────
    if !no_chat {
        let mut chat = ChatSession::new(llm, quick_model, state);
        chat.run().await?;
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute `(start_date, end_date)` in `"YYYYMMDD"` format.
///
/// * Stocks: lookback 30 days.
/// * Real estate: lookback 90 days.
fn compute_date_range(date: &str, asset_type: &AssetType) -> (String, String) {
    let lookback_days: i64 = match asset_type {
        AssetType::RealEstate => 90,
        AssetType::Stock => 30,
    };

    let end = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let start = end - Duration::days(lookback_days);

    let fmt = |d: NaiveDate| d.format("%Y%m%d").to_string();
    (fmt(start), fmt(end))
}

/// Return `Ok(value)` or a placeholder string `"DATA_UNAVAILABLE: <err>"`.
fn unwrap_or_placeholder(result: anyhow::Result<String>, label: &str) -> String {
    result.unwrap_or_else(|e| {
        tracing::warn!("{label} failed: {e:#}");
        format!("DATA_UNAVAILABLE ({label}): {e}")
    })
}

/// Fetch real-estate data only when the asset type warrants it.
/// Returns `None` for non-real-estate assets, otherwise `Some(Result<String>)`.
async fn fetch_real_estate_if_needed(
    router: &DataRouter,
    asset_type: &AssetType,
    ticker: &str,
) -> Option<anyhow::Result<String>> {
    if *asset_type != AssetType::RealEstate {
        return None;
    }
    let (news_res, price_res) = tokio::join!(
        router.get_real_estate_data("news"),
        router.get_real_estate_data(ticker),
    );
    let news = news_res.unwrap_or_else(|e| format!("RE_NEWS_UNAVAILABLE: {e}"));
    let price = price_res.unwrap_or_else(|e| format!("RE_PRICE_UNAVAILABLE: {e}"));
    Some(Ok(format!(
        "## Real Estate News\n{news}\n\n## Price Index\n{price}"
    )))
}

/// Interactive or flag-driven ticker resolution.
fn resolve_ticker(ticker: Option<String>, interactive: bool) -> anyhow::Result<String> {
    if let Some(t) = ticker {
        return Ok(t);
    }
    if interactive {
        loop {
            let t = input::prompt("Ticker symbol (e.g. 600519)")?;
            if !t.is_empty() {
                return Ok(t);
            }
            eprintln!("  Ticker cannot be empty.");
        }
    }
    anyhow::bail!(
        "No ticker provided. Use --ticker or omit the subcommand for interactive mode."
    );
}

/// Interactive or flag-driven date resolution.
fn resolve_date(date: Option<String>, interactive: bool) -> anyhow::Result<String> {
    let default_date = Local::now().format("%Y-%m-%d").to_string();
    if let Some(d) = date {
        return Ok(d);
    }
    if interactive {
        return input::prompt_with_default("Analysis date (YYYY-MM-DD)", &default_date);
    }
    Ok(default_date)
}

/// Interactive or flag-driven asset type resolution.
fn resolve_asset_type(
    asset_type: Option<String>,
    interactive: bool,
) -> anyhow::Result<String> {
    if let Some(a) = asset_type {
        return Ok(a);
    }
    if interactive {
        let idx = input::select("Asset type", &["stock", "real_estate"])?;
        return Ok(["stock", "real_estate"][idx].to_string());
    }
    Ok("stock".to_string())
}

/// Interactive or flag-driven language resolution.
fn resolve_language(
    language: Option<String>,
    config: &Config,
    interactive: bool,
) -> anyhow::Result<String> {
    if let Some(l) = language {
        return Ok(l);
    }
    if interactive {
        return input::prompt_with_default("Output language", &config.output_language);
    }
    Ok(config.output_language.clone())
}
