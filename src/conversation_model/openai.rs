use anyhow::{Result, anyhow};

use super::{ConversationModel, GenerationResult, InternalConfig};

pub struct OpenAIModel {
    api_key: String,
}

impl OpenAIModel {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY environment variable not set"))?;
        Ok(Self { api_key })
    }
}

#[async_trait::async_trait]
impl ConversationModel for OpenAIModel {
    async fn generate(
        &self,
        prompt: &str,
        config: &InternalConfig,
    ) -> Result<Vec<GenerationResult>> {
        let client = reqwest::Client::new();

        let mut messages = Vec::new();

        if let Some(system) = &config.model_config.system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let mut request_body = serde_json::json!({
            "model": config.model_config.model,
            "max_tokens": config.model_config.max_tokens,
            "messages": messages
        });

        if let Some(tools) = &config.model_config.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.schema
                        }
                    })
                })
                .collect();

            request_body["tools"] = serde_json::Value::Array(tool_defs);

            if let Some(forced_tool) = &config.force_tool {
                request_body["tool_choice"] = serde_json::json!({
                    "type": "function",
                    "function": {"name": forced_tool}
                });
            }
        }

        if let Some(temperature) = config.model_config.temperature {
            request_body["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temperature)
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            );
        }

        if let Some(top_p) = config.model_config.top_p {
            request_body["top_p"] = serde_json::Value::Number(
                serde_json::Number::from_f64(top_p).unwrap_or_else(|| serde_json::Number::from(0)),
            );
        }

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        if let Some(message) = json["choices"][0]["message"].as_object() {
            // Handle text content if present
            if let Some(content) = message["content"].as_str() {
                if !content.is_empty() {
                    results.push(GenerationResult::Text(content.to_string()));
                }
            }

            // Handle tool calls if present
            if let Some(tool_calls) = message["tool_calls"].as_array() {
                for tool_call in tool_calls {
                    let name = tool_call["function"]["name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();
                    let arguments: serde_json::Value = serde_json::from_str(
                        tool_call["function"]["arguments"].as_str().unwrap_or("{}"),
                    )
                    .unwrap_or_default();
                    results.push(GenerationResult::ToolUse { name, arguments });
                }
            }
        }

        if results.is_empty() {
            results.push(GenerationResult::Text("Failed to get response".to_string()));
        }

        Ok(results)
    }
}
