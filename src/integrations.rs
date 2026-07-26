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
    pub rtk_conflicts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupAction {
    Detect,
    Preview,
    Apply,
    Status,
    Uninstall,
}

#[derive(Debug, Clone)]
pub struct SetupOptions {
    pub action: SetupAction,
    pub only: Vec<Agent>,
    pub include_experimental: bool,
    pub project_root: PathBuf,
    pub confirmed: bool,
    /// Test-only root for home-scoped integration directories.
    pub root_override: Option<PathBuf>,
    #[cfg(test)]
    pub fail_after_writes: Option<usize>,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            action: SetupAction::Detect,
            only: Vec::new(),
            include_experimental: false,
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            confirmed: false,
            root_override: None,
            #[cfg(test)]
            fail_after_writes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupAdapter {
    pub agent: Agent,
    pub tier: &'static str,
    pub detected: bool,
    pub selected: bool,
    pub installed: bool,
    pub changed: bool,
    pub path: PathBuf,
    pub preview: String,
    pub rtk_conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    pub action: SetupAction,
    pub adapters: Vec<SetupAdapter>,
    pub changed: usize,
}

fn setup_root(agent: Agent, options: &SetupOptions) -> Result<PathBuf> {
    if let Some(base) = &options.root_override {
        return Ok(match agent {
            Agent::Claude => base.join(".claude"),
            Agent::Cursor => base.join(".cursor"),
            Agent::Codex => options.project_root.clone(),
        });
    }
    if agent == Agent::Codex {
        return Ok(options.project_root.clone());
    }
    default_root(agent)
}

fn setup_agents(options: &SetupOptions) -> Result<Vec<Agent>> {
    let agents = if options.only.is_empty() {
        let mut defaults = vec![Agent::Claude, Agent::Codex];
        if options.include_experimental {
            defaults.push(Agent::Cursor);
        }
        defaults
    } else {
        options.only.clone()
    };
    if agents.contains(&Agent::Cursor) && !options.include_experimental {
        anyhow::bail!("Cursor setup is experimental; pass --include-experimental to select it");
    }
    let mut unique = Vec::new();
    for agent in agents {
        if !unique.contains(&agent) {
            unique.push(agent);
        }
    }
    Ok(unique)
}

fn integration_path(agent: Agent, root: &Path) -> PathBuf {
    match agent {
        Agent::Claude => root.join("settings.json"),
        Agent::Cursor => root.join("hooks.json"),
        Agent::Codex => root.join("AGENTS.md"),
    }
}

fn restore_setup_files(snapshots: &[(PathBuf, Option<Vec<u8>>)]) -> Result<()> {
    for (path, original) in snapshots.iter().rev() {
        match original {
            Some(bytes) => crate::product::write_atomic(path, bytes)?,
            None if path.exists() => fs::remove_file(path)
                .with_context(|| format!("failed to roll back {}", path.display()))?,
            None => {}
        }
    }
    Ok(())
}

pub fn setup(options: SetupOptions) -> Result<SetupReport> {
    if matches!(options.action, SetupAction::Apply | SetupAction::Uninstall) && !options.confirmed {
        anyhow::bail!("setup changes require confirmation; review preview and pass --yes");
    }
    let agents = setup_agents(&options)?;
    let lifecycle_action = match options.action {
        SetupAction::Detect | SetupAction::Status => Action::Status,
        SetupAction::Preview | SetupAction::Apply => Action::Preview,
        SetupAction::Uninstall => Action::Status,
    };

    // Preflight every selected adapter before the first write.
    let mut adapters = Vec::new();
    for agent in agents {
        let root = setup_root(agent, &options)?;
        let path = integration_path(agent, &root);
        if path.exists() && !path.is_file() {
            anyhow::bail!("integration path is not a regular file: {}", path.display());
        }
        let detected = root.exists() || path.exists() || agent == Agent::Codex;
        let result = run(agent, lifecycle_action, Some(root), None)?;
        if options.action == SetupAction::Apply && result.rtk_conflicts > 0 {
            anyhow::bail!(
                "recognized RTK integration conflict in {}; migrate it explicitly before setup",
                result.path.display()
            );
        }
        adapters.push(SetupAdapter {
            agent,
            tier: if agent == Agent::Cursor {
                "experimental"
            } else if agent == Agent::Codex {
                "supported-guidance"
            } else {
                "supported"
            },
            detected,
            selected: true,
            installed: result.installed,
            changed: false,
            path: result.path,
            preview: result.preview,
            rtk_conflicts: result.rtk_conflicts,
        });
    }

    if matches!(
        options.action,
        SetupAction::Detect | SetupAction::Preview | SetupAction::Status
    ) {
        return Ok(SetupReport {
            action: options.action,
            adapters,
            changed: 0,
        });
    }

    let snapshots: Vec<(PathBuf, Option<Vec<u8>>)> = adapters
        .iter()
        .map(|adapter| {
            let original = fs::read(&adapter.path).ok();
            (adapter.path.clone(), original)
        })
        .collect();
    for (path, original) in &snapshots {
        if let Some(bytes) = original {
            let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.fZ");
            let backup = path.with_extension(format!("contextdroid-backup-{timestamp}"));
            fs::write(&backup, bytes)
                .with_context(|| format!("failed to create backup {}", backup.display()))?;
        }
    }

    let action = if options.action == SetupAction::Apply {
        Action::Install
    } else {
        Action::Uninstall
    };
    let mut changed = 0;
    #[cfg(test)]
    let mut writes_completed = 0;
    for adapter in &mut adapters {
        let root = adapter
            .path
            .parent()
            .context("integration path has no parent")?
            .to_path_buf();
        match run(adapter.agent, action, Some(root), None) {
            Ok(result) => {
                adapter.installed = result.installed;
                adapter.changed = result.changed;
                adapter.preview = result.preview;
                if result.changed {
                    changed += 1;
                }
                #[cfg(test)]
                {
                    writes_completed += 1;
                    if options.fail_after_writes == Some(writes_completed) {
                        restore_setup_files(&snapshots)?;
                        anyhow::bail!(
                            "simulated setup write failure after {writes_completed} adapter"
                        );
                    }
                }
            }
            Err(error) => {
                restore_setup_files(&snapshots)
                    .with_context(|| format!("setup failed ({error}); rollback also failed"))?;
                return Err(error.context("setup failed; all selected adapters were rolled back"));
            }
        }
    }

    Ok(SetupReport {
        action: options.action,
        adapters,
        changed,
    })
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
    let rtk_conflicts = json_hook_entries(&root, hook_key).map_or(0, |entries| {
        entries
            .iter()
            .filter(|value| contains_rtk_hook(value))
            .count()
    });
    if action == Action::Install && rtk_conflicts > 0 {
        anyhow::bail!(
            "recognized RTK hook conflict in {}; run `contextdroid migrate rtk --apply` to back up and replace it",
            path.display()
        );
    }
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed && rtk_conflicts == 0 => {
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
        rtk_conflicts,
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
    let rtk_conflicts = json_hook_entries(&root, "preToolUse").map_or(0, |entries| {
        entries
            .iter()
            .filter(|value| contains_rtk_hook(value))
            .count()
    });
    if action == Action::Install && rtk_conflicts > 0 {
        anyhow::bail!(
            "recognized RTK hook conflict in {}; run `contextdroid migrate rtk --apply` to back up and replace it",
            path.display()
        );
    }
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed && rtk_conflicts == 0 => {
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
        rtk_conflicts,
    })
}

fn codex_lifecycle(path: PathBuf, action: Action) -> Result<IntegrationResult> {
    let original = fs::read_to_string(&path).unwrap_or_default();
    let installed = managed_range(&original).is_some();
    let rtk_conflicts = usize::from(original.contains("<!-- rtk-managed:start"));
    if action == Action::Install && rtk_conflicts > 0 {
        anyhow::bail!(
            "recognized RTK managed block in {}; run explicit RTK migration first",
            path.display()
        );
    }
    let mut next = original.clone();
    let mut changed = false;
    match action {
        Action::Install | Action::Preview if !installed && rtk_conflicts == 0 => {
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
        rtk_conflicts,
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

fn contains_rtk_hook(value: &Value) -> bool {
    value
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| {
            matches!(command.trim(), "rtk hook claude" | "rtk hook cursor")
                || command.ends_with("/rtk-rewrite.sh")
                || command.ends_with("\\rtk-rewrite.ps1")
        })
        || value
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|hooks| hooks.iter().any(contains_rtk_hook))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct RtkHookMigrationReport {
    pub conflicts: usize,
    pub contextdroid_entries: usize,
    pub changed: bool,
    pub backup: Option<PathBuf>,
}

pub fn migrate_claude_rtk_hooks(root: &Path, apply: bool) -> Result<RtkHookMigrationReport> {
    let path = root.join("settings.json");
    let mut value = read_json_object(&path, json!({}))?;
    let entries = json_hook_entries_mut(&mut value, "PreToolUse")?;
    let conflicts = entries
        .iter()
        .filter(|entry| contains_rtk_hook(entry))
        .count();
    let contextdroid_entries = entries
        .iter()
        .filter(|entry| contains_command(entry, CLAUDE_COMMAND))
        .count();
    let changed = conflicts > 0 || contextdroid_entries != 1;
    let mut seen_contextdroid = false;
    entries.retain(|entry| {
        if contains_rtk_hook(entry) {
            return false;
        }
        if contains_command(entry, CLAUDE_COMMAND) {
            if seen_contextdroid {
                return false;
            }
            seen_contextdroid = true;
        }
        true
    });
    if !seen_contextdroid {
        entries.push(claude_entry());
    }
    let mut backup = None;
    if apply && changed {
        if path.exists() {
            let backup_path = path.with_extension(format!(
                "json.contextdroid-backup-{}",
                chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
            ));
            fs::copy(&path, &backup_path).with_context(|| {
                format!(
                    "failed to retain integration backup {}",
                    backup_path.display()
                )
            })?;
            backup = Some(backup_path);
        }
        crate::product::write_atomic(&path, serde_json::to_string_pretty(&value)?.as_bytes())?;
    }
    Ok(RtkHookMigrationReport {
        conflicts,
        contextdroid_entries,
        changed,
        backup,
    })
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

    #[test]
    fn install_fails_closed_on_rtk_conflict_and_migration_backs_up_and_replaces() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(
            &path,
            r#"{"theme":"dark","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"rtk hook claude"}]},{"command":"unrelated"}]}}"#,
        )
        .unwrap();

        let status = run(
            Agent::Claude,
            Action::Status,
            Some(temp.path().into()),
            None,
        )
        .unwrap();
        assert_eq!(status.rtk_conflicts, 1);
        assert!(run(
            Agent::Claude,
            Action::Install,
            Some(temp.path().into()),
            None
        )
        .is_err());

        let report = migrate_claude_rtk_hooks(temp.path(), true).unwrap();
        assert_eq!(report.conflicts, 1);
        assert!(report
            .backup
            .as_ref()
            .is_some_and(|backup| backup.is_file()));
        let migrated = fs::read_to_string(&path).unwrap();
        assert!(migrated.contains(CLAUDE_COMMAND));
        assert!(migrated.contains("unrelated"));
        assert!(migrated.contains("dark"));
        assert!(!migrated.contains("rtk hook claude"));

        let second = migrate_claude_rtk_hooks(temp.path(), true).unwrap();
        assert!(!second.changed);
    }

    #[test]
    fn setup_detect_is_read_only_and_excludes_experimental_cursor_by_default() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".claude")).unwrap();
        fs::create_dir(temp.path().join(".cursor")).unwrap();

        let report = setup(SetupOptions {
            action: SetupAction::Detect,
            project_root: temp.path().to_path_buf(),
            root_override: Some(temp.path().to_path_buf()),
            ..SetupOptions::default()
        })
        .unwrap();

        assert!(report
            .adapters
            .iter()
            .any(|item| item.agent == Agent::Claude));
        assert!(report
            .adapters
            .iter()
            .any(|item| item.agent == Agent::Codex));
        assert!(!report
            .adapters
            .iter()
            .any(|item| item.agent == Agent::Cursor));
        assert!(!temp.path().join(".claude/settings.json").exists());
        assert!(!temp.path().join("AGENTS.md").exists());
    }

    #[test]
    fn setup_apply_is_idempotent_and_uninstall_removes_only_managed_entries() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".claude")).unwrap();
        fs::write(
            temp.path().join(".claude/settings.json"),
            r#"{"theme":"dark","hooks":{"PreToolUse":[{"command":"other"}]}}"#,
        )
        .unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# Team rules\n").unwrap();
        let options = SetupOptions {
            action: SetupAction::Apply,
            only: vec![Agent::Claude, Agent::Codex],
            project_root: temp.path().to_path_buf(),
            root_override: Some(temp.path().to_path_buf()),
            confirmed: true,
            ..SetupOptions::default()
        };

        let first = setup(options.clone()).unwrap();
        assert_eq!(first.changed, 2);
        let second = setup(options).unwrap();
        assert_eq!(second.changed, 0);

        setup(SetupOptions {
            action: SetupAction::Uninstall,
            only: vec![Agent::Claude, Agent::Codex],
            project_root: temp.path().to_path_buf(),
            root_override: Some(temp.path().to_path_buf()),
            confirmed: true,
            ..SetupOptions::default()
        })
        .unwrap();
        let settings = fs::read_to_string(temp.path().join(".claude/settings.json")).unwrap();
        assert!(settings.contains(r#""theme": "dark""#));
        assert!(settings.contains(r#""command": "other""#));
        assert!(!settings.contains(CLAUDE_COMMAND));
        assert_eq!(
            fs::read_to_string(temp.path().join("AGENTS.md")).unwrap(),
            "# Team rules\n"
        );
    }

    #[test]
    fn setup_preflight_conflict_prevents_partial_application() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".claude")).unwrap();
        fs::write(
            temp.path().join(".claude/settings.json"),
            r#"{"hooks":{"PreToolUse":[{"command":"rtk hook claude"}]}}"#,
        )
        .unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# Existing\n").unwrap();

        let result = setup(SetupOptions {
            action: SetupAction::Apply,
            only: vec![Agent::Codex, Agent::Claude],
            project_root: temp.path().to_path_buf(),
            root_override: Some(temp.path().to_path_buf()),
            confirmed: true,
            ..SetupOptions::default()
        });

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("AGENTS.md")).unwrap(),
            "# Existing\n"
        );
    }

    #[test]
    fn setup_rolls_back_an_earlier_adapter_when_a_later_write_fails() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".claude")).unwrap();
        let settings = temp.path().join(".claude/settings.json");
        fs::write(&settings, r#"{"theme":"dark"}"#).unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# Team rules\n").unwrap();

        let result = setup(SetupOptions {
            action: SetupAction::Apply,
            only: vec![Agent::Claude, Agent::Codex],
            project_root: temp.path().to_path_buf(),
            root_override: Some(temp.path().to_path_buf()),
            confirmed: true,
            fail_after_writes: Some(1),
            ..SetupOptions::default()
        });

        assert!(result.is_err());
        let restored = fs::read_to_string(settings).unwrap();
        assert_eq!(restored, r#"{"theme":"dark"}"#);
        assert!(!restored.contains(CLAUDE_COMMAND));
        assert_eq!(
            fs::read_to_string(temp.path().join("AGENTS.md")).unwrap(),
            "# Team rules\n"
        );
    }
}
