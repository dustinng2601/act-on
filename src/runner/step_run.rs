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

    let env_map = rc.env.lock().clone();
    let exit = rc
        .sandbox
        .exec(argv, env_map, workdir.to_string_lossy().as_ref())
        .await?;
    if exit == 0 {
        Ok(())
    } else {
        anyhow::bail!("step exited with code {exit}")
    }
}
