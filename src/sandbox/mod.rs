//! Sandbox — abstractions for running steps either on the host or in a
//! container.

pub mod env;
pub mod host;
pub mod container;
pub mod profile;
pub mod file_cmd;

pub use env::{ExecutionsEnvironment, ExecResult};
pub use host::HostEnvironment;
pub use container::DockerEnvironmentStub as DockerEnvironment;
pub use profile::SandboxProfile;
