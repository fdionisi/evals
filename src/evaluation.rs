use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{judge::JudgePrompt, model_config::ModelConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalCase {
    pub input: String,
    pub expected_output: Option<ExpectedOutput>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ExpectedOutput {
    String(String),
    Object(ExpectedOutputObject),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ExpectedOutputObject {
    #[serde(rename = "comparison")]
    ContentComparison { description: String },
    #[serde(rename = "behavior")]
    BehaviorDescription { description: String },
}

impl ExpectedOutput {
    pub fn to_object(&self) -> Option<ExpectedOutputObject> {
        match self {
            ExpectedOutput::String(content) => Some(ExpectedOutputObject::ContentComparison {
                description: content.clone(),
            }),
            ExpectedOutput::Object(obj) => Some(obj.clone()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalResult {
    pub case: EvalCaseReport,
    pub actual_output: String,
    pub judge_score: f64,
    pub judge_reasoning: String,
    pub passed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalCaseReport {
    pub input: String,
    pub expected_output: Option<ExpectedOutputObject>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationReport {
    pub metadata: ReportMetadata,
    pub configuration: ModelConfig,
    pub judge_configuration: JudgePrompt,
    pub summary: ReportSummary,
    pub results: Vec<EvalResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub generated_at: DateTime<Utc>,
    pub total_cases: usize,
    pub threshold: f64,
    pub execution_time_seconds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportSummary {
    pub passed_count: usize,
    pub failed_count: usize,
    pub pass_rate_percent: f64,
    pub average_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub category_breakdown: HashMap<String, CategoryStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryStats {
    pub total: usize,
    pub passed: usize,
    pub pass_rate_percent: f64,
}
