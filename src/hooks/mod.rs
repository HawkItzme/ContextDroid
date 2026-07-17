//! Hook installation and lifecycle management for AI coding agents.

use anyhow::{Context, Result};
use std::path::PathBuf;

#[allow(dead_code)]
pub mod constants;
pub mod hook_audit_cmd;
pub mod hook_check;
#[deny(clippy::print_stdout, clippy::print_stderr)]
pub mod hook_cmd;
#[allow(dead_code)]
pub mod integrity;
pub mod permissions;
pub mod rewrite_cmd;
pub mod trust;
pub mod verify_cmd;

pub fn resolve_claude_dir() -> Result<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".claude")))
        .context("cannot determine Claude Code configuration directory")
}
