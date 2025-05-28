use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::time::Duration;

use crate::{EvalResult, ModelConfig};

pub struct TerminalUI {
    progress_bar: Option<ProgressBar>,
}

impl TerminalUI {
    pub fn new() -> Self {
        Self { progress_bar: None }
    }

    pub fn print_header(&self, config: &ModelConfig, total_cases: usize) {
        println!(
            "🧠 {} {} · {} cases",
            config.provider.dimmed(),
            config.model.bold(),
            total_cases.to_string().dimmed()
        );
    }

    pub fn create_progress_bar(&mut self, total: u64) {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.dim} {pos}/{len} cases {wide_bar:.dim} {percent}%\n")
                .unwrap()
                .progress_chars("━━╾─"),
        );
        pb.enable_steady_tick(Duration::from_millis(125));
        self.progress_bar = Some(pb);
    }

    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                .template("  {spinner:.dim} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(120));
        spinner
    }

    pub fn update_progress(&self, current: usize, _total: usize, passed: usize, failed: usize) {
        if let Some(pb) = &self.progress_bar {
            pb.set_position(current as u64);

            let pass_rate = if current > 0 {
                (passed as f64 / current as f64) * 100.0
            } else {
                0.0
            };
            let status_color = if pass_rate >= 80.0 {
                "green"
            } else if pass_rate >= 60.0 {
                "yellow"
            } else {
                "red"
            };

            let rate_display = match status_color {
                "green" => format!("{:.0}", pass_rate).green().to_string(),
                "yellow" => format!("{:.0}", pass_rate).yellow().to_string(),
                _ => format!("{:.0}", pass_rate).red().to_string(),
            };

            pb.set_message(format!(
                "{} pass {} fail ({}%)",
                passed.to_string().green(),
                failed.to_string().red(),
                rate_display
            ));
        }
    }

    pub fn set_current_case(&self, case_num: usize, passed: usize, failed: usize) {
        if let Some(pb) = &self.progress_bar {
            let completed = passed + failed;
            let pass_rate = if completed > 0 {
                (passed as f64 / completed as f64) * 100.0
            } else {
                0.0
            };
            let status_color = if pass_rate >= 80.0 {
                "green"
            } else if pass_rate >= 60.0 {
                "yellow"
            } else {
                "red"
            };

            let rate_display = match status_color {
                "green" => format!("{:.0}", pass_rate).green().to_string(),
                "yellow" => format!("{:.0}", pass_rate).yellow().to_string(),
                _ => format!("{:.0}", pass_rate).red().to_string(),
            };

            pb.set_message(format!(
                "case {} · {} pass {} fail ({}%)",
                case_num.to_string().dimmed(),
                passed.to_string().green(),
                failed.to_string().red(),
                rate_display
            ));
        }
    }

    pub fn finish_progress(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_and_clear();
        }
    }

    pub fn print_summary(&self, results: &[EvalResult], _threshold: f64, execution_time: f64) {
        let passed_count = results.iter().filter(|r| r.passed).count();
        let total_count = results.len();
        let pass_rate = (passed_count as f64 / total_count as f64) * 100.0;

        let scores: Vec<f64> = results.iter().map(|r| r.judge_score).collect();
        let avg_score = scores.iter().sum::<f64>() / scores.len() as f64;

        let (status_icon, status_text) = if pass_rate >= 80.0 {
            ("✓".green().to_string(), "passed".green().to_string())
        } else if pass_rate >= 60.0 {
            ("!".yellow().to_string(), "warning".yellow().to_string())
        } else {
            ("✗".red().to_string(), "failed".red().to_string())
        };

        println!(
            "  {} {} · {}/{} pass ({:.0}%) · avg {:.2} · {:.1}s",
            status_icon,
            status_text,
            passed_count.to_string().bold(),
            total_count,
            pass_rate,
            avg_score,
            execution_time
        );

        let mut category_stats: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();
        for result in results {
            if let Some(category) = result.case.metadata.get("category") {
                let entry = category_stats.entry(category.clone()).or_insert((0, 0));
                entry.0 += 1;
                if result.passed {
                    entry.1 += 1;
                }
            }
        }

        if !category_stats.is_empty() {
            print!("  ");
            for (i, (category, (total, passed))) in category_stats.iter().enumerate() {
                if i > 0 {
                    print!(" · ");
                }
                let _rate = (*passed as f64 / *total as f64) * 100.0;
                print!("{} {}/{}", category.dimmed(), passed, total);
            }
            println!();
        }
    }
}
