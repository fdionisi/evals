mod conversation_model;
mod ui;

use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use rmcp::{service::ServiceExt, transport::TokioChildProcess};
use serde::{Deserialize, Serialize};
use tokio_stream::{Stream, StreamExt};
use futures::stream::FuturesUnordered;

use conversation_model::{
    ConversationModel, GenerationResult, InternalConfig, ToolDefinition, create_model,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalCase {
    pub input: String,
    pub expected_output: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalResult {
    pub case: EvalCase,
    pub actual_output: String,
    pub judge_score: f64,
    pub judge_reasoning: String,
    pub passed: bool,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgePrompt {
    pub system: String,
    pub user_template: String,
}

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
        }
    }
}

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

        let internal_config = InternalConfig::new(enhanced_config);
        let results = self.model.generate(input, &internal_config).await?;

        let mut response = String::new();
        for result in results {
            response.push_str(&format!("{result}\n"));
        }

        Ok(response.trim().to_string())
    }
}

pub struct JudgeModel {
    model: Arc<dyn ConversationModel>,
    prompt: JudgePrompt,
}

impl JudgeModel {
    pub fn new(model: Arc<dyn ConversationModel>, prompt: JudgePrompt) -> Self {
        Self { model, prompt }
    }

    pub async fn evaluate(&self, case: &EvalCase, actual_output: &str) -> Result<(f64, String)> {
        let prompt_text = self
            .prompt
            .user_template
            .replace("{input}", &case.input)
            .replace(
                "{expected}",
                &case.expected_output.as_deref().unwrap_or("N/A"),
            )
            .replace("{actual}", actual_output);

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
            InternalConfig::new(judge_config).with_forced_tool("evaluate_response".to_string());

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

impl Default for JudgePrompt {
    fn default() -> Self {
        Self {
            system: "You are an AI judge evaluating response quality. You must use the evaluate_response tool to provide your assessment.".to_string(),
            user_template: "Evaluate this response:\n\nInput: {input}\nExpected: {expected}\nActual: {actual}\n\nUse the evaluate_response tool to provide your score (0.0-1.0) and reasoning.".to_string(),
        }
    }
}

#[derive(Parser)]
#[command(name = "evals")]
#[command(about = "A simple AI evaluation library")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run {
        #[arg(long)]
        cases_file: String,
        #[arg(long)]
        threshold: Option<f64>,
        #[arg(long)]
        judge_model: Option<String>,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: String,
        #[arg(long)]
        max_tokens: Option<u32>,
        #[arg(long)]
        temperature: Option<f64>,
        #[arg(long)]
        top_k: Option<u32>,
        #[arg(long)]
        top_p: Option<f64>,
        #[arg(long)]
        system: Option<String>,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        mcp_servers: Option<String>,
    },
}

pub fn run_eval_stream(
    cases: Vec<EvalCase>,
    tested_model: Arc<TestedModel>,
    config: Arc<ModelConfig>,
    judge: Arc<JudgeModel>,
    threshold: f64,
) -> impl Stream<Item = Result<EvalResult>> {
    let futures: FuturesUnordered<_> = cases
        .into_iter()
        .map(|case| {
            let tested_model = Arc::clone(&tested_model);
            let config = Arc::clone(&config);
            let judge = Arc::clone(&judge);

            async move {
                let actual_output = tested_model.respond(&case.input, &config).await?;
                let (judge_score, judge_reasoning) = judge.evaluate(&case, &actual_output).await?;
                let passed = judge_score >= threshold;

                Ok(EvalResult {
                    case,
                    actual_output,
                    judge_score,
                    judge_reasoning,
                    passed,
                })
            }
        })
        .collect();

    futures
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            cases_file,
            threshold,
            judge_model,
            provider,
            model,
            max_tokens,
            temperature,
            top_k,
            top_p,
            system,
            output,
            mcp_servers,
        } => {
            let threshold = threshold.unwrap_or(0.8);
            let start_time = std::time::Instant::now();

            let cases_content = std::fs::read_to_string(&cases_file)?;
            let cases: Vec<EvalCase> = serde_json::from_str(&cases_content)?;

            let system_prompt = if let Some(system_str) = system {
                if system_str.starts_with('@') {
                    let file_path = &system_str[1..];
                    Some(tokio::fs::read_to_string(file_path).await.map_err(|e| {
                        anyhow!("Failed to read system prompt file '{}': {}", file_path, e)
                    })?)
                } else {
                    Some(system_str)
                }
            } else {
                None
            };

            let config = ModelConfig {
                provider: provider.clone(),
                model,
                max_tokens: max_tokens.unwrap_or(1000),
                temperature,
                top_k,
                top_p,
                system: system_prompt,
                tools: None,
            };

            let conversation_model = create_model(&provider)?;

            let mcp_manager = if let Some(mcp_config_path) = mcp_servers {
                let mcp_config_content = tokio::fs::read_to_string(&mcp_config_path).await?;
                let mcp_config: McpServersConfig = serde_json::from_str(&mcp_config_content)?;
                Some(Arc::new(
                    McpManager::start_servers(&mcp_config.servers).await?,
                ))
            } else {
                None
            };

            let tested_model = if let Some(mcp_manager) = mcp_manager {
                Arc::new(TestedModel::with_mcp(
                    Arc::clone(&conversation_model),
                    mcp_manager,
                ))
            } else {
                Arc::new(TestedModel::new(Arc::clone(&conversation_model)))
            };

            let _judge_model_name =
                judge_model.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());
            let judge_conversation_model = create_model("anthropic")?;
            let judge_prompt = JudgePrompt::default();
            let judge = Arc::new(JudgeModel::new(judge_conversation_model, judge_prompt));

            let config_arc = Arc::new(config.clone());

            let mut ui = ui::TerminalUI::new();
            let total_cases = cases.len();
            ui.print_header(&config, total_cases);

            ui.create_progress_bar(total_cases as u64);

            let judge_for_report = Arc::clone(&judge);
            let stream = run_eval_stream(cases, tested_model, config_arc, judge, threshold);
            tokio::pin!(stream);
            let mut results = Vec::new();
            let mut passed_count = 0;
            let mut failed_count = 0;

            while let Some(result) = stream.next().await {
                ui.set_current_case(results.len() + 1, passed_count, failed_count);

                match result {
                    Ok(eval_result) => {
                        if eval_result.passed {
                            passed_count += 1;
                        } else {
                            failed_count += 1;
                        }

                        ui.update_progress(
                            results.len() + 1,
                            total_cases,
                            passed_count,
                            failed_count,
                        );
                        results.push(eval_result);
                    }
                    Err(e) => {
                        ui.finish_progress();
                        eprintln!("  ✗ Error: {}", e);
                        return Err(e);
                    }
                }
            }

            ui.finish_progress();

            ui.print_summary(&results, threshold, start_time.elapsed().as_secs_f64());

            if let Some(output_file) = output {
                let spinner = ui.create_spinner("Generating report...");

                let report = generate_report(
                    &results,
                    &config,
                    &judge_for_report.prompt,
                    threshold,
                    start_time.elapsed().as_secs_f64(),
                )?;

                let report_json = serde_json::to_string_pretty(&report)?;
                tokio::fs::write(&output_file, report_json).await?;

                spinner.finish_with_message(format!("Report saved to {}", output_file));
            }
        }
    }

    Ok(())
}

fn generate_report(
    results: &[EvalResult],
    config: &ModelConfig,
    judge_prompt: &JudgePrompt,
    threshold: f64,
    execution_time: f64,
) -> Result<EvaluationReport> {
    let total_cases = results.len();
    let passed_count = results.iter().filter(|r| r.passed).count();
    let failed_count = total_cases - passed_count;
    let pass_rate = (passed_count as f64 / total_cases as f64) * 100.0;

    let scores: Vec<f64> = results.iter().map(|r| r.judge_score).collect();
    let average_score = scores.iter().sum::<f64>() / scores.len() as f64;
    let min_score = scores.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_score = scores.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    let mut category_breakdown = HashMap::new();
    for result in results {
        if let Some(category) = result.case.metadata.get("category") {
            let entry = category_breakdown
                .entry(category.clone())
                .or_insert(CategoryStats {
                    total: 0,
                    passed: 0,
                    pass_rate_percent: 0.0,
                });
            entry.total += 1;
            if result.passed {
                entry.passed += 1;
            }
            entry.pass_rate_percent = (entry.passed as f64 / entry.total as f64) * 100.0;
        }
    }

    let report = EvaluationReport {
        metadata: ReportMetadata {
            generated_at: Utc::now(),
            total_cases,
            threshold,
            execution_time_seconds: execution_time,
        },
        configuration: config.clone(),
        judge_configuration: judge_prompt.clone(),
        summary: ReportSummary {
            passed_count,
            failed_count,
            pass_rate_percent: pass_rate,
            average_score,
            min_score,
            max_score,
            category_breakdown,
        },
        results: results.to_vec(),
    };

    Ok(report)
}
