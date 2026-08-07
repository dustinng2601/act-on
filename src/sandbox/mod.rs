//! Sandbox — abstractions for running steps either on the host or in a
//! container.

pub mod container;
pub mod env;
pub mod file_cmd;
pub mod host;
pub mod profile;

pub use container::DockerEnvironmentStub as DockerEnvironment;
pub use env::{ExecResult, ExecutionsEnvironment};
pub use host::HostEnvironment;
pub use profile::SandboxProfile;
