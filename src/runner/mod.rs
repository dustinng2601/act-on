//! Runner — runtime orchestration.
//!
//! The runner walks a [`crate::model::Plan`], builds a [`RunContext`] per
//! matrix fork, and stages pre/main/post step pipelines in the right order.

pub mod run_context;
pub mod executor;
pub mod job;
pub mod step;
pub mod step_factory;
pub mod step_run;
pub mod step_action;

pub use run_context::RunContext;
pub use executor::{Executor, Pipeline, Parallel};
pub use job::run_job;
