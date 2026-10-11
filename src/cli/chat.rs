#![allow(dead_code)]

use crate::agents::RunState;
use crate::cli::display::AppDisplay;
use crate::llm::{LlmClient, Message};

/// An interactive post-analysis chat session.
///
/// After the analysis pipeline completes the user can ask follow-up questions
/// about the report.  The full `RunState` is baked into the system prompt so
/// the model always has the complete analysis context available.
pub struct ChatSession {
    llm: LlmClient,
    model: String,
    state: RunState,
    /// Full conversation history sent to the LLM on every turn.
    history: Vec<Message>,
    display: AppDisplay,
}

impl ChatSession {
    /// Create a new `ChatSession`.
    ///
    /// * `llm`   — the LLM client (usually the same one used during analysis)
    /// * `model` — model name to use for chat responses (typically `quick_think_model`)
    /// * `state` — the completed `RunState` that provides all context
    pub fn new(llm: LlmClient, model: &str, state: RunState) -> Self {
        Self {
            llm,
            model: model.to_owned(),
            state,
            history: Vec::new(),
            display: AppDisplay::new(),
        }
    }

    /// Run the interactive chat loop until the user types `exit` or `quit`.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Welcome messages
        println!();
        self.display.render_markdown(
            "💬 **Analysis complete!**  You can now ask questions about the report.",
        );
        self.display.render_markdown(
            "Type `exit` or `quit` to exit.  Type `report` to re-display the final decision.",
        );
        println!();

        // Seed the history with the system prompt (not shown to user).
        let system_prompt = self.build_system_prompt();
        self.history.push(Message::system(system_prompt));

        loop {
            // Read user input.
            let input = match super::input::prompt("You") {
                Ok(s) => s,
                Err(e) => {
                    // EOF (Ctrl-D or piped input) exits quietly; real errors get logged.
                    if e.to_string() != "EOF" {
                        self.display.print_error(&format!("Failed to read input: {e}"));
                    }
                    break;
                }
            };

            let trimmed = input.trim();

            if trimmed.is_empty() {
                continue;
            }

            // Exit commands.
            if matches!(trimmed.to_lowercase().as_str(), "exit" | "quit" | "q") {
                println!("👋  Goodbye!");
                break;
            }

            // Re-print the final decision.
            if trimmed.to_lowercase() == "report" {
                self.display.print_rating(&self.state.ticker, &self.state.final_rating);
                self.display.render_markdown("### Final Decision\n");
                self.display.render_markdown(&self.state.final_decision);
                println!();
                continue;
            }

            // Ask the LLM.
            let spinner = self.display.spinner("Thinking…");
            match self.ask_llm(trimmed).await {
                Ok(response) => {
                    spinner.finish_and_clear();
                    println!();
                    self.display.render_markdown(&response);
                    println!();
                }
                Err(e) => {
                    spinner.finish_and_clear();
                    self.display.print_error(&format!("LLM error: {e}"));
                    // Don't crash — let the user try again.
                }
            }
        }

        Ok(())
    }

    /// Build the system prompt that provides the model with the full analysis
    /// context.  Truncates individual report sections to ~500 characters each
    /// to avoid hitting context limits on cheaper models.
    fn build_system_prompt(&self) -> String {
        let s = &self.state;

        fn truncate(text: &str, max: usize) -> String {
            if text.len() <= max {
                text.to_owned()
            } else {
                format!("{}… [truncated]", &text[..max])
            }
        }

        let mut prompt = format!(
            "You are a financial analysis assistant.\n\
             The following analysis has already been completed for the user.\n\n\
             **Ticker:** {ticker}\n\
             **Date:** {date}\n\
             **Asset type:** {asset_type}\n\n",
            ticker = s.ticker,
            date = s.trade_date,
            asset_type = s.asset_type,
        );

        prompt.push_str("---\n\n## Analyst Reports (summaries)\n\n");
        prompt.push_str(&format!(
            "**Market Analysis:**\n{}\n\n",
            truncate(&s.analyst_reports.market, 500)
        ));
        prompt.push_str(&format!(
            "**News & Macro Analysis:**\n{}\n\n",
            truncate(&s.analyst_reports.news, 500)
        ));
        prompt.push_str(&format!(
            "**Fundamentals Analysis:**\n{}\n\n",
            truncate(&s.analyst_reports.fundamentals, 500)
        ));
        prompt.push_str(&format!(
            "**Sentiment Analysis:**\n{}\n\n",
            truncate(&s.analyst_reports.sentiment, 500)
        ));
        if let Some(re) = &s.analyst_reports.real_estate {
            if !re.is_empty() {
                prompt.push_str(&format!(
                    "**Real Estate Analysis:**\n{}\n\n",
                    truncate(re, 500)
                ));
            }
        }

        prompt.push_str("---\n\n## Debate & Decision\n\n");
        prompt.push_str(&format!(
            "**Investment Plan:**\n{}\n\n",
            truncate(&s.debate_state.investment_plan, 600)
        ));
        prompt.push_str(&format!(
            "**Risk Assessment:**\n{}\n\n",
            truncate(&s.risk_state.risk_assessment, 600)
        ));
        prompt.push_str(&format!(
            "**Final Decision:**\n{}\n\n",
            truncate(&s.final_decision, 800)
        ));
        prompt.push_str(&format!("**Final Rating:** {}\n\n", s.final_rating));

        // Detect language from asset_type / existing reports to set response language.
        let language = detect_language(s);
        prompt.push_str(&format!(
            "---\n\n\
             Answer the user's questions about this analysis. \
             Be concise and cite specific data from the reports when possible. \
             Respond in {language}."
        ));

        prompt
    }

    /// Append `user_input` to the conversation history, call the LLM, append
    /// the assistant response, and return the response text.
    async fn ask_llm(&mut self, user_input: &str) -> anyhow::Result<String> {
        self.history.push(Message::user(user_input));

        let response = self
            .llm
            .chat(&self.model, &self.history, 0.5)
            .await?;

        self.history.push(Message::assistant(&response));

        Ok(response)
    }
}

/// Heuristic: if any report contains Chinese characters, use Chinese; otherwise English.
fn detect_language(state: &RunState) -> &'static str {
    let probe = format!(
        "{}{}{}",
        state.analyst_reports.market,
        state.final_decision,
        state.asset_type,
    );
    if probe.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        "Chinese (中文)"
    } else {
        "English"
    }
}
