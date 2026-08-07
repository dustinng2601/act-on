//! Step executor — the universal pre/main/post wrapper.
//!
//! For each step we:
//!  1. Allocate an empty `StepResult` in the RunContext.
//!  2. Build `env` from job/step env and `with` inputs (`INPUT_*`), then
//!     interpolate.
//!  3. Evaluate the step `if:` (default status check `Success`).
//!  4. Prepare the `GITHUB_*` file-command files.
//!  5. Dispatch to the appropriate step impl (run / action / docker / ...).
//!  6. Read back the file-command files and apply them to env / outputs.
//!  7. Finalize `StepResult.conclusion` (respect `continue-on-error`).

use std::sync::Arc;

use crate::expr::{eval_if, DefaultStatusCheck};
use crate::model::StepResult;
use crate::model::StepStatus;
use crate::runner::run_context::RunContext;
use crate::Result;

/// The state of a step's execution stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStage {
    Pre,
    Main,
    Post,
}

pub async fn run_step(
    rc: Arc<RunContext>,
    step: crate::model::Step,
    index: usize,
    stage: StepStage,
) -> Result<()> {
    let step_id = step.id.clone().unwrap_or_else(|| format!("step-{}", index));

    // Allocate / reuse StepResult.
    let mut result = rc
        .step_results
        .lock()
        .get(&step_id)
        .cloned()
        .unwrap_or_else(StepResult::new);
    *rc.current_step.lock() = step_id.clone();

    // Evaluate if: (default status check: Pre/Main -> Success, Post -> Always).
    let dsc = match stage {
        StepStage::Pre | StepStage::Main => DefaultStatusCheck::Success,
        StepStage::Post => DefaultStatusCheck::Always,
    };
    let if_expr = step.if_expr.as_deref().unwrap_or("");
    let env = rc.expr_env();
    let enabled = if if_expr.is_empty() {
        true
    } else {
        eval_if(if_expr, &env, dsc).unwrap_or(false)
    };
    if !enabled {
        result.outcome = StepStatus::Skipped;
        result.conclusion = StepStatus::Skipped;
        crate::runner::job::record_outcome(&rc, &step_id, result);
        return Ok(());
    }

    // Prepare file-command files.
    let fc = crate::workflow_cmd::FileCommands::new(&rc.actpath)?;

    // Dispatch to the appropriate step impl.
    let outcome = match stage {
        StepStage::Main => super::step_factory::dispatch_main(rc.clone(), &step, index).await,
        StepStage::Pre | StepStage::Post => Ok(()),
    };

    // Read back file commands.
    let mut env_map = rc.env.lock().clone();
    fc.read_back(&mut result, &mut env_map).ok();
    *rc.env.lock() = env_map;

    // Finalize status. continue-on-error support: replace with runtime eval later.
    let success = outcome.is_ok();
    let coc = step.continue_on_error.is_some();
    crate::workflow_cmd::finalize_status(&mut result, success, coc);
    crate::runner::job::record_outcome(&rc, &step_id, result);

    outcome
}
