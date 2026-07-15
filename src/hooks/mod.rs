//! Hook installation and lifecycle management for AI coding agents.

#[allow(dead_code)]
pub mod constants;
pub mod hook_audit_cmd;
pub mod hook_check;
#[deny(clippy::print_stdout, clippy::print_stderr)]
pub mod hook_cmd;
// Retained only as migration/reference code while the unsafe inherited `init`
// command is quarantined in favor of the verified integrations lifecycle.
#[allow(dead_code)]
pub mod init;
#[allow(dead_code)]
pub mod integrity;
pub mod permissions;
pub mod rewrite_cmd;
pub mod trust;
pub mod verify_cmd;
