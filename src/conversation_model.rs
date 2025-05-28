pub mod anthropic;
pub mod openai;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ModelConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct InternalConfig {
    pub model_config: ModelConfig,
    pub force_tool: Option<String>,
}

impl InternalConfig {
    pub fn new(model_config: ModelConfig) -> Self {
        Self {
            model_config,
            force_tool: None,
        }
    }

    pub fn with_forced_tool(mut self, tool_name: String) -> Self {
        self.force_tool = Some(tool_name);
        self
    }
}

#[derive(Debug)]
pub enum GenerationResult {
    Text(String),
    ToolUse {
        name: String,
        arguments: serde_json::Value,
    },
}

#[async_trait::async_trait]
pub trait ConversationModel: Send + Sync {
    async fn generate(&self, prompt: &str, config: &InternalConfig) -> Result<GenerationResult>;
}

pub fn create_model(provider: &str) -> Result<Arc<dyn ConversationModel>> {
    match provider {
        "anthropic" => Ok(Arc::new(anthropic::AnthropicModel::new()?)),
        "openai" => Ok(Arc::new(openai::OpenAIModel::new()?)),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
    }
}