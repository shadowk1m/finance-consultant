pub mod prompts;

use anyhow::bail;
use futures::StreamExt;
use serde_json::Value;

// ─── Message ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }
}

// ─── LlmClient ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LlmClient {
    pub base_url: String,
    pub api_key: String,
    pub client: reqwest::Client,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key: api_key.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    /// Non-streaming chat completion. Returns the assistant's reply text.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        temperature: f32,
    ) -> anyhow::Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "stream": false,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("LLM request failed ({status}): {text}");
        }

        let json: Value = resp.json().await?;
        let content = json
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("unexpected response shape: {json}"))?
            .to_owned();

        Ok(content)
    }

    /// Streaming chat completion via SSE. Sends text chunks through `tx`;
    /// sends a final empty string to signal end-of-stream.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        temperature: f32,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "stream": true,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("LLM stream request failed ({status}): {text}");
        }

        // Buffer for assembling partial SSE lines split across chunks.
        let mut remainder = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(result) = stream.next().await {
            let chunk = result?;
            let text = String::from_utf8_lossy(&chunk);
            remainder.push_str(&text);

            // Process all complete lines present in the buffer.
            loop {
                match remainder.find('\n') {
                    None => break,
                    Some(pos) => {
                        let line = remainder[..pos].trim_end_matches('\r').to_owned();
                        remainder = remainder[pos + 1..].to_owned();

                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim() == "[DONE]" {
                                continue;
                            }
                            if let Ok(json) = serde_json::from_str::<Value>(data) {
                                if let Some(delta_content) = json
                                    .pointer("/choices/0/delta/content")
                                    .and_then(Value::as_str)
                                {
                                    if !delta_content.is_empty() {
                                        // If the receiver is gone, stop gracefully.
                                        if tx.send(delta_content.to_owned()).await.is_err() {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Signal end-of-stream to the receiver.
        let _ = tx.send(String::new()).await;
        Ok(())
    }
}
