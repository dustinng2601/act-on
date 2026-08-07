//! Reporter — pretty-print pass/fail per step and per job.

use std::sync::Arc;

use crate::model::JobStatus;
use crate::runner::RunContext;

pub fn print_job_summary(rc: &Arc<RunContext>, status: JobStatus) {
    let symbol = match status {
        JobStatus::Success => "✓",
        JobStatus::Failure => "✗",
        JobStatus::Cancelled => "⊘",
        JobStatus::Skipped => "·",
    };
    eprintln!("  {symbol}  {}", rc.job_id);
}

pub fn print_run_summary(plan: &crate::model::Plan, results: &[(String, JobStatus)]) {
    eprintln!();
    eprintln!("  Workflow result:");
    for (job_id, status) in results {
        let symbol = match status {
            JobStatus::Success => "✓",
            JobStatus::Failure => "✗",
            JobStatus::Cancelled => "⊘",
            JobStatus::Skipped => "·",
        };
        let colour = match status {
            JobStatus::Success => "\x1b[32m",
            JobStatus::Failure | JobStatus::Cancelled => "\x1b[31m",
            JobStatus::Skipped => "\x1b[90m",
        };
        eprintln!("  {colour}{symbol} {job_id}\x1b[0m");
    }
    let _ = plan;
}
