use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    conversation_model::{ConversationModel, GenerationResult, ConversationConifg, ToolDefinition},
    evaluation::{EvalCase, ExpectedOutput, ExpectedOutputObject},
    model_config::ModelConfig,
};

pub struct JudgeModel {
    model: Arc<dyn ConversationModel>,
    prompt: JudgePrompt,
}

impl JudgeModel {
    pub fn new(model: Arc<dyn ConversationModel>, prompt: JudgePrompt) -> Self {
        Self { model, prompt }
    }

    pub async fn evaluate(&self, case: &EvalCase, actual_output: &str) -> Result<(f64, String)> {
        let (expected_text, evaluation_type) = match &case.expected_output {
            Some(ExpectedOutput::String(content)) => (content.as_str(), "content"),
            Some(ExpectedOutput::Object(ExpectedOutputObject::ContentComparison {
                description,
            })) => (description.as_str(), "content"),
            Some(ExpectedOutput::Object(ExpectedOutputObject::BehaviorDescription {
                description,
            })) => (description.as_str(), "behavior"),
            None => ("N/A", "none"),
        };

        let prompt_text = self
            .prompt
            .user_template
            .replace("{input}", &case.input)
            .replace("{expected}", expected_text)
            .replace("{actual}", actual_output)
            .replace("{evaluation_type}", evaluation_type);

        let eval_tool = ToolDefinition {
            name: "evaluate_response".to_string(),
            description: "Evaluate the quality of a response and provide a score".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "score": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Quality score from 0.0 to 1.0"
                    },
                    "reasoning": {
                        "type": "string",
                        "description": "Detailed reasoning for the score"
                    }
                },
                "required": ["score", "reasoning"]
            }),
        };

        let judge_config = ModelConfig {
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1000,
            temperature: Some(0.0),
            top_k: None,
            top_p: None,
            system: Some(self.prompt.system.clone()),
            tools: Some(vec![eval_tool]),
        };

        let internal_config =
            ConversationConifg::new(judge_config).with_forced_tool("evaluate_response".to_string());

        let results = self.model.generate(&prompt_text, &internal_config).await?;

        for result in results {
            match result {
                GenerationResult::ToolUse { name: _, arguments } => {
                    let score = arguments["score"].as_f64().unwrap_or(0.0);
                    let reasoning = arguments["reasoning"]
                        .as_str()
                        .unwrap_or("No reasoning provided")
                        .to_string();
                    return Ok((score, reasoning));
                }
                _ => continue,
            }
        }

        Err(anyhow!("Expected tool use response from judge model"))
    }

    pub fn prompt(&self) -> &JudgePrompt {
        &self.prompt
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgePrompt {
    pub system: String,
    pub user_template: String,
}

impl Default for JudgePrompt {
    fn default() -> Self {
        Self {
            system: "You are an AI judge evaluating response quality. You must use the evaluate_response tool to provide your assessment. Consider the evaluation type when scoring.".to_string(),
            user_template: "Evaluate this response:\n\nInput: {input}\nExpected: {expected}\nActual: {actual}\nEvaluation Type: {evaluation_type}\n\nEvaluation Instructions:\n- If evaluation_type is 'content': Compare the actual output against the expected content. The actual output should convey the same meaning/information as expected, but doesn't need to be word-for-word identical.\n- If evaluation_type is 'behavior': Assess whether the actual output demonstrates the described behavior. The expected text describes how the model should behave, not what it should output.\n- If evaluation_type is 'none': Evaluate the general quality and appropriateness of the response.\n\nUse the evaluate_response tool to provide your score (0.0-1.0) and reasoning.".to_string(),
        }
    }
}
