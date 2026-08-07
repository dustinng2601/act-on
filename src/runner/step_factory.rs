//! Step factory — pick the right step impl for a model::Step.

use std::sync::Arc;

use crate::model::{Step, StepType};
use crate::Result;
use crate::runner::Executor;

pub fn build_step_executor(
    rc: Arc<crate::runner::RunContext>,
    step: Step,
    index: usize,
) -> Result<Executor> {
    let rc2 = rc.clone();
    let step_clone = step.clone();
    Ok(Executor::new(move || {
        let rc = rc.clone();
        let step = step_clone.clone();
        async move { super::step::run_step(rc, step, index, super::step::StepStage::Main).await }
    }))
    .map(|e| {
        let _ = rc2;
        e
    })
}

pub async fn dispatch_main(
    rc: Arc<crate::runner::RunContext>,
    step: &Step,
    _index: usize,
) -> Result<()> {
    match step.kind() {
        StepType::Run => super::step_run::run_run_step(rc, step).await,
        StepType::UsesActionLocal => super::step_action::run_local_action(rc, step).await,
        StepType::UsesActionRemote => super::step_action::run_remote_action(rc, step).await,
        StepType::UsesDocker => super::step_action::run_docker_action(rc, step).await,
        kind => {
            tracing::warn!(target: "act_on::step", "unsupported step kind {:?} (uses={:?})", kind, step.uses);
            Ok(())
        }
    }
}
