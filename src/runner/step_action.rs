//! `uses:` steps — local / remote / docker actions.

use std::sync::Arc;

use crate::model::{Step, StepType};
use crate::Result;
use crate::runner::RunContext;

pub async fn run_local_action(rc: Arc<RunContext>, step: &Step) -> Result<()> {
    let uses = step.uses.clone().unwrap_or_default();
    tracing::info!(target: "act_on::action", "local action uses={uses}");

    // A local action is a directory in the checkout rather than something to
    // fetch. That is the only difference from a remote one, so the path is
    // resolved here and everything after it is shared.
    let action_dir = rc.workdir.join(uses.trim_start_matches("./"));
    if !action_dir.is_dir() {
        anyhow::bail!(
            "local action {uses} is not a directory at {}",
            action_dir.display()
        );
    }
    dispatch_action(rc, step, &action_dir, &uses).await
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
    dispatch_action(rc, step, &action_dir, &uses).await
}

/// Read an action's manifest and run it, whatever kind it turns out to be.
///
/// Shared by the local and remote paths: where the directory came from stops
/// mattering once it exists, and one dispatch is what stops a local action
/// quietly behaving differently from the same action fetched.
async fn dispatch_action(
    rc: Arc<RunContext>,
    step: &Step,
    action_dir: &std::path::Path,
    uses: &str,
) -> Result<()> {
    let action = crate::model::action::read_action(action_dir)?;
    // `runs` is optional in the schema, and the predicates below belong to it.
    // An action without it declares no way to run, which is worth saying rather
    // than silently taking none of the branches.
    let Some(runs) = action.runs.as_ref() else {
        tracing::warn!(
            target: "act_on::action",
            "action at {} declares no `runs` section; nothing to execute",
            action_dir.display()
        );
        return Ok(());
    };
    if runs.is_composite() {
        run_composite(rc, step, &action, runs).await
    } else if runs.is_node() {
        run_node(rc, step, &action, runs, action_dir).await
    } else if runs.is_docker() {
        // Docker needs a second container sharing the job's network, which the
        // sandbox does not do yet. Reported rather than passed over: a step that
        // silently did nothing is worse than one that says it cannot run.
        anyhow::bail!("docker actions are not supported yet ({uses})")
    } else {
        anyhow::bail!("action {uses} declares an unsupported `using:`")
    }
}

/// Inputs as the environment an action reads them from.
///
/// GitHub passes `with:` through as `INPUT_<NAME>`, upper-cased with spaces
/// turned into underscores, and fills any input the caller omitted from the
/// action's own `default:`. Without the defaults an action reading one gets
/// nothing, which it cannot tell apart from the input being set to empty.
fn input_env(
    rc: &RunContext,
    step: &Step,
    action: &crate::model::action::Action,
) -> Result<std::collections::HashMap<String, String>> {
    let mut env = rc.env.lock().clone();
    let expr = rc.expr_env();

    for (name, input) in &action.inputs {
        if let Some(default) = input.default.as_ref() {
            env.insert(input_key(name), crate::expr::interpolate(default, &expr)?);
        }
    }
    // `with` is optional on a step; absent means no caller-supplied inputs, and
    // the action's own defaults above still stand.
    for (name, value) in step.with.iter().flatten() {
        let raw = match value {
            serde_yaml::Value::String(s) => s.clone(),
            other => serde_yaml::to_string(other)
                .unwrap_or_default()
                .trim_end()
                .to_string(),
        };
        env.insert(input_key(name), crate::expr::interpolate(&raw, &expr)?);
    }
    Ok(env)
}

fn input_key(name: &str) -> String {
    format!("INPUT_{}", name.to_uppercase().replace(' ', "_"))
}

/// Run a JavaScript action: `node <action_dir>/<main>`.
async fn run_node(
    rc: Arc<RunContext>,
    step: &Step,
    action: &crate::model::action::Action,
    runs: &crate::model::action::ActionRuns,
    action_dir: &std::path::Path,
) -> Result<()> {
    let main = runs
        .main
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("node action declares no `main:`"))?;
    let entry = action_dir.join(main);
    if !entry.exists() {
        anyhow::bail!("node action entry point {} does not exist", entry.display());
    }

    let env = input_env(&rc, step, action)?;
    let argv = vec!["node".to_string(), entry.to_string_lossy().into_owned()];
    tracing::info!(target: "act_on::action", "node action → {}", entry.display());

    let exit = rc
        .sandbox
        .exec(argv, env, rc.workdir.to_string_lossy().as_ref())
        .await?;
    if exit == 0 {
        Ok(())
    } else {
        anyhow::bail!("action exited with code {exit}")
    }
}

/// Run a composite action by running its steps in order.
///
/// Sub-steps are ordinary [`Step`]s and go back through the same dispatch as
/// everything else, which is what lets a composite hold `run:` and `uses:`
/// alike — including another composite.
async fn run_composite(
    rc: Arc<RunContext>,
    step: &Step,
    action: &crate::model::action::Action,
    runs: &crate::model::action::ActionRuns,
) -> Result<()> {
    tracing::info!(
        target: "act_on::action",
        "composite action → {} step(s)",
        runs.steps.len()
    );

    // Sub-steps read inputs from the environment like any other step, and they
    // run through the job's context — so the inputs are put there for the
    // duration and taken out again. Leaving them would let one action's inputs
    // be read by a later step that never declared them.
    let inputs = input_env(&rc, step, action)?;
    let added: Vec<String> = {
        let mut env = rc.env.lock();
        inputs
            .iter()
            .filter(|(k, _)| k.starts_with("INPUT_"))
            .map(|(k, v)| {
                env.insert(k.clone(), v.clone());
                k.clone()
            })
            .collect()
    };

    let mut result = Ok(());
    for (i, sub) in runs.steps.iter().enumerate() {
        tracing::debug!(
            target: "act_on::action",
            "composite step {i} of {:?}",
            step.uses
        );
        let exec = super::step_factory::build_step_executor(rc.clone(), sub.clone(), i)?;
        // Stop at the first failure, but still restore the environment below —
        // an early return here would leave the inputs behind.
        if let Err(error) = exec.run().await {
            result = Err(error);
            break;
        }
    }

    {
        let mut env = rc.env.lock();
        for key in added {
            env.remove(&key);
        }
    }
    result
}

pub async fn run_docker_action(_rc: Arc<RunContext>, _step: &Step) -> Result<()> {
    // TODO v1.2: spawn a separate docker container sharing the job container
    // network (`--network container:<id>`). For v0.1 we just bail with a
    // friendly message.
    Err(anyhow::anyhow!("docker:// actions are not yet supported in v0.1"))
}

// re-export StepType so callers can match on it
pub use crate::model::StepType as Kind;

#[cfg(test)]
mod tests {
    use super::input_key;

    #[test]
    fn an_input_becomes_the_variable_an_action_reads() {
        assert_eq!(input_key("token"), "INPUT_TOKEN");
        assert_eq!(input_key("fetch-depth"), "INPUT_FETCH-DEPTH");
        // Spaces are legal in an input name and are not legal in a variable
        // name, so GitHub substitutes them.
        assert_eq!(input_key("my input"), "INPUT_MY_INPUT");
        assert_eq!(input_key("Mixed Case"), "INPUT_MIXED_CASE");
    }
}
