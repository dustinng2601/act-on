//! `uses:` steps — local / remote / docker actions.

use std::sync::Arc;

use crate::model::{Step, StepType};
use crate::Result;
use crate::runner::RunContext;

pub async fn run_local_action(_rc: Arc<RunContext>, step: &Step) -> Result<()> {
    tracing::info!(target: "act_on::action", "local action uses={:?}", step.uses);
    // TODO v1.1: read action.yml from <workdir>/<uses>/action.yml, dispatch
    // to composite or docker path. For now, composite only.
    Ok(())
}

pub async fn run_remote_action(rc: Arc<RunContext>, step: &Step) -> Result<()> {
    let uses = step.uses.clone().unwrap_or_default();
    tracing::info!(target: "act_on::action", "remote action uses={uses}");

    // actions/checkout short-circuit: copy the workdir into the sandbox.
    if let Some(name) = crate::action_cache::parse_uses(&uses) {
        if name.org == "actions" && name.repo == "checkout" {
            tracing::info!(target: "act_on::action", "actions/checkout short-circuit");
            rc.sandbox
                .copy_dir(&rc.workdir, rc.sandbox.workspace().to_string_lossy().as_ref())
                .await?;
            return Ok(());
        }
    }

    // Clone / fetch the action and read action.yml.
    let action_dir = crate::action_cache::fetch(&rc.config, &uses).await?;
    if action_dir.is_none() {
        tracing::warn!(target: "act_on::action", "no action cloned (dry-run or unsupported)");
        return Ok(());
    }
    let action_dir = action_dir.unwrap();
    let action = crate::model::action::read_action(&action_dir)?;
    if action.runs.is_composite() {
        // TODO v1.1: recurse into composite.
        tracing::info!(target: "act_on::action", "composite action (TODO)");
    } else if action.runs.is_node() {
        tracing::info!(target: "act_on::action", "node action (TODO)");
    } else if action.runs.is_docker() {
        tracing::info!(target: "act_on::action", "docker action (TODO)");
    }
    Ok(())
}

pub async fn run_docker_action(_rc: Arc<RunContext>, _step: &Step) -> Result<()> {
    // TODO v1.2: spawn a separate docker container sharing the job container
    // network (`--network container:<id>`). For v0.1 we just bail with a
    // friendly message.
    Err(anyhow::anyhow!("docker:// actions are not yet supported in v0.1"))
}

// re-export StepType so callers can match on it
pub use crate::model::StepType as Kind;
