#![allow(dead_code)]

use std::io::{self, Write};

/// Print `"label: "` and return the trimmed line the user enters.
///
/// An empty response is returned as-is (the empty string).
pub fn prompt(label: &str) -> anyhow::Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;

    let mut buf = String::new();
    let n = io::stdin().read_line(&mut buf)?;
    // 0 bytes means EOF (piped input finished, Ctrl-D, etc.) — signal the
    // caller to stop rather than returning an empty string that loops forever.
    if n == 0 {
        anyhow::bail!("EOF");
    }
    Ok(buf.trim().to_owned())
}

/// Like [`prompt`] but shows the default value in brackets.
///
/// ```text
/// API base URL [https://api.deepseek.com]: _
/// ```
///
/// If the user presses Enter without typing anything the default is returned.
pub fn prompt_with_default(label: &str, default: &str) -> anyhow::Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;

    let mut buf = String::new();
    let n = io::stdin().read_line(&mut buf)?;
    if n == 0 {
        // EOF → use the default silently
        return Ok(default.to_owned());
    }
    let trimmed = buf.trim();

    if trimmed.is_empty() {
        Ok(default.to_owned())
    } else {
        Ok(trimmed.to_owned())
    }
}

/// Ask a yes/no question.  Accepts `y`, `yes`, `n`, `no` (case-insensitive).
/// Pressing Enter alone is treated as **yes**.
///
/// ```text
/// Start interactive chat? [Y/n]: _
/// ```
pub fn confirm(label: &str) -> anyhow::Result<bool> {
    print!("{label} [Y/n]: ");
    io::stdout().flush()?;

    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;

    let answer = buf.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "" | "y" | "yes"))
}

/// Present a numbered list of options and return the **0-based** index of the
/// user's selection.
///
/// ```text
/// Asset type:
///   1) stock
///   2) real_estate
/// Choose [1]: _
/// ```
///
/// The user may enter a number 1–N; pressing Enter alone selects option 1.
/// Invalid input is re-prompted.
pub fn select(label: &str, options: &[&str]) -> anyhow::Result<usize> {
    if options.is_empty() {
        anyhow::bail!("select: option list is empty");
    }

    println!("{label}:");
    for (i, opt) in options.iter().enumerate() {
        println!("  {}) {opt}", i + 1);
    }

    loop {
        print!("Choose [1]: ");
        io::stdout().flush()?;

        let mut buf = String::new();
        let n = io::stdin().read_line(&mut buf)?;
        // EOF → default to first option
        if n == 0 {
            return Ok(0);
        }
        let trimmed = buf.trim();

        // Default to first option on empty input.
        if trimmed.is_empty() {
            return Ok(0);
        }

        match trimmed.parse::<usize>() {
            Ok(n) if n >= 1 && n <= options.len() => return Ok(n - 1),
            _ => {
                eprintln!(
                    "  Please enter a number between 1 and {}.",
                    options.len()
                );
            }
        }
    }
}
