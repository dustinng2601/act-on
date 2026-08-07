//! Job executor — orchestrates container start/stop and step pipeline.

use std::sync::Arc;

use crate::model::{JobStatus, StepResult, StepStatus};
use crate::Result;

use super::run_context::RunContext;
use super::step_factory;
use super::Pipeline;

/// Run one job. Returns the final [`JobStatus`].
pub async fn run_job(rc: Arc<RunContext>) -> Result<JobStatus> {
    tracing::info!(target: "act_on::job", "starting job={}", rc.job_id);

    // Start the sandbox environment (no-op on host).
    rc.sandbox.start().await?;

    // Build pipeline.
    let mut pipeline = Pipeline::new();

    // Pre-stage: nothing yet (JS pre-hooks live in step action flow).
    // Main: every step in order.
    for (i, step) in rc.job.steps.iter().enumerate() {
        let exec = step_factory::build_step_executor(rc.clone(), step, i)?;
        pipeline.push(exec);
    }

    let outcome = pipeline.run().await;
    rc.sandbox.stop().await?;

    let status = match outcome {
        Ok(()) => JobStatus::Success,
        Err(_) => JobStatus::Failure,
    };
    *rc.result.lock() = status.to_string();
    tracing::info!(target: "act_on::job", "job={} result={}", rc.job_id, status);
    Ok(status)
}

/// Helper — record a step result.
pub(crate) fn record_outcome(rc: &RunContext, step_id: &str, result: StepResult) {
    let id = if step_id.is_empty() {
        "Step".to_string()
    } else {
        step_id.to_string()
    };
    rc.step_results
        .lock()
        .insert(id, result.clone());
}

#[allow(dead_code)]
fn into_step_status(b: bool) -> StepStatus {
    if b {
        StepStatus::Success
    } else {
        StepStatus::Failure
    }
}
