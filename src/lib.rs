//! Offline, read-only MCP projection of approved public service metadata.

mod config;
mod file_input;
mod protocol;
mod server;
mod transport;

pub use config::Config;
pub use protocol::Gateway;
pub use transport::run_stdio;

#[cfg(test)]
mod unit_tests;
