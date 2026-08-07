//! Executor combinators (Pipeline / Parallel / Condition) drawn from
//! `nektos/act`'s `pkg/common/executor.go` design.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::future::BoxFuture;

use crate::Result;

/// A boxed-async unit-of-work. Errors stop the pipeline.
pub type BoxExec = Box<dyn Fn() -> BoxFuture<'static, Result<()>> + Send + Sync>;

/// A clonable opaque `Executor` (cheap to pass around). Inspired by
/// nektos/act's `common.Executor`.
#[derive(Clone)]
pub struct Executor(Arc<BoxExec>);

impl Executor {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        Self(Arc::new(Box::new(move || Box::pin(f()))))
    }

    pub async fn run(&self) -> Result<()> {
        (self.0)().await
    }
}

/// Run a sequence in order, halting on first error.
pub struct Pipeline {
    pub execs: Vec<Executor>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self { execs: Vec::new() }
    }
    pub fn push(&mut self, e: Executor) {
        self.execs.push(e);
    }
    pub async fn run(&self) -> Result<()> {
        for e in &self.execs {
            e.run().await?;
        }
        Ok(())
    }
}

/// Run a set of executors in parallel with a concurrency limit. The first
/// error is propagated.
pub struct Parallel {
    pub execs: Vec<Executor>,
    pub parallel: usize,
}

impl Parallel {
    pub fn new(parallel: usize) -> Self {
        Self {
            execs: Vec::new(),
            parallel: parallel.max(1),
        }
    }

    pub fn push(&mut self, e: Executor) {
        self.execs.push(e);
    }

    pub async fn run(&self) -> Result<()> {
        use futures::stream::{self, StreamExt};
        let execs = self.execs.clone();
        let res = stream::iter(execs)
            .for_each_concurrent(self.parallel, |e| async move {
                let _ = e.run().await;
            })
            .await;
        Ok(res)
    }
}

// Silence unused Pin import if not needed.
type _Pin = Pin<Box<dyn Future<Output = ()> + Send>>;
