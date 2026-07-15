//! Idempotent, scoped integration lifecycle helpers.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const CLAUDE_COMMAND: &str = "contextdroid hook claude";
const CURSOR_COMMAND: &str = "contextdroid hook cursor";
const CODEX_START: &str = "<!-- contextdroid-managed:start v1 -->";
const CODEX_END: &str = "<!-- contextdroid-managed:end -->";
const CODEX_BLOCK: &str = "<!-- contextdroid-managed:start v1 -->\n# ContextDroid managed instructions\n\nContextDroid does not transparently intercept Codex shell commands. Use `contextdroid gradlew`, `contextdroid adb`, or `contextdroid logcat` explicitly for supported Android diagnostics. Keep pipelines, redirects, structured output, security tools, binaries, unknown commands, and this repository's own build/test/diff/log commands raw. Recover optimized output with `contextdroid show <RUN_ID> --raw`.\n<!-- contextdroid-managed:end -->";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Cursor,
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Preview,
    Install,
    Status,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationResult {
    pub changed: bool,
    pub installed: bool,
    pub path: PathBuf,
    pub preview: String,
}

pub fn run(
    agent: Agent,
    action: Action,
    root: Option<PathBuf>,
    cursor_schema_version: Option<u32>,
) -> Result<IntegrationResult> {
    let root = match root {
        Some(root) => root,
        None => default_root(agent)?,
    };
    match agent {
        Agent::Claude => json_lifecycle(
            root.join("settings.json"),
            "PreToolUse",
            claude_entry(),
            CLAUDE_COMMAND,
            action,
        ),
        Agent::Cursor => {
            let version = cursor_schema_version.unwrap_or(1);
            if version != 1 {
                anyhow::bail!("Cursor hook schema version {version} is not verified; supported: 1");
            }
            cursor_lifecycle(root.join("hooks.json"), action)
        }
        Agent::Codex => codex_lifecycle(root.join("AGENTS.md"), action),
    }
}

fn default_root(agent: Agent) -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(match agent {
        Agent::Claude => home.join(".claude"),
        Agent::Cursor => home.join(".cursor"),
        Agent::Codex => std::env::current_dir()?,
    })
}

fn claude_entry() -> Value {
    json!({
        "matcher": "Bash",
        "hooks": [{ "type": "command", "command": CLAUDE_COMMAND }]
    })
}

fn cursor_entry() -> Value {
    json!({ "command": CURSOR_COMMAND, "matcher": "Shell" })
}

fn json_lifecycle(
    path: PathBuf,
    hook_key: &str,
    entry: Value,
    command: &str,
    action: Action,
) -> Result<IntegrationResult> {
    let mut root = read_json_object(&path, json!({}))?;
    let installed = json_hook_entries(&root, hook_key)
        .is_some_and(|entries| entries.iter().any(|value| contains_command(value, command)));
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed => {
            json_hook_entries_mut(&mut root, hook_key)?.push(entry);
            changed = true;
        }
        Action::Uninstall if installed => {
            json_hook_entries_mut(&mut root, hook_key)?
                .retain(|value| !contains_command(value, command));
            changed = true;
        }
        _ => {}
    }
    let preview = serde_json::to_string_pretty(&root)?;
    if changed && !matches!(action, Action::Preview | Action::Status) {
        crate::product::write_atomic(&path, preview.as_bytes())?;
    }
    Ok(IntegrationResult {
        changed,
        installed: match action {
            Action::Install => true,
            Action::Uninstall => false,
            _ => installed,
        },
        path,
        preview,
    })
}

fn cursor_lifecycle(path: PathBuf, action: Action) -> Result<IntegrationResult> {
    let mut root = read_json_object(&path, json!({ "version": 1 }))?;
    if root.get("version").is_none() {
        root.as_object_mut()
            .context("Cursor hooks root must be an object")?
            .insert("version".into(), json!(1));
    }
    let installed = json_hook_entries(&root, "preToolUse").is_some_and(|entries| {
        entries
            .iter()
            .any(|value| contains_command(value, CURSOR_COMMAND))
    });
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed => {
            json_hook_entries_mut(&mut root, "preToolUse")?.push(cursor_entry());
            changed = true;
        }
        Action::Uninstall if installed => {
            json_hook_entries_mut(&mut root, "preToolUse")?
                .retain(|value| !contains_command(value, CURSOR_COMMAND));
            changed = true;
        }
        _ => {}
    }
    let preview = serde_json::to_string_pretty(&root)?;
    if changed && !matches!(action, Action::Preview | Action::Status) {
        crate::product::write_atomic(&path, preview.as_bytes())?;
    }
    Ok(IntegrationResult {
        changed,
        installed: match action {
            Action::Install => true,
            Action::Uninstall => false,
            _ => installed,
        },
        path,
        preview,
    })
}

fn codex_lifecycle(path: PathBuf, action: Action) -> Result<IntegrationResult> {
    let original = fs::read_to_string(&path).unwrap_or_default();
    let installed = managed_range(&original).is_some();
    let mut next = original.clone();
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed => {
            if !next.is_empty() && !next.ends_with('\n') {
                next.push('\n');
            }
            if !next.is_empty() {
                next.push('\n');
            }
            next.push_str(CODEX_BLOCK);
            next.push('\n');
            changed = true;
        }
        Action::Uninstall if installed => {
            let (start, end) = managed_range(&next).expect("installed range");
            next.replace_range(start..end, "");
            next = next.trim_end().to_string();
            if !next.is_empty() {
                next.push('\n');
            }
            changed = true;
        }
        _ => {}
    }
    if changed && !matches!(action, Action::Preview | Action::Status) {
        crate::product::write_atomic(&path, next.as_bytes())?;
    }
    Ok(IntegrationResult {
        changed,
        installed: match action {
            Action::Install => true,
            Action::Uninstall => false,
            _ => installed,
        },
        path,
        preview: next,
    })
}

fn managed_range(content: &str) -> Option<(usize, usize)> {
    let start = content.find(CODEX_START)?;
    let relative_end = content[start..].find(CODEX_END)?;
    let mut end = start + relative_end + CODEX_END.len();
    if content.as_bytes().get(end) == Some(&b'\n') {
        end += 1;
    }
    Some((start, end))
}

fn read_json_object(path: &Path, default: Value) -> Result<Value> {
    if !path.exists() {
        return Ok(default);
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(default);
    }
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if !value.is_object() {
        anyhow::bail!(
            "integration file root must be a JSON object: {}",
            path.display()
        );
    }
    Ok(value)
}

fn json_hook_entries<'a>(root: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    root.get("hooks")?.get(key)?.as_array()
}

fn json_hook_entries_mut<'a>(root: &'a mut Value, key: &str) -> Result<&'a mut Vec<Value>> {
    let root = root
        .as_object_mut()
        .context("JSON root must be an object")?;
    let hooks = root.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks.as_object_mut().context("hooks must be an object")?;
    hooks
        .entry(key)
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .with_context(|| format!("hooks.{key} must be an array"))
}

fn contains_command(value: &Value, command: &str) -> bool {
    value.get("command").and_then(Value::as_str) == Some(command)
        || value
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|hooks| hooks.iter().any(|hook| contains_command(hook, command)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_lifecycle_is_idempotent_and_preserves_unrelated_settings() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("settings.json"),
            r#"{"theme":"dark","hooks":{"PreToolUse":[{"matcher":"Other"}]}}"#,
        )
        .unwrap();
        let first = run(
            Agent::Claude,
            Action::Install,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert!(first.changed);
        let second = run(
            Agent::Claude,
            Action::Install,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert!(!second.changed);
        let value: Value =
            serde_json::from_str(&fs::read_to_string(temp.path().join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        run(
            Agent::Claude,
            Action::Uninstall,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(temp.path().join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(value["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
        assert_eq!(value["theme"], "dark");
    }

    #[test]
    fn cursor_rejects_unverified_schema_and_preserves_other_hooks() {
        let temp = tempfile::tempdir().unwrap();
        assert!(run(
            Agent::Cursor,
            Action::Install,
            Some(temp.path().into()),
            Some(2)
        )
        .is_err());
        fs::write(
            temp.path().join("hooks.json"),
            r#"{"version":1,"hooks":{"preToolUse":[{"command":"other"}]}}"#,
        )
        .unwrap();
        run(
            Agent::Cursor,
            Action::Install,
            Some(temp.path().into()),
            Some(1),
        )
        .unwrap();
        run(
            Agent::Cursor,
            Action::Uninstall,
            Some(temp.path().into()),
            Some(1),
        )
        .unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(temp.path().join("hooks.json")).unwrap())
                .unwrap();
        assert_eq!(value["hooks"]["preToolUse"][0]["command"], "other");
    }

    #[test]
    fn codex_managed_block_is_idempotent_and_bounded() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# User instructions\n").unwrap();
        run(
            Agent::Codex,
            Action::Install,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        let second = run(
            Agent::Codex,
            Action::Install,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert!(!second.changed);
        let installed = fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
        assert!(installed.contains("does not transparently intercept"));
        run(
            Agent::Codex,
            Action::Uninstall,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("AGENTS.md")).unwrap(),
            "# User instructions\n"
        );
    }

    #[test]
    fn preview_and_status_do_not_write() {
        let temp = tempfile::tempdir().unwrap();
        let preview = run(
            Agent::Claude,
            Action::Preview,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert!(preview.changed);
        assert!(preview.preview.contains(CLAUDE_COMMAND));
        assert!(!preview.path.exists());
        let status = run(
            Agent::Claude,
            Action::Status,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert!(!status.installed);
        assert!(!status.path.exists());
    }
}
