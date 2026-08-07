//! Device picker — given a set of `runs-on:` labels and a [`Policy`],
//! pick the best device to run the job on.

use crate::pool::{Device, Policy, ShareMode};

/// A decision from the picker.
#[derive(Debug, Clone)]
pub enum Route {
    Owned(Device),
    Pool(Device),
    /// No matching device anywhere; delegate to GitHub CI.
    Github,
    /// No matching device anywhere; queue and retry later.
    Queue,
    /// No matching device and fallback says "fail".
    Fail,
}

/// Decide where a job whose `runs-on` is `labels` should run.
pub fn pick_device_for_job(labels: &[String], policy: &Policy) -> Route {
    // Try owned devices first (unless prefer_pool is true).
    let owned = policy
        .devices
        .iter()
        .find(|d| d.owner.as_deref().unwrap_or(&policy.owner) == policy.owner && d.matches(labels));

    if !policy.prefer_pool {
        if let Some(d) = owned {
            return Route::Owned(d.clone());
        }
    }

    // Pool — any shared device matching the labels.
    if let Some(d) = policy
        .devices
        .iter()
        .find(|d| matches!(d.share, ShareMode::Pool | ShareMode::Open) && d.matches(labels))
    {
        return Route::Pool(d.clone());
    }

    if let Some(d) = owned {
        return Route::Owned(d.clone());
    }

    // Nothing matched — fall back per policy.
    match policy.fallback.missing_platform {
        crate::pool::FallbackStrategy::Github => Route::Github,
        crate::pool::FallbackStrategy::Pool => Route::Pool(Device::default()),
        crate::pool::FallbackStrategy::Queue => Route::Queue,
        crate::pool::FallbackStrategy::Fail => Route::Fail,
    }
}
