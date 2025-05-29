use std::sync::Arc;

use anyhow::Result;

use crate::{
    conversation_model::{ConversationConifg, ConversationModel},
    mcp_manager::McpManager,
    model_config::ModelConfig,
};

pub struct TestedModel {
    model: Arc<dyn ConversationModel>,
    mcp_manager: Option<Arc<McpManager>>,
}

impl TestedModel {
    pub fn new(model: Arc<dyn ConversationModel>) -> Self {
        Self {
            model,
            mcp_manager: None,
        }
    }

    pub fn with_mcp(model: Arc<dyn ConversationModel>, mcp_manager: Arc<McpManager>) -> Self {
        Self {
            model,
            mcp_manager: Some(mcp_manager),
        }
    }

    pub async fn respond(&self, input: &str, config: &ModelConfig) -> Result<String> {
        let mut enhanced_config = config.clone();

        if let Some(mcp_manager) = &self.mcp_manager {
            let mcp_tools = mcp_manager.get_available_tools().await?;
            let mut all_tools = enhanced_config.tools.unwrap_or_default();
            all_tools.extend(mcp_tools);
            enhanced_config.tools = Some(all_tools);
        }

        let internal_config = ConversationConifg::new(enhanced_config);
        let results = self.model.generate(input, &internal_config).await?;

        let mut response = String::new();
        for result in results {
            response.push_str(&format!("{result}\n"));
        }

        Ok(response.trim().to_string())
    }
}
