//! `act-on` library root.
//!
//! Re-exports the public surface used by the `act-on` CLI and by embedders.

pub mod cli;
pub mod config;
pub mod logger;
pub mod model;
pub mod expr;
pub mod workflow_cmd;
pub mod runner;
pub mod sandbox;
pub mod pool;
pub mod platform;
pub mod action_cache;
pub mod reporter;
pub mod util;

pub use anyhow::{Error, Result};
