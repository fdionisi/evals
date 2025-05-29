use std::collections::HashMap;

use anyhow::{Result, anyhow};
use rmcp::{ServiceExt, transport::TokioChildProcess};
use serde::{Deserialize, Serialize};

use crate::conversation_model::ToolDefinition;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServersConfig {
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: McpServerType,
    pub command: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum McpServerType {
    Local,
}

pub struct McpManager {
    available_tools: Vec<ToolDefinition>,
}

impl McpManager {
    pub async fn start_servers(configs: &[McpServerConfig]) -> Result<Self> {
        let mut all_tools = Vec::new();

        for config in configs {
            let mut cmd = tokio::process::Command::new(&config.command[0]);
            cmd.args(&config.args);

            for (key, value) in &config.env {
                cmd.env(key, value);
            }

            let transport = TokioChildProcess::new(&mut cmd)
                .map_err(|e| anyhow!("Failed to create transport for '{}': {}", config.name, e))?;

            let service = ()
                .serve(transport)
                .await
                .map_err(|e| anyhow!("Failed to create service for '{}': {}", config.name, e))?;

            let tools_response = service
                .list_tools(Default::default())
                .await
                .map_err(|e| anyhow!("Failed to list tools for '{}': {}", config.name, e))?;

            for tool in tools_response.tools {
                let tool_def = ToolDefinition {
                    name: tool.name.to_string(),
                    description: tool.description.to_string(),
                    schema: serde_json::Value::Object((*tool.input_schema).clone()),
                };
                all_tools.push(tool_def);
            }
        }

        Ok(Self {
            available_tools: all_tools,
        })
    }

    pub async fn get_available_tools(&self) -> Result<Vec<ToolDefinition>> {
        Ok(self.available_tools.clone())
    }
}
