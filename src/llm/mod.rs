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

    /// Split a message slice into (system_prompt, non_system_messages).
    /// The Anthropic Messages API takes `system` as a top-level string and
    /// `messages` as a list of user/assistant turns only.
    fn split_messages<'a>(messages: &'a [Message]) -> (String, Vec<&'a Message>) {
        let system: Vec<&str> = messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect();
        let turns: Vec<&Message> = messages
            .iter()
            .filter(|m| m.role != "system")
            .collect();
        (system.join("\n\n"), turns)
    }

    /// Non-streaming chat completion using the Anthropic Messages API.
    /// Returns the assistant's reply text.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        _temperature: f32,
    ) -> anyhow::Result<String> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, turns) = Self::split_messages(messages);

        // Build the messages array — role must alternate user/assistant.
        // Ensure it starts with a user message.
        let msgs: Vec<Value> = turns
            .iter()
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 8096,
            "messages": msgs,
        });
        if !system.is_empty() {
            body["system"] = Value::String(system);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("LLM request failed ({status}): {text}");
        }

        let json: Value = resp.json().await?;
        // Anthropic response: {"content": [{"type": "text", "text": "..."}], ...}
        let content = json
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("unexpected Anthropic response shape: {json}"))?
            .to_owned();

        Ok(content)
    }

    /// Streaming chat completion via SSE using the Anthropic Messages API.
    /// Sends text chunks through `tx`; sends a final empty string to signal end.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        _temperature: f32,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/messages", self.base_url);
        let (system, turns) = Self::split_messages(messages);

        let msgs: Vec<Value> = turns
            .iter()
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 8096,
            "stream": true,
            "messages": msgs,
        });
        if !system.is_empty() {
            body["system"] = Value::String(system);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            bail!("LLM stream request failed ({status}): {text}");
        }

        let mut remainder = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(result) = stream.next().await {
            let chunk = result?;
            remainder.push_str(&String::from_utf8_lossy(&chunk));

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
                                // Anthropic SSE: content_block_delta events carry
                                // {"type":"content_block_delta","delta":{"type":"text_delta","text":"..."}}
                                if json.get("type").and_then(Value::as_str) == Some("content_block_delta") {
                                    if let Some(text) = json
                                        .pointer("/delta/text")
                                        .and_then(Value::as_str)
                                    {
                                        if !text.is_empty() {
                                            if tx.send(text.to_owned()).await.is_err() {
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
        }

        let _ = tx.send(String::new()).await;
        Ok(())
    }
}
