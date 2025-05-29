pub mod anthropic;
pub mod openai;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

use crate::ModelConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ConversationConifg {
    pub model_config: ModelConfig,
    pub force_tool: Option<String>,
}

impl ConversationConifg {
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

impl fmt::Display for GenerationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerationResult::Text(text) => write!(f, "{}", text),
            GenerationResult::ToolUse { name, arguments } => {
                write!(
                    f,
                    "{{ \"name\": \"{}\", \"arguments\": {} }}",
                    name, arguments
                )
            }
        }
    }
}

#[async_trait::async_trait]
pub trait ConversationModel: Send + Sync {
    async fn generate(
        &self,
        prompt: &str,
        config: &ConversationConifg,
    ) -> Result<Vec<GenerationResult>>;
}

pub fn create_model(provider: &str) -> Result<Arc<dyn ConversationModel>> {
    match provider {
        "anthropic" => Ok(Arc::new(anthropic::AnthropicModel::new()?)),
        "openai" => Ok(Arc::new(openai::OpenAIModel::new()?)),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
    }
}
