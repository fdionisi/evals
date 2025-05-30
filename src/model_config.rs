use serde::{Deserialize, Serialize};

use crate::conversation_model::ToolDefinition;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub top_k: Option<u32>,
    pub top_p: Option<f64>,
    pub system: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub iterations: Option<usize>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1000,
            temperature: None,
            top_k: None,
            top_p: None,
            system: None,
            tools: None,
            iterations: None,
        }
    }
}
