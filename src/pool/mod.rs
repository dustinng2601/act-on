//! Device pool — declares which devices belong to whom, what can be shared,
//! and what fallback strategy to use when a platform is missing locally.
//!
//! This is the second big extension over nektos/act: a job's `runs-on:`
//! labels are matched against a fleet of owned and pool-shared devices.

pub mod device;
pub mod policy;
pub mod registry;
pub mod router;

pub use device::{Device, DeviceId, DeviceStatus};
pub use policy::{Fallback, FallbackStrategy, Policy, ShareMode};
pub use registry::Registry;
pub use router::pick_device_for_job;
