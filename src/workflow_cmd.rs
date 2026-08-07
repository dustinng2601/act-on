//! Workflow command & file-command protocol.
//!
//! Legacy `::command::` lines and modern `GITHUB_*` files are both handled
//! here. The actual step driver writes the files and reads them back via
//! [`FileCommands::read_back`]; the line-based commands are handled by
//! [`LineHandler`] as output streams in.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::model::{StepResult, StepStatus};

/// Lines matching `::command name=value,name=value::arg`.
pub struct LineHandler;

#[derive(Debug, Clone, Default)]
pub struct WorkflowCommand {
    pub name: String,
    pub kv: HashMap<String, String>,
    pub arg: String,
}

impl LineHandler {
    /// Try to parse a single line as a workflow command. Returns the
    /// leftover text if the line is not a command.
    pub fn parse(line: &str) -> Option<WorkflowCommand> {
        let line = line.trim_end_matches(['\r', '\n']);
        if !line.starts_with("::") || line.len() < 4 {
            return None;
        }
        let body = &line[2..];
        let end = body.find("::")?;
        let head = body[..end].trim();
        let arg = unescape(&body[end + 2..]);
        let (name, kv) = parse_head(head);
        Some(WorkflowCommand {
            name,
            kv,
            arg,
        })
    }
}

fn parse_head(head: &str) -> (String, HashMap<String, String>) {
    let mut kv = HashMap::new();
    let mut parts = head.splitn(2, ' ');
    let name = parts.next().unwrap_or("").to_string();
    if let Some(rest) = parts.next() {
        for pair in rest.split(',') {
            let mut iter = pair.splitn(2, '=');
            let k = unescape(iter.next().unwrap_or(""));
            let v = unescape(iter.next().unwrap_or(""));
            kv.insert(k, v);
        }
    }
    (name, kv)
}

fn unescape(s: &str) -> String {
    s.replace("%25", "%")
        .replace("%0D", "\r")
        .replace("%0A", "\n")
}

/// Handle a parsed workflow command. Mutates `step_result` / `env` in place.
pub fn handle_command(
    cmd: &WorkflowCommand,
    step_result: &mut StepResult,
    env: &mut HashMap<String, String>,
    masks: &mut Vec<String>,
) {
    match cmd.name.as_str() {
        "set-output" => {
            if let Some(name) = cmd.kv.get("name") {
                step_result
                    .outputs
                    .insert(name.clone(), cmd.arg.clone());
            }
        }
        "save-state" => {
            // Stored in step-scoped state, surfaced later via STATE_<name>.
            let name = cmd.kv.get("name").cloned().unwrap_or_default();
            step_result
                .outputs
                .insert(format!("__state__{}", name), cmd.arg.clone());
        }
        "add-path" => {
            let cur = env.entry("PATH".into()).or_default();
            cur.insert_str(0, &format!("{}:", cmd.arg.trim()));
        }
        "add-mask" => masks.push(cmd.arg.clone()),
        "debug" => tracing::debug!(target: "act_on::step", "{}", cmd.arg),
        "warning" => tracing::warn!(target: "act_on::step", "{}", cmd.arg),
        "error" => tracing::error!(target: "act_on::step", "{}", cmd.arg),
        "stop-commands" => {
            // respected by the caller toggling its own LineHandler state.
        }
        "set-env" | "add-matcher" | "remove-matcher" => {
            // recognised but unimplemented (parity with nektos/act).
        }
        _ => {}
    }
}

/// Path layout for the file-based commands emitted to `GITHUB_*`.
pub struct FileCommands {
    pub output: PathBuf,
    pub state: PathBuf,
    pub path: PathBuf,
    pub env: PathBuf,
    pub summary: PathBuf,
}

impl FileCommands {
    /// Build the set of empty command files inside `actpath/workflow/`.
    pub fn new(actpath: &Path) -> std::io::Result<Self> {
        let dir = actpath.join("workflow");
        std::fs::create_dir_all(&dir)?;
        let fc = Self {
            output: dir.join("outputcmd.txt"),
            state: dir.join("statecmd.txt"),
            path: dir.join("pathcmd.txt"),
            env: dir.join("envs.txt"),
            summary: dir.join("SUMMARY.md"),
        };
        for f in [&fc.output, &fc.state, &fc.path, &fc.env, &fc.summary] {
            if !f.exists() {
                std::fs::write(f, "")?;
            }
        }
        std::fs::write(&fc.summary, "")?;
        Ok(fc)
    }

    /// Read file commands back into env / step_result.
    pub fn read_back(
        &self,
        step_result: &mut StepResult,
        env: &mut HashMap<String, String>,
    ) -> std::io::Result<()> {
        // output
        for kv in parse_env_file(&self.output)? {
            step_result.outputs.insert(kv.0, kv.1);
        }
        // state
        for kv in parse_env_file(&self.state)? {
            step_result
                .outputs
                .insert(format!("__state__{}", kv.0), kv.1);
        }
        // path
        let path_lines = std::fs::read_to_string(&self.path)?;
        if let Some(cur) = env.get_mut("PATH") {
            let mut prepend = path_lines
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect::<Vec<_>>()
                .join(":");
            if !prepend.is_empty() {
                prepend.push(':');
                cur.insert_str(0, &prepend);
            }
        }
        // env
        for kv in parse_env_file(&self.env)? {
            env.insert(kv.0, kv.1);
        }
        Ok(())
    }
}

/// Parse a `KEY=VALUE` file (one per line).
fn parse_env_file(p: &Path) -> std::io::Result<Vec<(String, String)>> {
    let s = std::fs::read_to_string(p)?;
    let mut out = Vec::new();
    for line in s.lines() {
        if line.is_empty() {
            continue;
        }
        if let Some(eq) = line.find('=') {
            out.push((line[..eq].trim().to_string(), line[eq + 1..].trim().to_string()));
        }
    }
    Ok(out)
}

/// Apply a `StepStatus` to a `StepResult`, honouring `continue-on-error`.
pub fn finalize_status(
    step_result: &mut StepResult,
    success: bool,
    continue_on_error: bool,
) {
    step_result.outcome = if success {
        StepStatus::Success
    } else {
        StepStatus::Failure
    };
    step_result.conclusion = if success || continue_on_error {
        StepStatus::Success
    } else {
        StepStatus::Failure
    };
}
