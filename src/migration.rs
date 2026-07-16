//! Explicit, conservative RTK-to-ContextDroid migration.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MigrationOptions {
    pub source_dir: PathBuf,
    pub destination_dir: PathBuf,
    pub apply: bool,
    /// Claude integration root; defaults to `~/.claude` for the CLI.
    pub claude_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MigrationReport {
    pub config_found: bool,
    pub config_would_change: bool,
    pub legacy_analytics_found: bool,
    pub legacy_analytics_rows: u64,
    pub applied: bool,
    pub skipped_sensitive_sections: Vec<String>,
    pub claude_hooks: crate::integrations::RtkHookMigrationReport,
}

pub fn migrate_rtk(options: &MigrationOptions) -> Result<MigrationReport> {
    let source_config = options.source_dir.join("config.toml");
    let source_db = options.source_dir.join("tracking.db");
    let destination_config = options.destination_dir.join("config.toml");
    let destination_db = options.destination_dir.join(crate::product::ANALYTICS_DB);
    let mut report = MigrationReport {
        config_found: source_config.is_file(),
        legacy_analytics_found: source_db.is_file(),
        skipped_sensitive_sections: vec![
            "hooks".into(),
            "telemetry".into(),
            "trusted_filters".into(),
        ],
        ..MigrationReport::default()
    };

    let claude_root = options
        .claude_root
        .clone()
        .or_else(|| dirs::home_dir().map(|home| home.join(".claude")));
    if let Some(root) = claude_root {
        report.claude_hooks = crate::integrations::migrate_claude_rtk_hooks(&root, options.apply)?;
    }

    if source_config.is_file() {
        let source_text = fs::read_to_string(&source_config)?;
        let source: toml::Value = toml::from_str(&source_text)
            .with_context(|| format!("invalid RTK config: {}", source_config.display()))?;
        let destination = if destination_config.is_file() {
            toml::from_str(&fs::read_to_string(&destination_config)?)?
        } else {
            toml::Value::Table(Default::default())
        };
        let migrated = migrate_config_value(source, destination)?;
        let serialized = toml::to_string_pretty(&migrated)?;
        let current = fs::read_to_string(&destination_config).unwrap_or_default();
        report.config_would_change = serialized != current;
        if options.apply && report.config_would_change {
            crate::product::write_atomic(&destination_config, serialized.as_bytes())?;
        }
    }

    if source_db.is_file() {
        report.legacy_analytics_rows = count_legacy_rows(&source_db)?;
        if options.apply {
            import_legacy_analytics(&source_db, &destination_db)?;
        }
    }
    report.applied = options.apply;
    Ok(report)
}

fn migrate_config_value(source: toml::Value, mut destination: toml::Value) -> Result<toml::Value> {
    let source = source
        .as_table()
        .context("RTK config root must be a TOML table")?;
    let destination_table = destination
        .as_table_mut()
        .context("ContextDroid config root must be a TOML table")?;

    for section in ["display", "limits"] {
        if let Some(value) = source.get(section) {
            destination_table.insert(section.into(), value.clone());
        }
    }
    if let Some(source_tracking) = source.get("tracking").and_then(toml::Value::as_table) {
        let tracking = destination_table
            .entry("tracking")
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .context("destination tracking section must be a table")?;
        for key in ["enabled", "history_days"] {
            if let Some(value) = source_tracking.get(key) {
                tracking.insert(key.into(), value.clone());
            }
        }
    }
    Ok(destination)
}

fn count_legacy_rows(source: &Path) -> Result<u64> {
    let conn = Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='commands')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(0);
    }
    conn.query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
        .map_err(Into::into)
}

fn import_legacy_analytics(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let source_conn =
        Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut destination_conn = Connection::open(destination)?;
    destination_conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS legacy_rtk_commands (
           legacy_id INTEGER PRIMARY KEY,
           timestamp TEXT NOT NULL,
           original_cmd TEXT NOT NULL,
           rtk_cmd TEXT NOT NULL,
           input_tokens INTEGER NOT NULL,
           output_tokens INTEGER NOT NULL,
           saved_tokens INTEGER NOT NULL,
           savings_pct REAL NOT NULL
         );",
    )?;
    let exists: bool = source_conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='commands')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let transaction = destination_conn.transaction()?;
    let mut source_rows = source_conn.prepare(
        "SELECT id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens,
                saved_tokens, savings_pct FROM commands",
    )?;
    let rows = source_rows.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, f64>(7)?,
        ))
    })?;
    for row in rows {
        let (id, timestamp, original, rtk, input, output, saved, percent) = row?;
        transaction.execute(
            "INSERT OR IGNORE INTO legacy_rtk_commands
             (legacy_id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens,
              saved_tokens, savings_pct)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, timestamp, original, rtk, input, output, saved, percent],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn default_source_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("cannot determine local data directory")?
        .join("rtk"))
}

pub fn default_destination_dir() -> Result<PathBuf> {
    crate::product::data_dir().context("cannot determine ContextDroid data directory")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_legacy_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE commands (
               id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, original_cmd TEXT NOT NULL,
               rtk_cmd TEXT NOT NULL, input_tokens INTEGER NOT NULL,
               output_tokens INTEGER NOT NULL, saved_tokens INTEGER NOT NULL,
               savings_pct REAL NOT NULL
             );
             INSERT INTO commands VALUES
               (1, '2026-01-01T00:00:00Z', 'git status', 'rtk git status', 100, 25, 75, 75.0);",
        )
        .unwrap();
    }

    #[test]
    fn dry_run_reports_without_writing() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("config.toml"),
            "[display]\ncolors=false\n[hooks]\nexclude_commands=['x']\n[telemetry]\nenabled=true\n",
        )
        .unwrap();
        make_legacy_db(&source.path().join("tracking.db"));
        let report = migrate_rtk(&MigrationOptions {
            source_dir: source.path().into(),
            destination_dir: destination.path().into(),
            apply: false,
            claude_root: Some(destination.path().join(".claude")),
        })
        .unwrap();
        assert!(report.config_would_change);
        assert_eq!(report.legacy_analytics_rows, 1);
        assert!(!destination.path().join("config.toml").exists());
        assert!(!destination
            .path()
            .join(crate::product::ANALYTICS_DB)
            .exists());
    }

    #[test]
    fn apply_copies_only_safe_preferences_and_archives_legacy_analytics() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("config.toml"),
            "[tracking]\nenabled=true\nhistory_days=30\ndatabase_path='secret'\n[display]\ncolors=false\nemoji=false\nmax_width=90\n[hooks]\nexclude_commands=['curl']\n[telemetry]\nenabled=true\n",
        )
        .unwrap();
        make_legacy_db(&source.path().join("tracking.db"));
        let report = migrate_rtk(&MigrationOptions {
            source_dir: source.path().into(),
            destination_dir: destination.path().into(),
            apply: true,
            claude_root: Some(destination.path().join(".claude")),
        })
        .unwrap();
        assert!(report.applied);
        let config = fs::read_to_string(destination.path().join("config.toml")).unwrap();
        assert!(config.contains("history_days = 30"));
        assert!(config.contains("colors = false"));
        assert!(!config.contains("hooks"));
        assert!(!config.contains("telemetry"));
        assert!(!config.contains("database_path"));
        let conn = Connection::open(destination.path().join(crate::product::ANALYTICS_DB)).unwrap();
        let count: u64 = conn
            .query_row("SELECT COUNT(*) FROM legacy_rtk_commands", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
