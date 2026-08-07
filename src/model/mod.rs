//! Data model: workflow / job / step / strategy / container / action / context.

pub mod action;
pub mod context;
pub mod event;
pub mod plan;
pub mod workflow;

pub use action::{Action, ActionInput, ActionOutput, ActionRuns, ActionRunsUsing};
pub use context::{GithubContext, JobContext, JobStatus, Needs, StepResult, StepStatus};
pub use event::Event;
pub use plan::{Plan, Run, Stage, WorkflowPlanner};
pub use workflow::{ContainerSpec, Job, JobType, Step, StepEnv, StepType, StrategyKind, Workflow};
