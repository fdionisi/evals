use anyhow::{Result, anyhow};
use tokio::time::{Duration, sleep};

use super::{ConversationModel, GenerationResult, InternalConfig};

pub struct AnthropicModel {
    api_key: String,
}

impl AnthropicModel {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;
        Ok(Self { api_key })
    }
}

#[async_trait::async_trait]
impl ConversationModel for AnthropicModel {
    async fn generate(
        &self,
        prompt: &str,
        config: &InternalConfig,
    ) -> Result<Vec<GenerationResult>> {
        let client = reqwest::Client::new();

        let mut request_body = serde_json::json!({
            "model": config.model_config.model,
            "max_tokens": config.model_config.max_tokens,
            "messages": [
                {"role": "user", "content": prompt}
            ]
        });

        if let Some(system) = &config.model_config.system {
            request_body["system"] = serde_json::Value::String(system.clone());
        }

        if let Some(tools) = &config.model_config.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.schema
                    })
                })
                .collect();

            request_body["tools"] = serde_json::Value::Array(tool_defs);

            if let Some(forced_tool) = &config.force_tool {
                request_body["tool_choice"] = serde_json::json!({
                    "type": "tool",
                    "name": forced_tool
                });
            }
        }

        if let Some(temperature) = config.model_config.temperature {
            request_body["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temperature)
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            );
        }

        if let Some(top_k) = config.model_config.top_k {
            request_body["top_k"] = serde_json::Value::Number(serde_json::Number::from(top_k));
        }

        if let Some(top_p) = config.model_config.top_p {
            request_body["top_p"] = serde_json::Value::Number(
                serde_json::Number::from_f64(top_p).unwrap_or_else(|| serde_json::Number::from(0)),
            );
        }

        loop {
            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await?;

            if response.status() == 429 {
                if let Some(retry_after) = response.headers().get("retry-after") {
                    if let Ok(retry_seconds) = retry_after.to_str().unwrap_or("60").parse::<u64>() {
                        sleep(Duration::from_secs(retry_seconds)).await;
                        continue;
                    }
                }

                sleep(Duration::from_secs(60)).await;
                continue;
            }

            let json: serde_json::Value = response.json().await?;

            let mut results = Vec::new();

            if let Some(content) = json["content"].as_array() {
                for item in content {
                    if item["type"] == "tool_use" {
                        let name = item["name"].as_str().unwrap_or("unknown").to_string();
                        let arguments = item["input"].clone();
                        results.push(GenerationResult::ToolUse { name, arguments });
                    } else if item["type"] == "text" {
                        let text = item["text"].as_str().unwrap_or("Failed to get response");
                        results.push(GenerationResult::Text(text.to_string()));
                    }
                }
            }

            if results.is_empty() {
                return Err(anyhow!("No valid content found in response"));
            } else {
                return Ok(results);
            }
        }
    }
}
