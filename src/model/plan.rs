//! Workflow planner — builds a `Plan{Stages[]{Runs[]}}` from `needs:`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::{Job, JobType, Workflow};

/// A single job (with optional matrix-forked variants) scheduled for execution.
#[derive(Debug, Clone)]
pub struct Run {
    pub workflow_file: PathBuf,
    pub job_id: String,
    /// One combination of the job's `strategy.matrix`, empty when it has none.
    ///
    /// A job with a matrix becomes one `Run` per combination — that is what
    /// makes `matrix.os` mean something, and what makes the job run more than
    /// once.
    pub matrix: std::collections::HashMap<String, serde_yaml::Value>,
}

/// A stage is a set of jobs that can run in parallel.
#[derive(Debug, Clone, Default)]
pub struct Stage {
    pub runs: Vec<Run>,
}

/// The full plan, with stages running sequentially and jobs inside a stage
/// running in parallel.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub stages: Vec<Stage>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.stages.iter().all(|s| s.runs.is_empty())
    }
}

/// Plan builder.
pub struct WorkflowPlanner {
    pub workflows: Vec<(PathBuf, Workflow)>,
}

impl WorkflowPlanner {
    /// Reads every `.yml`/`.yaml` under `workflows_dir`.
    pub fn new(workflows_dir: &Path) -> anyhow::Result<Self> {
        let mut workflows = Vec::new();
        if workflows_dir.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(workflows_dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.is_file()
                        && matches!(
                            p.extension().and_then(|s| s.to_str()),
                            Some("yml") | Some("yaml")
                        )
                })
                .collect();
            entries.sort();
            for path in entries {
                let bytes = std::fs::read(&path)?;
                let mut wf: Workflow = serde_yaml::from_slice(&bytes)
                    .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
                wf.file = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(str::to_string);
                workflows.push((path, wf));
            }
        }
        Ok(Self { workflows })
    }

    /// Plan one event (`push`, `pull_request`, `workflow_dispatch`, ...).
    pub fn plan_event(&self, event: &str) -> Plan {
        let mut stage_map: HashMap<String, (Workflow, Job)> = HashMap::new();
        for (_, wf) in &self.workflows {
            for (job_id, job) in &wf.jobs {
                if job.kind() != JobType::Default {
                    continue;
                }
                if Self::job_hooked_on_event(wf, job_id, event) {
                    stage_map.insert(job_id.clone(), (wf.clone(), job.clone()));
                }
            }
        }
        Self::build_plan(stage_map)
    }

    /// Plan a specific single job regardless of `on:`.
    pub fn plan_job(&self, job_id: &str) -> Plan {
        let mut stage_map: HashMap<String, (Workflow, Job)> = HashMap::new();
        for (_, wf) in &self.workflows {
            if let Some((id, job)) = wf.jobs.iter().find(|(k, _)| *k == job_id) {
                stage_map.insert(id.clone(), (wf.clone(), job.clone()));
                break;
            }
        }
        Self::build_plan(stage_map)
    }

    /// Plan every job across every workflow.
    pub fn plan_all(&self) -> Plan {
        let mut stage_map: HashMap<String, (Workflow, Job)> = HashMap::new();
        for (_, wf) in &self.workflows {
            for (job_id, job) in &wf.jobs {
                if job.kind() == JobType::Default {
                    stage_map.insert(job_id.clone(), (wf.clone(), job.clone()));
                }
            }
        }
        Self::build_plan(stage_map)
    }

    /// Build the dependency-ordered plan from a flat job map.
    fn build_plan(jobs: HashMap<String, (Workflow, Job)>) -> Plan {
        // Topologically sort jobs by `needs:`.
        let mut indeg: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for (id, (_, job)) in &jobs {
            indeg.entry(id.clone()).or_insert(0);
            for dep in job.needs() {
                if !jobs.contains_key(&dep) {
                    continue;
                }
                graph.entry(dep.clone()).or_default().push(id.clone());
                *indeg.entry(id.clone()).or_insert(0) += 1;
            }
        }

        let mut plan = Plan::default();
        let mut visited: HashSet<String> = HashSet::new();
        while visited.len() < jobs.len() {
            let mut stage = Stage::default();
            for (id, indegree) in &indeg {
                if *indegree == 0 && !visited.contains(id) {
                    let workflow_file = jobs
                        .get(id)
                        .map(|(w, _)| PathBuf::from(w.file.clone().unwrap_or_default()))
                        .unwrap_or_default();
                    // One run per matrix combination. A job without a matrix
                    // yields a single empty one, so this stays the ordinary case.
                    for matrix in jobs
                        .get(id)
                        .map(|(_, j)| matrix_combinations(j))
                        .unwrap_or_else(|| vec![Default::default()])
                    {
                        stage.runs.push(Run {
                            workflow_file: workflow_file.clone(),
                            job_id: id.clone(),
                            matrix,
                        });
                    }
                }
            }
            if stage.runs.is_empty() {
                // dependency cycle; bail with what we have.
                break;
            }
            for r in &stage.runs {
                visited.insert(r.job_id.clone());
                indeg.insert(r.job_id.clone(), usize::MAX);
                if let Some(deps) = graph.get(&r.job_id) {
                    for d in deps {
                        if let Some(v) = indeg.get_mut(d) {
                            if *v > 0 {
                                *v -= 1;
                            }
                        }
                    }
                }
            }
            plan.stages.push(stage);
        }
        plan
    }

    /// Check whether a job hooks the given event based on `on:`.
    fn job_hooked_on_event(wf: &Workflow, _job_id: &str, event: &str) -> bool {
        match &wf.raw_on {
            Some(serde_yaml::Value::String(s)) => s == event,
            Some(serde_yaml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str())
                .any(|s| s == event),
            Some(serde_yaml::Value::Mapping(m)) => m
                .keys()
                .filter_map(|k| k.as_str())
                .any(|k| k == event),
            None => event == "push",
            // `on:` written as a scalar other than a string — a bare number or
            // boolean — names no event, so nothing hooks it.
            Some(_) => false,
        }
    }
}

/// Every combination of a job's `strategy.matrix`.
///
/// Returns a single empty combination when the job has no matrix, so a caller
/// can treat both alike. `include` and `exclude` are not applied here: a bare
/// matrix is what the cartesian product describes, and honouring the directives
/// belongs with them rather than half-done.
fn matrix_combinations(
    job: &Job,
) -> Vec<std::collections::HashMap<String, serde_yaml::Value>> {
    let Some(serde_yaml::Value::Mapping(map)) = job
        .strategy
        .as_ref()
        .and_then(|s| s.matrix.as_ref())
    else {
        return vec![Default::default()];
    };

    let mut dims: std::collections::HashMap<String, Vec<serde_yaml::Value>> =
        std::collections::HashMap::new();
    for (key, value) in map {
        let Some(key) = key.as_str() else { continue };
        // `include` / `exclude` are directives, not dimensions; folding them in
        // here would invent combinations nobody asked for.
        if key == "include" || key == "exclude" {
            continue;
        }
        if let serde_yaml::Value::Sequence(values) = value {
            dims.insert(key.to_string(), values.clone());
        }
    }

    let combos = crate::util::cartesian::product(&dims);
    if combos.is_empty() {
        vec![Default::default()]
    } else {
        combos
    }
}
