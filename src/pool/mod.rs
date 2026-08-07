//! Device pool — declares which devices belong to whom, what can be shared,
//! and what fallback strategy to use when a platform is missing locally.
//!
//! This is the second big extension over nektos/act: a job's `runs-on:`
//! labels are matched against a fleet of owned and pool-shared devices.

pub mod policy;
pub mod device;
pub mod router;
pub mod registry;

pub use policy::{Policy, Fallback, FallbackStrategy, ShareMode};
pub use device::{Device, DeviceId, DeviceStatus};
pub use router::pick_device_for_job;
pub use registry::Registry;
