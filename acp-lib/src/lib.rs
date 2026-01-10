/// Agent Credential Proxy - Shared Library
///
/// This library contains core types, error handling, and shared logic
/// used by both the `acp` CLI and `acp-server` daemon.
pub mod error;
pub mod types;

pub use error::{AcpError, Result};
pub use types::{ACPCredentials, ACPPlugin, ACPRequest, AgentToken, Config};
