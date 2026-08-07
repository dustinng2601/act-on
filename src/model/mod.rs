//! Data model: workflow / job / step / strategy / container / action / context.

pub mod workflow;
pub mod action;
pub mod context;
pub mod plan;
pub mod event;

pub use workflow::{Workflow, Job, Step, StrategyKind, ContainerSpec, StepType, JobType, StepEnv};
pub use action::{Action, ActionRuns, ActionRunsUsing, ActionInput, ActionOutput};
pub use context::{GithubContext, JobContext, JobStatus, StepResult, StepStatus, Needs};
pub use plan::{WorkflowPlanner, Plan, Stage, Run};
pub use event::Event;
