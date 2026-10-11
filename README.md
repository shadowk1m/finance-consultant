# 💹 finance-consultant

An AI-powered, multi-agent financial consultant built in **Rust** — designed from the ground up for **China markets** with an **interactive CLI chat** interface.

Inspired by [TradingAgents](https://github.com/TauricResearch/TradingAgents), rebuilt in Rust with:
- 🇨🇳 China-accessible data vendors (AKShare, Tushare, BaoStock)
- 🏠 China Real Estate analysis
- 💬 Interactive chat after report generation
- ⚡ Concurrent analyst execution via Tokio
- 🦀 Single statically-linked binary — no Python runtime needed for the core

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   fc (CLI binary)                   │
│                                                     │
│  Interactive setup → Analyze → Chat                 │
└────────────────────────┬────────────────────────────┘
                         │
         ┌───────────────▼───────────────┐
         │         Data Router           │
         │  Tushare → AKShare → BaoStock │
         │  (+ AKShare Real Estate)      │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │      Analyst Team (parallel)  │
         │  Market | News | Fundamentals │
         │  Sentiment | Real Estate      │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │    Investment Debate (LLM)    │
         │    Bull ↔ Bear → Research Mgr │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │      Risk Panel (LLM)         │
         │  Aggressive / Conservative    │
         │  Neutral → Risk Assessment    │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │    Portfolio Manager (LLM)    │
         │    Final Decision + Rating    │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │   Interactive Chat Session    │
         │   Ask follow-up questions     │
         └───────────────────────────────┘
```

**Ratings:** 🟢 Strong Buy | 🟩 Buy | 🟡 Hold | 🟧 Sell | 🔴 Strong Sell

---

## Supported Asset Types

### 📈 A-Share Stocks
- Tickers: `600519`(茅台), `000858`(五粮液), `300750`(宁德时代), etc.
- Data: price history, financials, news, macro

### 🏠 China Real Estate
- Input the city name or developer name (e.g. `上海`, `万科A`, `碧桂园`)
- Data: house price index, land supply, real estate news, policy environment

---

## Quick Start

### 1. Prerequisites

```bash
# Python deps for AKShare/BaoStock data (required)
pip install akshare baostock pandas

# Get a free Tushare token at https://tushare.pro (optional, improves data quality)
```

### 2. Install

```bash
# Clone and build
git clone https://github.com/shadowk1m/finance-consultant.git
cd finance-consultant
cargo build --release

# The binary is at ./target/release/fc
# Optionally: cp target/release/fc /usr/local/bin/fc
```

### 3. Configure

```bash
cp .env.example .env
# Edit .env with your LLM API key and data vendor tokens
```

**Recommended LLM for China users:**

```env
LLM_API_KEY=sk-...           # DeepSeek API key
LLM_BASE_URL=https://api.deepseek.com
DEEP_THINK_MODEL=deepseek-reasoner
QUICK_THINK_MODEL=deepseek-chat
```

Other options: Kimi (Moonshot), Qwen (DashScope), or any OpenAI-compatible endpoint.

### 4. Run

```bash
# Interactive mode (prompts for all inputs)
fc

# Direct mode
fc analyze --ticker 600519 --date 2024-01-15

# Real estate mode
fc analyze --ticker 上海 --asset-type real_estate

# English output
fc analyze --ticker 600519 --language English

# Analysis only, skip interactive chat
fc analyze --ticker 600519 --no-chat

# Show current config
fc config
```

---

## Data Vendors

| Vendor | Asset | Data | Access |
|--------|-------|------|--------|
| **AKShare** | A-shares, RE | Price, news, macro, RE data | Free, China ✅ |
| **Tushare** | A-shares | Price, financials, news | Token (free tier), China ✅ |
| **BaoStock** | A-shares | Historical OHLCV | Free, China ✅ |

The router tries Tushare first (best quality), falls back to AKShare, then BaoStock. No Yahoo Finance or Alpha Vantage — both blocked in China.

---

## Supported LLM Providers

Any **OpenAI-compatible** API endpoint works. Set `LLM_BASE_URL` accordingly:

| Provider | Base URL | Notes |
|----------|----------|-------|
| DeepSeek | `https://api.deepseek.com` | Best value, strong reasoning |
| Kimi (Moonshot) | `https://api.moonshot.cn/v1` | Good Chinese language quality |
| Qwen (Alibaba) | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Strong on Chinese finance |
| OpenAI | `https://api.openai.com/v1` | Requires VPN in China |

---

## Report Output

Reports are saved to `~/.finance-consultant/reports/{ticker}/{date}/`:
- `{ticker}_{date}_report.md` — full Markdown analysis report
- `{ticker}_{date}_state.json` — machine-readable state for programmatic use

---

## Interactive Chat

After analysis completes, you enter a chat session:

```
💬 Analysis complete! You can now ask questions about the report.
Type 'exit' to quit, 'report' to re-display the final decision.

You: 这个股票的主要风险是什么？
Assistant: 根据本次分析，主要风险包括...

You: 和同行业相比估值如何？
Assistant: 从基本面分析来看...

You: exit
```

The chat has the full analysis context — all analyst reports, debate history, and final decision — so it can answer detailed follow-up questions.

---

## Configuration File

You can also configure via `~/.finance-consultant/config.toml`:

```toml
llm_provider = "deepseek"
llm_base_url = "https://api.deepseek.com"
deep_think_model = "deepseek-reasoner"
quick_think_model = "deepseek-chat"
output_language = "Chinese"
max_debate_rounds = 2
max_risk_rounds = 1
```

Environment variables override the config file. See `.env.example` for all options.

---

## Contributing

PRs welcome! Key areas for contribution:
- Additional China data vendors (EastMoney/东方财富, Wind, iFinD)
- Hong Kong & Taiwan market support  
- Backtesting framework
- Web UI / API server mode
- More real estate data sources (贝壳, 链家, NBS 国家统计局)

---

## Disclaimer

This tool is for **research and educational purposes only**. It is not financial advice. Trading decisions based on AI analysis carry significant risk. Always do your own due diligence.

---

## License

MIT
