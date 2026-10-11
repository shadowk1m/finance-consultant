#![allow(dead_code)]

use indicatif::{ProgressBar, ProgressStyle};
use termimad::MadSkin;

/// Central display helper.  Holds a pre-configured [`MadSkin`] so colours and
/// styles are applied consistently throughout the session.
pub struct AppDisplay {
    skin: MadSkin,
}

impl AppDisplay {
    /// Create an `AppDisplay` with a colour scheme that works well in typical
    /// CJK (China-appropriate) terminal environments.
    ///
    /// We start from termimad's default skin and then:
    /// * Bright cyan for headers — high contrast on both dark and light backgrounds.
    /// * Bold yellow for inline code / emphasis — easy to read without colour bleeding.
    /// * Plain white body text.
    pub fn new() -> Self {
        let mut skin = MadSkin::default();

        // Headers: bold bright-cyan
        skin.headers[0]
            .set_fg(termimad::crossterm::style::Color::Cyan);
        skin.headers[0].add_attr(termimad::crossterm::style::Attribute::Bold);

        skin.headers[1]
            .set_fg(termimad::crossterm::style::Color::Cyan);

        skin.headers[2]
            .set_fg(termimad::crossterm::style::Color::Blue);

        // Bold inline text → yellow so it pops without clashing with headers
        skin.bold
            .set_fg(termimad::crossterm::style::Color::Yellow);
        skin.bold.add_attr(termimad::crossterm::style::Attribute::Bold);

        // Italic → dim white
        skin.italic
            .set_fg(termimad::crossterm::style::Color::White);
        skin.italic.add_attr(termimad::crossterm::style::Attribute::Italic);

        // Inline code → bright green
        skin.inline_code
            .set_fg(termimad::crossterm::style::Color::Green);

        Self { skin }
    }

    /// Print a Markdown string to stdout, using the configured skin.
    pub fn render_markdown(&self, md: &str) {
        self.skin.print_text(md);
    }

    /// Print a prominent bordered header.
    ///
    /// ```text
    /// ══════════════════════════════════
    ///   Finance Consultant — Analysis
    /// ══════════════════════════════════
    /// ```
    pub fn print_header(&self, title: &str) {
        let width = 56usize;
        let border = "═".repeat(width);
        let padding = (width.saturating_sub(title.len() + 2)) / 2;
        let pad = " ".repeat(padding);
        println!("\n\x1b[1;36m{border}\x1b[0m");
        println!("\x1b[1;36m{pad} {title} {pad}\x1b[0m");
        println!("\x1b[1;36m{border}\x1b[0m\n");
    }

    /// Print a status line prefixed with 📊.
    pub fn print_status(&self, msg: &str) {
        println!("📊  {msg}");
    }

    /// Print an error message in red.
    pub fn print_error(&self, msg: &str) {
        eprintln!("\x1b[1;31m❌  Error: {msg}\x1b[0m");
    }

    /// Print the final rating banner for `ticker`.
    ///
    /// ```text
    /// ┌──────────────────────────────────┐
    /// │  600519  →  🟢 Strong Buy        │
    /// └──────────────────────────────────┘
    /// ```
    pub fn print_rating(&self, ticker: &str, rating: &str) {
        let badge = crate::report::format_rating_badge(rating);
        let line = format!("  {ticker}  →  {badge}  ");
        let width = line.chars().count() + 2;
        let top = format!("┌{}┐", "─".repeat(width));
        let bot = format!("└{}┘", "─".repeat(width));
        println!("\n\x1b[1;33m{top}\x1b[0m");
        println!("\x1b[1;33m│{line} │\x1b[0m");
        println!("\x1b[1;33m{bot}\x1b[0m\n");
    }

    /// Return a spinner [`ProgressBar`] with `msg` as the label.
    ///
    /// The caller is responsible for calling `.finish_with_message` or
    /// `.finish_and_clear` when the work is done.
    pub fn spinner(&self, msg: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(msg.to_owned());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }

    /// Return a determinate [`ProgressBar`] with `len` steps and `msg` as
    /// the prefix label.
    pub fn progress_bar(&self, len: u64, msg: &str) -> ProgressBar {
        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::with_template(
                "{msg} [{bar:40.cyan/blue}] {pos}/{len} ({elapsed})",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        pb.set_message(msg.to_owned());
        pb
    }
}

impl Default for AppDisplay {
    fn default() -> Self {
        Self::new()
    }
}
