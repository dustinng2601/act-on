//! `act-on` CLI definition.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::config::{Config, NameValue, parse_kv_file, PlatformMapping};
use crate::model::plan::WorkflowPlanner;
use crate::model::{JobStatus, JobType};
use crate::pool::{Policy, Registry};
use crate::runner::run_job;
use crate::sandbox::HostEnvironment;

#[derive(Parser, Debug)]
#[command(
    name = "act-on",
    bin_name = "act-on",
    version,
    about = "Run GitHub Actions on your own devices + GitHub CI / enterprise pools.",
    long_about = "Cross-platform (Windows / Linux / macOS) local sandbox runner with policy-based device assignment. Rust implementation inspired by nektos/act."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Command>,

    /// Workflows directory.
    #[arg(short = 'W', long = "workflows", default_value = ".github/workflows")]
    pub workflows: PathBuf,

    /// Specific job id to run.
    #[arg(short = 'j', long = "job")]
    pub job: Option<String>,

    /// Actor.
    #[arg(short = 'a', long = "actor", default_value = "act-on")]
    pub actor: String,

    /// Event name.
    #[arg(short = 'e', long = "event", default_value = "push")]
    pub event: String,

    /// Path to the event JSON file.
    #[arg(long = "eventpath")]
    pub eventpath: Option<PathBuf>,

    /// `-P runs-on-label=image` mappings.
    #[arg(short = 'P', long = "platform")]
    pub platforms: Vec<String>,

    /// Secrets (`NAME=VALUE`).
    #[arg(short = 's', long = "secret")]
    pub secrets: Vec<String>,

    /// Variables (`NAME=VALUE`).
    #[arg(long = "var")]
    pub vars: Vec<String>,

    /// Environment variables (`NAME=VALUE`).
    #[arg(long = "env")]
    pub env: Vec<String>,

    /// Inputs for `workflow_dispatch`.
    #[arg(long = "input")]
    pub inputs: Vec<String>,

    /// Matrix includes filter (`key=value`).
    #[arg(long = "matrix")]
    pub matrix: Vec<String>,

    /// Working directory.
    #[arg(short = 'C', long = "directory", default_value = ".")]
    pub directory: PathBuf,

    /// Path to `policy.yml`.
    #[arg(long = "policy")]
    pub policy: Option<PathBuf>,

    /// Dry-run (don't actually run steps).
    #[arg(short = 'n', long = "dryrun")]
    pub dryrun: bool,

    /// List jobs.
    #[arg(short = 'l', long = "list")]
    pub list: bool,

    /// Emit JSON logs.
    #[arg(long = "json")]
    pub json: bool,

    /// Quiet.
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Verbose (-vv for more detail).
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List the jobs in a workflow.
    List { #[arg(short = 'e', long = "event", default_value = "push")] event: String },
    /// Validate a workflow file (parse-only).
    Validate,
    /// Show device pool and policy.
    Pool,
    /// Print the version banner and exit.
    Version,
}

/// Main entry point invoked from `main()`.
pub async fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();

    crate::logger::init(cli.json);

    // Read .env-style files into Config.
    let mut config = Config {
        workdir: cli.directory.clone(),
        workflows: vec![cli.workflows.clone()],
        job: cli.job.clone(),
        actor: cli.actor.clone(),
        event_name: cli.event.clone(),
        event_path: cli.eventpath.clone(),
        secrets: cli.secrets.iter().map(|s| NameValue::parse(s)).collect(),
        vars: cli.vars.iter().map(|s| NameValue::parse(s)).collect(),
        env: cli.env.iter().map(|s| NameValue::parse(s)).collect(),
        workflows_inputs: cli.inputs.iter().map(|s| NameValue::parse(s)).collect(),
        matrix: cli.matrix.clone(),
        platforms: parse_platforms(&cli.platforms),
        policy_path: cli.policy.clone(),
        dryrun: cli.dryrun,
        list: cli.list,
        json_logger: cli.json,
        quiet: cli.quiet,
        verbose: cli.verbose,
        ..Default::default()
    };

    // Honour .env / .secrets convenience files if present in workdir.
    load_dot_files(&mut config);

    // If asked a subcommand, handle it first.
    match &cli.cmd {
        Some(Command::List { event }) => return list_jobs(&config, event).await,
        Some(Command::Validate) => return validate(&config).await,
        Some(Command::Pool) => return print_pool(&config).await,
        Some(Command::Version) => {
            println!("act-on {} ", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        None => {}
    }

    if cli.list {
        return list_jobs(&config, &cli.event).await;
    }

    run_plan(&config).await
}

async fn run_plan(config: &Config) -> anyhow::Result<ExitCode> {
    let planner = WorkflowPlanner::new(&config.workflows[0])?;
    let plan = match &config.job {
        Some(j) => planner.plan_job(j),
        None => planner.plan_event(&config.event_name),
    };

    if plan.is_empty() {
        eprintln!("act-on: no jobs matched event `{}`", config.event_name);
        return Ok(ExitCode::from(1));
    }

    // Load policy if present (optional — without it, just run locally).
    let _policy: Option<Policy> = config
        .policy_path
        .as_ref()
        .and_then(|p| Policy::from_path(p).ok());
    let _registry: Option<Registry> =
        _policy.as_ref().map(|p| Registry::from_policy(p.clone()));

    let mut results = Vec::new();

    for stage in &plan.stages {
        // Parallel within stage, sequential across stages.
        let mut handles = Vec::new();
        for run in &stage.runs {
            let workflow = planner
                .workflows
                .iter()
                .find(|(p, _)| *p == run.workflow_file)
                .map(|(_, w)| w.clone())
                .or_else(|| planner.workflows.first().map(|(_, w)| w.clone()));
            let workflow = workflow.ok_or_else(|| anyhow::anyhow!("workflow not found for job {}", run.job_id))?;
            let job = workflow
                .jobs
                .get(&run.job_id)
                .ok_or_else(|| anyhow::anyhow!("job `{}` not found in workflow", run.job_id))?
                .clone();
            if job.kind() != JobType::Default {
                continue;
            }

            let config = Arc::new(config.clone());
            let workflow = Arc::new(workflow);
            let workdir = config.workdir.clone();
            let sandbox = Arc::new(HostEnvironment::new(workdir)?);
            let rc = Arc::new(crate::runner::RunContext::new(
                config, workflow, run.job_id.clone(), job, Default::default(), sandbox,
            ));

            let rc_clone = rc.clone();
            handles.push(tokio::spawn(async move {
                let status = run_job(rc_clone).await?;
                Ok::<_, anyhow::Error>((run.job_id.clone(), status))
            }));
        }
        for h in handles {
            match h.await {
                Ok(Ok((id, status))) => {
                    results.push((id, status));
                }
                Ok(Err(e)) => {
                    eprintln!("act-on: job failed: {e:#}");
                    return Ok(ExitCode::FAILURE);
                }
                Err(e) => {
                    eprintln!("act-on: job panicked: {e}");
                    return Ok(ExitCode::FAILURE);
                }
            }
        }
    }

    let any_failed = results
        .iter()
        .any(|(_, s)| matches!(s, JobStatus::Failure | JobStatus::Cancelled));
    crate::reporter::print_run_summary(&plan, &results);
    Ok(if any_failed { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}

async fn list_jobs(config: &Config, event: &str) -> anyhow::Result<ExitCode> {
    let planner = WorkflowPlanner::new(&config.workflows[0])?;
    let plan = planner.plan_event(event);
    eprintln!("Workflows / jobs for event `{event}`:");
    if plan.is_empty() {
        eprintln!("  (no jobs matched)");
    }
    for (i, stage) in plan.stages.iter().enumerate() {
        for run in &stage.runs {
            eprintln!("  stage {i}: {}", run.job_id);
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn validate(config: &Config) -> anyhow::Result<ExitCode> {
    let planner = WorkflowPlanner::new(&config.workflows[0])?;
    eprintln!("ok: {} workflow(s) loaded", planner.workflows.len());
    Ok(ExitCode::SUCCESS)
}

async fn print_pool(config: &Config) -> anyhow::Result<ExitCode> {
    let Some(p) = &config.policy_path else {
        eprintln!("act-on: no --policy provided");
        return Ok(ExitCode::from(2));
    };
    let policy = Policy::from_path(p)?;
    eprintln!("owner: {}", policy.owner);
    eprintln!("prefer_pool: {}", policy.prefer_pool);
    eprintln!("fallback.missing_platform: {:?}", policy.fallback.missing_platform);
    eprintln!("devices:");
    for d in &policy.devices {
        eprintln!(
            "  {} {:?} {:?} share={:?} labels={:?}",
            d.id, d.os, d.arch, d.share, d.labels
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_platforms(list: &[String]) -> PlatformMapping {
    let mut m = std::collections::HashMap::new();
    for p in list {
        if let Some((k, v)) = p.split_once('=') {
            m.insert(k.trim().into(), v.trim().into());
        }
    }
    PlatformMapping(m)
}

fn load_dot_files(config: &mut Config) {
    for (file, target) in [
        (".env", &mut config.env),
        (".secrets", &mut config.secrets),
        (".vars", &mut config.vars),
    ] {
        let path = config.workdir.join(file);
        if let Ok(s) = std::fs::read_to_string(&path) {
            target.extend(parse_kv_file(&s));
        }
    }
}
