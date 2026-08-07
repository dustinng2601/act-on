//! `run:` step — execute an arbitrary shell command.

use std::path::PathBuf;
use std::sync::Arc;

use crate::model::Step;
use crate::Result;
use crate::runner::RunContext;
use crate::util::shell;

pub async fn run_run_step(rc: Arc<RunContext>, step: &Step) -> Result<()> {
    let raw = step.run.clone().unwrap_or_default();
    // Interpolate ${{ }} in the script.
    let env = rc.expr_env();
    let script = crate::expr::interpolate(&raw, &env)?;

    let shell = step.shell.clone().unwrap_or_else(|| shell::default_shell(&rc));
    let cmd_template = crate::model::workflow::shell_command(&shell)
        .ok_or_else(|| anyhow::anyhow!("invalid shell {shell}"))?;

    // Write the script to a file in actpath.
    let actpath = rc.actpath.clone();
    std::fs::create_dir_all(actpath.join("workflow"))?;
    let ext = shell::script_extension(&shell);
    let script_path = actpath.join("workflow").join(format!("run.{}", ext));
    std::fs::write(&script_path, script)?;

    // Resolve the working directory.
    let workdir = step
        .working_directory
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| rc.workdir.clone());

    // Final command argv: template + script path.
    let final_cmd = cmd_template.replace("{0}", &script_path.to_string_lossy());
    let argv = shell::split(&final_cmd);

    // The files a step writes to in order to say something back: an output, a
    // variable for later steps, a PATH entry, a summary. GitHub hands every step
    // fresh ones and reads them afterwards; without them `>> "$GITHUB_OUTPUT"`
    // redirects to nothing and the shell fails on an empty path.
    let commands = crate::workflow_cmd::FileCommands::new(&rc.actpath)?;
    let mut env_map = rc.env.lock().clone();
    crate::sandbox::file_cmd::populate_env(&mut env_map, &commands);

    let exit = rc
        .sandbox
        .exec(argv, env_map, workdir.to_string_lossy().as_ref())
        .await?;

    // Read back whatever the step wrote, whether or not it succeeded. A step
    // that sets an output and then fails still set it, and a later step asking
    // for it should see what happened rather than nothing.
    apply_file_commands(&rc, step, &commands)?;

    if exit == 0 {
        Ok(())
    } else {
        anyhow::bail!("step exited with code {exit}")
    }
}

/// Fold a step's file commands back into the job.
///
/// Outputs are recorded under the step's `id`, which is what `steps.<id>.outputs`
/// reads. A step without an id cannot be referred to, so its outputs are dropped
/// rather than stored somewhere nothing will look.
pub(super) fn apply_file_commands(
    rc: &RunContext,
    step: &Step,
    commands: &crate::workflow_cmd::FileCommands,
) -> Result<()> {
    let mut result = rc
        .step_results
        .lock()
        .get(step.id.as_deref().unwrap_or_default())
        .cloned()
        .unwrap_or_default();

    let mut env = rc.env.lock().clone();
    commands.read_back(&mut result, &mut env)?;
    *rc.env.lock() = env;

    if let Some(id) = step.id.as_deref() {
        rc.step_results
            .lock()
            .insert(id.to_string(), result);
    }
    Ok(())
}
