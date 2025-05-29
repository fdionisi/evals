mod conversation_model;
mod evaluation;
mod judge;
mod mcp_manager;
mod model_config;
mod tested_model;
mod ui;

use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use chrono::Utc;
use clap::{Parser, Subcommand};

use futures::stream::FuturesUnordered;
use tokio_stream::{Stream, StreamExt};

use crate::{
    conversation_model::create_model,
    evaluation::{
        CategoryStats, EvalCase, EvalCaseReport, EvalResult, EvaluationReport, ReportMetadata,
        ReportSummary,
    },
    judge::{JudgeModel, JudgePrompt},
    mcp_manager::{McpManager, McpServersConfig},
    model_config::ModelConfig,
    tested_model::TestedModel,
};

/// Command-line interface for the AI evaluation tool
#[derive(Parser)]
#[command(name = "evals")]
#[command(about = "A deadly simple evaluation framework for AI models")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands for the evaluation tool
#[derive(Subcommand)]
pub enum Commands {
    /// Run evaluations on a set of test cases
    Run {
        /// Path to JSON file containing evaluation cases
        #[arg(long)]
        cases_file: String,
        /// Minimum score threshold for passing evaluations (default: 0.8)
        #[arg(long)]
        threshold: Option<f64>,
        /// Judge model to use for evaluation (default: claude-3-5-sonnet-20241022)
        #[arg(long)]
        judge_model: Option<String>,
        /// AI provider to use (e.g., "anthropic", "openai")
        #[arg(long)]
        provider: String,
        /// Model name to evaluate
        #[arg(long)]
        model: String,
        /// Maximum tokens to generate (default: 1000)
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Temperature for text generation (0.0-1.0)
        #[arg(long)]
        temperature: Option<f64>,
        /// Top-k sampling parameter
        #[arg(long)]
        top_k: Option<u32>,
        /// Top-p (nucleus) sampling parameter (0.0-1.0)
        #[arg(long)]
        top_p: Option<f64>,
        /// System prompt (use @filename to load from file)
        #[arg(long)]
        system: Option<String>,
        /// Output file path for evaluation report (JSON format)
        #[arg(long)]
        output: Option<String>,
        /// Path to MCP servers configuration file
        #[arg(long)]
        mcp_servers: Option<String>,
    },
}

fn run_eval_stream(
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

                let case_report = EvalCaseReport {
                    input: case.input.clone(),
                    expected_output: case.expected_output.as_ref().and_then(|e| e.to_object()),
                    metadata: case.metadata.clone(),
                };

                Ok(EvalResult {
                    case: case_report,
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
                    &judge_for_report.prompt(),
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
