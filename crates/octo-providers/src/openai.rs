use async_trait::async_trait;
use octo_core::error::ProviderError;
use octo_core::message::*;
use octo_core::model::Model;
use octo_core::provider::*;
use octo_core::tool::ToolDefinition;
use reqwest::Client;
use std::sync::Arc;

const MAX_RETRIES: u32 = 6;
const INITIAL_BACKOFF_MS: u64 = 2000;
const MAX_BACKOFF_MS: u64 = 60_000;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: Model,
    base_url: String,
    max_tokens: u64,
    last_request: Arc<tokio::sync::Mutex<std::time::Instant>>,
}

/// Minimum interval between API requests (ms) to avoid rate limiting with a single key
const MIN_REQUEST_INTERVAL_MS: u64 = 500;

impl OpenAiProvider {
    pub fn new(api_key: String, model: Model, base_url: String, max_tokens: u64) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
            max_tokens,
            last_request: Arc::new(tokio::sync::Mutex::new(
                std::time::Instant::now() - std::time::Duration::from_secs(10),
            )),
        }
    }

    /// Throttle requests to respect rate limits with a single API key
    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed().as_millis() as u64;
        if elapsed < MIN_REQUEST_INTERVAL_MS {
            let wait = MIN_REQUEST_INTERVAL_MS - elapsed;
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }
        *last = std::time::Instant::now();
    }

    fn convert_messages(
        &self,
        messages: &[Message],
        system_prompt: &str,
    ) -> Vec<serde_json::Value> {
        let mut result = vec![serde_json::json!({
            "role": "system",
            "content": system_prompt,
        })];

        for msg in messages {
            match msg.role {
                MessageRole::System => continue,
                MessageRole::User => {
                    let text = msg.text_content();
                    if !text.is_empty() {
                        result.push(serde_json::json!({
                            "role": "user",
                            "content": text,
                        }));
                    }
                }
                MessageRole::Assistant => {
                    let mut content_parts = Vec::new();
                    let mut tool_calls = Vec::new();

                    for part in &msg.parts {
                        match part {
                            ContentPart::Text { text } => {
                                content_parts.push(text.clone());
                            }
                            ContentPart::ToolCall { id, name, input } => {
                                let input_val: serde_json::Value =
                                    serde_json::from_str(input).unwrap_or(serde_json::json!({}));
                                tool_calls.push(serde_json::json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": input_val.to_string(),
                                    }
                                }));
                            }
                            _ => {}
                        }
                    }

                    let mut assistant_msg = serde_json::json!({"role": "assistant"});
                    if !content_parts.is_empty() {
                        assistant_msg["content"] =
                            serde_json::Value::String(content_parts.join(""));
                    }
                    if !tool_calls.is_empty() {
                        assistant_msg["tool_calls"] = serde_json::json!(tool_calls);
                    }
                    result.push(assistant_msg);
                }
                MessageRole::Tool => {
                    for part in &msg.parts {
                        if let ContentPart::ToolResult {
                            tool_call_id,
                            content,
                            ..
                        } = part
                        {
                            result.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "content": content,
                            }));
                        }
                    }
                }
            }
        }

        result
    }

    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                let properties: serde_json::Map<String, serde_json::Value> = t
                    .parameters
                    .iter()
                    .map(|(k, v)| {
                        let mut schema = serde_json::Map::new();
                        schema.insert(
                            "type".into(),
                            serde_json::Value::String(v.param_type.clone()),
                        );
                        schema.insert(
                            "description".into(),
                            serde_json::Value::String(v.description.clone()),
                        );
                        if let Some(enums) = &v.enum_values {
                            schema.insert("enum".into(), serde_json::json!(enums));
                        }
                        (k.clone(), serde_json::Value::Object(schema))
                    })
                    .collect();

                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": {
                            "type": "object",
                            "properties": properties,
                            "required": t.required,
                        }
                    }
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn send_messages(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError> {
        self.throttle().await;

        let mut body = serde_json::json!({
            "model": self.model.id.0,
            "max_tokens": self.max_tokens,
            "messages": self.convert_messages(messages, system_prompt),
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(self.convert_tools(tools));
        }

        let mut last_err = ProviderError::Http("no attempts made".into());

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let backoff = compute_backoff(attempt, None);
                eprintln!(
                    "\x1b[33m[retry {}/{}] Rate limited, waiting {:.1}s...\x1b[0m",
                    attempt, MAX_RETRIES - 1, backoff as f64 / 1000.0
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
            }

            let resp = match self
                .client
                .post(format!("{}/v1/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_err = ProviderError::Http(e.to_string());
                    continue;
                }
            };

            let status = resp.status().as_u16();
            if resp.status().is_success() {
                let api_resp: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| ProviderError::Http(e.to_string()))?;
                return parse_openai_response(api_resp);
            }

            // Parse Retry-After header if present
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|secs| secs * 1000);

            let text = resp.text().await.unwrap_or_default();
            if status == 429 || status == 502 || status == 503 {
                last_err = ProviderError::RateLimited {
                    retry_after_ms: retry_after.unwrap_or(compute_backoff(attempt + 1, None)),
                };
                continue;
            }
            return Err(ProviderError::Api {
                status,
                message: text,
            });
        }

        Err(last_err)
    }

    async fn stream_response(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDefinition],
    ) -> Result<ProviderEventStream, ProviderError> {
        self.throttle().await;

        let mut body = serde_json::json!({
            "model": self.model.id.0,
            "max_tokens": self.max_tokens,
            "messages": self.convert_messages(messages, system_prompt),
            "stream": true,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(self.convert_tools(tools));
        }

        let mut last_err = ProviderError::Http("no attempts made".into());
        let mut resp_ok = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let backoff = compute_backoff(attempt, None);
                eprintln!(
                    "\x1b[33m[retry {}/{}] Rate limited, waiting {:.1}s...\x1b[0m",
                    attempt, MAX_RETRIES - 1, backoff as f64 / 1000.0
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
            }

            let resp = match self
                .client
                .post(format!("{}/v1/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_err = ProviderError::Http(e.to_string());
                    continue;
                }
            };

            let status = resp.status().as_u16();
            if resp.status().is_success() {
                resp_ok = Some(resp);
                break;
            }

            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|secs| secs * 1000);

            let text = resp.text().await.unwrap_or_default();
            if status == 429 || status == 502 || status == 503 {
                last_err = ProviderError::RateLimited {
                    retry_after_ms: retry_after.unwrap_or(compute_backoff(attempt + 1, None)),
                };
                continue;
            }
            return Err(ProviderError::Api {
                status,
                message: text,
            });
        }

        let resp = resp_ok.ok_or(last_err)?;

        let byte_stream = resp.bytes_stream();

        let stream = async_stream::stream! {
            use tokio_stream::StreamExt;

            let mut byte_stream = Box::pin(byte_stream);
            let mut buffer = String::new();
            let mut current_tool_calls: std::collections::HashMap<i64, (String, String)> =
                std::collections::HashMap::new();
            let mut has_content = false;

            while let Some(chunk) = byte_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield ProviderEvent::Error {
                            error: ProviderError::Stream(e.to_string()),
                        };
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    let data = match line.strip_prefix("data: ") {
                        Some(d) => d.trim(),
                        None => continue,
                    };

                    if data == "[DONE]" {
                        continue;
                    }

                    let json: serde_json::Value = match serde_json::from_str(data) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };

                    if let Some(choices) = json["choices"].as_array() {
                        for choice in choices {
                            let delta = &choice["delta"];
                            let finish_reason = choice["finish_reason"].as_str();

                            // Content delta
                            if let Some(text) = delta["content"].as_str() {
                                if !has_content {
                                    yield ProviderEvent::ContentStart;
                                    has_content = true;
                                }
                                yield ProviderEvent::ContentDelta {
                                    text: text.to_string(),
                                };
                            }

                            // Tool call deltas
                            if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                for tc in tool_calls {
                                    let index = tc["index"].as_i64().unwrap_or(0);

                                    if let Some(func) = tc.get("function") {
                                        // Only register new tool call when name is non-empty
                                        // Atlas Cloud sends name="" in subsequent chunks
                                        if let Some(name) = func["name"].as_str() {
                                            if !name.is_empty()
                                                && !current_tool_calls.contains_key(&index)
                                            {
                                                let id = tc["id"]
                                                    .as_str()
                                                    .unwrap_or("")
                                                    .to_string();
                                                current_tool_calls.insert(
                                                    index,
                                                    (id.clone(), name.to_string()),
                                                );

                                                if has_content {
                                                    yield ProviderEvent::ContentStop;
                                                    has_content = false;
                                                }

                                                yield ProviderEvent::ToolUseStart {
                                                    id,
                                                    name: name.to_string(),
                                                };
                                            }
                                        }
                                        if let Some(args) = func["arguments"].as_str() {
                                            if !args.is_empty() {
                                                yield ProviderEvent::ToolUseDelta {
                                                    input_json_chunk: args.to_string(),
                                                };
                                            }
                                        }
                                    }
                                }
                            }

                            // Finish
                            if let Some(reason) = finish_reason {
                                if has_content {
                                    yield ProviderEvent::ContentStop;
                                    has_content = false;
                                }

                                for _ in current_tool_calls.drain() {
                                    yield ProviderEvent::ToolUseStop;
                                }

                                let finish = match reason {
                                    "stop" => FinishReason::EndTurn,
                                    "length" => FinishReason::MaxTokens,
                                    "tool_calls" => FinishReason::ToolUse,
                                    _ => FinishReason::EndTurn,
                                };

                                let usage = if let Some(u) = json.get("usage") {
                                    TokenUsage {
                                        input_tokens: u["prompt_tokens"]
                                            .as_u64()
                                            .unwrap_or(0),
                                        output_tokens: u["completion_tokens"]
                                            .as_u64()
                                            .unwrap_or(0),
                                        cache_creation_tokens: 0,
                                        cache_read_tokens: 0,
                                    }
                                } else {
                                    TokenUsage::default()
                                };

                                yield ProviderEvent::Complete {
                                    finish_reason: finish,
                                    usage,
                                };
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn model(&self) -> &Model {
        &self.model
    }
}

/// Exponential backoff with jitter to avoid thundering herd
fn compute_backoff(attempt: u32, server_retry_ms: Option<u64>) -> u64 {
    if let Some(ms) = server_retry_ms {
        return ms;
    }
    let base = INITIAL_BACKOFF_MS * 2u64.pow(attempt.saturating_sub(1));
    let capped = base.min(MAX_BACKOFF_MS);
    // Add 0-25% jitter
    let jitter = (capped as f64 * 0.25 * rand_f64()) as u64;
    capped + jitter
}

/// Simple pseudo-random float [0, 1) using thread-local state
fn rand_f64() -> f64 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (seed as f64 % 1000.0) / 1000.0
}

fn parse_openai_response(json: serde_json::Value) -> Result<ProviderResponse, ProviderError> {
    let choice = json["choices"]
        .as_array()
        .and_then(|c| c.first())
        .ok_or_else(|| ProviderError::Stream("No choices in response".into()))?;

    let message = &choice["message"];
    let mut content = Vec::new();

    if let Some(text) = message["content"].as_str() {
        content.push(ContentPart::Text {
            text: text.to_string(),
        });
    }

    if let Some(tool_calls) = message["tool_calls"].as_array() {
        for tc in tool_calls {
            content.push(ContentPart::ToolCall {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                input: tc["function"]["arguments"]
                    .as_str()
                    .unwrap_or("{}")
                    .to_string(),
            });
        }
    }

    let finish_reason = match choice["finish_reason"].as_str() {
        Some("stop") => FinishReason::EndTurn,
        Some("length") => FinishReason::MaxTokens,
        Some("tool_calls") => FinishReason::ToolUse,
        _ => FinishReason::EndTurn,
    };

    let usage = TokenUsage {
        input_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
        output_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        cache_creation_tokens: 0,
        cache_read_tokens: 0,
    };

    Ok(ProviderResponse {
        content,
        finish_reason,
        usage,
    })
}
