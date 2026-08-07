//! `act-on` library root.
//!
//! Re-exports the public surface used by the `act-on` CLI and by embedders.

pub mod action_cache;
pub mod cli;
pub mod config;
pub mod expr;
pub mod logger;
pub mod model;
pub mod platform;
pub mod pool;
pub mod reporter;
pub mod runner;
pub mod sandbox;
pub mod util;
pub mod workflow_cmd;

pub use anyhow::{Error, Result};
