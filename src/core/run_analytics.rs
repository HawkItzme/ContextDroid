//! Privacy-conscious, canonical local analytics for every ContextDroid execution.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::run_store::RunMetadata;
use super::time_window::{LastCount, PositiveDuration};

const SCHEMA_VERSION: i64 = 2;
static EXECUTION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub struct RunAnalytics {
    conn: Connection,
}

#[derive(Debug, Clone, Default)]
pub struct RunQuery {
    pub scope: Option<String>,
    pub command_family: Option<String>,
    pub project: Option<String>,
    pub profile: Option<String>,
    pub parser: Option<String>,
    pub since: Option<PositiveDuration>,
    /// Compatibility for internal callers while CLI duration migration lands.
    pub since_days: Option<i64>,
    pub last: Option<LastCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRecord {
    pub execution_key: String,
    pub record_origin: String,
    pub started_at_ms: i64,
    pub session_id: Option<String>,
    pub scope: String,
    pub command_family: String,
    pub operation: String,
    pub parser: Option<String>,
    pub profile: String,
    pub output_mode: String,
    pub project: Option<String>,
    pub execution_source: String,
    pub raw_bytes: Option<u64>,
    pub returned_bytes: Option<u64>,
    pub raw_lines: Option<u64>,
    pub returned_lines: Option<u64>,
    pub raw_tokens_estimate: Option<u64>,
    pub returned_tokens_estimate: Option<u64>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub confidence: Option<String>,
    pub raw_fallback: bool,
    pub never_worse_fallback: bool,
    pub recovery_requested: bool,
    pub parser_error: bool,
    pub detectable_rerun: bool,
    pub exit_code_parity: Option<bool>,
    pub fixture_preservation: Option<bool>,
    pub run_id: Option<String>,
    pub omission_preserved: u64,
    pub omission_collapsed: u64,
}

pub struct ExecutionRecordBuilder(ExecutionRecord);

impl ExecutionRecordBuilder {
    pub fn new(
        execution_key: impl Into<String>,
        scope: impl Into<String>,
        command_family: impl Into<String>,
    ) -> Self {
        Self(ExecutionRecord {
            execution_key: execution_key.into(),
            record_origin: "native".into(),
            started_at_ms: Utc::now().timestamp_millis(),
            session_id: None,
            scope: scope.into(),
            command_family: command_family.into(),
            operation: "other".into(),
            parser: None,
            profile: crate::product::DEFAULT_PROFILE.into(),
            output_mode: "balanced".into(),
            project: None,
            execution_source: "direct-cli".into(),
            raw_bytes: None,
            returned_bytes: None,
            raw_lines: None,
            returned_lines: None,
            raw_tokens_estimate: None,
            returned_tokens_estimate: None,
            duration_ms: None,
            exit_code: None,
            signal: None,
            confidence: None,
            raw_fallback: false,
            never_worse_fallback: false,
            recovery_requested: false,
            parser_error: false,
            detectable_rerun: false,
            exit_code_parity: None,
            fixture_preservation: None,
            run_id: None,
            omission_preserved: 0,
            omission_collapsed: 0,
        })
    }

    pub fn operation(mut self, value: impl Into<String>) -> Self {
        self.0.operation = value.into();
        self
    }

    pub fn origin(mut self, value: impl Into<String>) -> Self {
        self.0.record_origin = value.into();
        self
    }

    pub fn started_at_ms(mut self, value: i64) -> Self {
        self.0.started_at_ms = value;
        self
    }

    pub fn project(mut self, value: Option<String>) -> Self {
        self.0.project = value;
        self
    }

    pub fn parser(mut self, value: Option<String>) -> Self {
        self.0.parser = value;
        self
    }

    pub fn profile(mut self, value: impl Into<String>) -> Self {
        self.0.profile = value.into();
        self
    }

    pub fn output_mode(mut self, value: impl Into<String>) -> Self {
        self.0.output_mode = value.into();
        self
    }

    pub fn metrics(
        mut self,
        raw_bytes: Option<u64>,
        returned_bytes: Option<u64>,
        raw_tokens: Option<u64>,
        returned_tokens: Option<u64>,
    ) -> Self {
        self.0.raw_bytes = raw_bytes;
        self.0.returned_bytes = returned_bytes;
        self.0.raw_tokens_estimate = raw_tokens;
        self.0.returned_tokens_estimate = returned_tokens;
        self
    }

    pub fn duration_ms(mut self, value: Option<u64>) -> Self {
        self.0.duration_ms = value;
        self
    }

    pub fn build(self) -> Result<ExecutionRecord> {
        validate_label("scope", &self.0.scope)?;
        validate_label("command family", &self.0.command_family)?;
        validate_label("operation", &self.0.operation)?;
        if self.0.execution_key.is_empty() || self.0.execution_key.len() > 200 {
            bail!("invalid analytics execution key");
        }
        Ok(self.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GainReport {
    pub runs: u64,
    pub raw_tokens_estimate: u64,
    pub returned_tokens_estimate: u64,
    pub direct_saved_tokens_estimate: u64,
    pub direct_reduction_percent: f64,
    pub effective_returned_tokens_estimate: u64,
    pub effective_saved_tokens_estimate: u64,
    pub effective_reduction_percent: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QualityReport {
    pub runs: u64,
    pub high_confidence: u64,
    pub medium_confidence: u64,
    pub low_confidence: u64,
    pub fallbacks: u64,
    pub never_worse_fallbacks: u64,
    pub parser_errors: u64,
    pub raw_recoveries: u64,
    pub detectable_reruns: u64,
    pub exit_parity_failures: u64,
    pub exit_parity_unknown: u64,
    pub fixture_preservation_failures: u64,
    pub fixture_preservation_unknown: u64,
    pub fallback_rate_percent: f64,
    pub raw_recovery_rate_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionView {
    pub execution_id: i64,
    pub started_at_ms: i64,
    pub session_id: Option<String>,
    pub scope: String,
    pub command_family: String,
    pub operation: String,
    pub parser: Option<String>,
    pub profile: String,
    pub output_mode: String,
    pub raw_tokens_estimate: Option<u64>,
    pub returned_tokens_estimate: Option<u64>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub confidence: Option<String>,
    pub raw_fallback: bool,
    pub never_worse_fallback: bool,
    pub recovery_requested: bool,
    pub parser_error: bool,
    pub detectable_rerun: bool,
    pub exit_code_parity: Option<bool>,
    pub fixture_preservation: Option<bool>,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub executions: u64,
    pub raw_tokens_estimate: u64,
    pub returned_tokens_estimate: u64,
    pub latest_started_at_ms: i64,
}

impl RunAnalytics {
    pub fn open_default() -> Result<Self> {
        Self::open(&analytics_db_path()?)
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create analytics directory: {}", parent.display())
            })?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open analytics database: {}", path.display()))?;
        let analytics = Self { conn };
        analytics.initialize()?;
        Ok(analytics)
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self> {
        let analytics = Self {
            conn: Connection::open_in_memory()?,
        };
        analytics.initialize()?;
        Ok(analytics)
    }

    fn initialize(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS contextdroid_executions (
               execution_id INTEGER PRIMARY KEY AUTOINCREMENT,
               execution_key TEXT UNIQUE NOT NULL,
               record_origin TEXT NOT NULL,
               started_at_ms INTEGER NOT NULL,
               session_id TEXT,
               scope TEXT NOT NULL,
               command_family TEXT NOT NULL,
               operation TEXT NOT NULL,
               parser TEXT,
               profile TEXT NOT NULL,
               output_mode TEXT NOT NULL,
               project_id TEXT,
               execution_source TEXT NOT NULL,
               raw_bytes INTEGER,
               returned_bytes INTEGER,
               raw_lines INTEGER,
               returned_lines INTEGER,
               raw_tokens_estimate INTEGER,
               returned_tokens_estimate INTEGER,
               duration_ms INTEGER,
               exit_code INTEGER,
               signal INTEGER,
               confidence TEXT,
               raw_fallback INTEGER NOT NULL,
               never_worse_fallback INTEGER NOT NULL,
               recovery_requested INTEGER NOT NULL,
               parser_error INTEGER NOT NULL,
               detectable_rerun INTEGER NOT NULL,
               exit_code_parity INTEGER,
               fixture_preservation INTEGER,
               run_id TEXT,
               omission_preserved INTEGER NOT NULL,
               omission_collapsed INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_executions_started
               ON contextdroid_executions(started_at_ms DESC, execution_id DESC);
             CREATE INDEX IF NOT EXISTS idx_executions_filters
               ON contextdroid_executions(scope, command_family, profile, parser, started_at_ms);
             CREATE INDEX IF NOT EXISTS idx_executions_project
               ON contextdroid_executions(project_id, started_at_ms);
             CREATE TABLE IF NOT EXISTS analytics_meta (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS analytics_migrations (
               name TEXT PRIMARY KEY,
               completed_at_ms INTEGER NOT NULL,
               imported_rows INTEGER NOT NULL,
               skipped_rows INTEGER NOT NULL
             );",
        )?;
        self.conn
            .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        self.import_legacy_rows()?;
        Ok(())
    }

    pub fn record(&self, metadata: &RunMetadata) -> Result<()> {
        let started_at_ms = parse_timestamp_ms(&metadata.started_at)
            .unwrap_or_else(|_| Utc::now().timestamp_millis());
        let mut record = ExecutionRecordBuilder::new(
            format!("run-v1:{}", metadata.run_id.as_str()),
            metadata.scope.clone(),
            metadata.command_family.clone(),
        )
        .origin("contextdroid-v1")
        .started_at_ms(started_at_ms)
        .operation(classify_operation(&metadata.command))
        .project(Some(metadata.project_path.clone()))
        .parser(metadata.parser.clone())
        .profile(metadata.profile.clone())
        .output_mode(metadata.output_mode.clone())
        .metrics(
            Some(metadata.raw_bytes),
            Some(metadata.returned_bytes),
            Some(metadata.raw_tokens_estimate),
            Some(metadata.returned_tokens_estimate),
        )
        .duration_ms(metadata.duration_ms)
        .build()?;
        record.raw_lines = Some(metadata.raw_lines);
        record.returned_lines = Some(metadata.returned_lines);
        record.exit_code = metadata.exit_code;
        record.signal = metadata.signal;
        record.confidence = metadata.confidence.clone();
        record.raw_fallback = metadata.raw_fallback;
        record.never_worse_fallback = metadata.never_worse_fallback;
        record.recovery_requested = metadata.recovery_requested;
        record.parser_error = metadata.parser_error;
        record.detectable_rerun = metadata.detectable_rerun;
        // v1 values were asserted rather than measured; preserve them as unknown.
        record.exit_code_parity = None;
        record.fixture_preservation = None;
        record.run_id = Some(metadata.run_id.as_str().to_string());
        record.omission_preserved = metadata.omission_preserved;
        record.omission_collapsed = metadata.omission_collapsed;
        self.record_execution(&record)
    }

    pub fn record_execution(&self, record: &ExecutionRecord) -> Result<()> {
        let project_id = record
            .project
            .as_deref()
            .map(|path| self.private_id("project", path))
            .transpose()?;
        self.conn.execute(
            "INSERT INTO contextdroid_executions (
               execution_key, record_origin, started_at_ms, session_id, scope,
               command_family, operation, parser, profile, output_mode, project_id,
               execution_source, raw_bytes, returned_bytes, raw_lines, returned_lines,
               raw_tokens_estimate, returned_tokens_estimate, duration_ms, exit_code,
               signal, confidence, raw_fallback, never_worse_fallback, recovery_requested,
               parser_error, detectable_rerun, exit_code_parity, fixture_preservation,
               run_id, omission_preserved, omission_collapsed
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                       ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32)
             ON CONFLICT(execution_key) DO UPDATE SET
               recovery_requested=excluded.recovery_requested,
               returned_bytes=excluded.returned_bytes,
               returned_lines=excluded.returned_lines,
               returned_tokens_estimate=excluded.returned_tokens_estimate",
            params![
                record.execution_key,
                record.record_origin,
                record.started_at_ms,
                record.session_id,
                record.scope,
                record.command_family,
                record.operation,
                record.parser,
                record.profile,
                record.output_mode,
                project_id,
                record.execution_source,
                record.raw_bytes,
                record.returned_bytes,
                record.raw_lines,
                record.returned_lines,
                record.raw_tokens_estimate,
                record.returned_tokens_estimate,
                record.duration_ms,
                record.exit_code,
                record.signal,
                record.confidence,
                record.raw_fallback,
                record.never_worse_fallback,
                record.recovery_requested,
                record.parser_error,
                record.detectable_rerun,
                record.exit_code_parity,
                record.fixture_preservation,
                record.run_id,
                record.omission_preserved,
                record.omission_collapsed,
            ],
        )?;
        Ok(())
    }

    pub fn record_compatibility(
        &self,
        original_command: &str,
        _transformed_command: &str,
        input: &str,
        output: &str,
        duration_ms: u64,
    ) -> Result<()> {
        let family = classify_family(original_command);
        let scope = classify_scope(&family);
        let now = Utc::now().timestamp_millis();
        let sequence = EXECUTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let record = ExecutionRecordBuilder::new(
            format!("native-compat:{now}:{}:{sequence}", std::process::id()),
            scope,
            family,
        )
        .operation(classify_operation(original_command))
        .project(
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
        )
        .metrics(
            Some(input.len() as u64),
            Some(output.len() as u64),
            Some(estimate_tokens(input)),
            Some(estimate_tokens(output)),
        )
        .duration_ms(Some(duration_ms))
        .build()?;
        self.record_execution(&record)
    }

    pub fn record_passthrough(
        &self,
        original_command: &str,
        _transformed_command: &str,
        duration_ms: u64,
    ) -> Result<()> {
        let family = classify_family(original_command);
        let scope = classify_scope(&family);
        let now = Utc::now().timestamp_millis();
        let sequence = EXECUTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let record = ExecutionRecordBuilder::new(
            format!("passthrough:{now}:{}:{sequence}", std::process::id()),
            scope,
            family,
        )
        .operation(classify_operation(original_command))
        .project(
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
        )
        .duration_ms(Some(duration_ms))
        .build()?;
        self.record_execution(&record)
    }

    pub fn mark_recovery(&self, run_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE contextdroid_executions SET recovery_requested=1 WHERE run_id=?1",
            [run_id],
        )?;
        Ok(())
    }

    pub fn executions(&self, query: &RunQuery) -> Result<Vec<ExecutionView>> {
        self.import_legacy_rows()?;
        let cutoff = query_cutoff(query)?;
        let project_id = query
            .project
            .as_deref()
            .map(|path| self.private_id("project", path))
            .transpose()?;
        let limit = query.last.map(|count| count.get() as i64);
        let mut stmt = self.conn.prepare(
            "SELECT execution_id, started_at_ms, session_id, scope, command_family,
                    operation, parser, profile, output_mode, raw_tokens_estimate,
                    returned_tokens_estimate, duration_ms, exit_code, signal, confidence,
                    raw_fallback, never_worse_fallback, recovery_requested, parser_error,
                    detectable_rerun, exit_code_parity, fixture_preservation, run_id
             FROM contextdroid_executions
             WHERE (?1 IS NULL OR scope=?1)
               AND (?2 IS NULL OR command_family=?2)
               AND (?3 IS NULL OR project_id=?3)
               AND (?4 IS NULL OR profile=?4)
               AND (?5 IS NULL OR parser=?5)
               AND (?6 IS NULL OR started_at_ms>=?6)
             ORDER BY started_at_ms DESC, execution_id DESC
             LIMIT COALESCE(?7, -1)",
        )?;
        let rows = stmt.query_map(
            params![
                query.scope.as_deref(),
                query.command_family.as_deref(),
                project_id,
                query.profile.as_deref(),
                query.parser.as_deref(),
                cutoff,
                limit,
            ],
            |row| {
                Ok(ExecutionView {
                    execution_id: row.get(0)?,
                    started_at_ms: row.get(1)?,
                    session_id: row.get(2)?,
                    scope: row.get(3)?,
                    command_family: row.get(4)?,
                    operation: row.get(5)?,
                    parser: row.get(6)?,
                    profile: row.get(7)?,
                    output_mode: row.get(8)?,
                    raw_tokens_estimate: row.get(9)?,
                    returned_tokens_estimate: row.get(10)?,
                    duration_ms: row.get(11)?,
                    exit_code: row.get(12)?,
                    signal: row.get(13)?,
                    confidence: row.get(14)?,
                    raw_fallback: row.get(15)?,
                    never_worse_fallback: row.get(16)?,
                    recovery_requested: row.get(17)?,
                    parser_error: row.get(18)?,
                    detectable_rerun: row.get(19)?,
                    exit_code_parity: row.get(20)?,
                    fixture_preservation: row.get(21)?,
                    run_id: row.get(22)?,
                })
            },
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn gain(&self, query: &RunQuery) -> Result<GainReport> {
        let mut report = GainReport::default();
        for row in self.executions(query)? {
            report.runs += 1;
            let raw = row.raw_tokens_estimate.unwrap_or_default();
            let returned = row.returned_tokens_estimate.unwrap_or_default();
            report.raw_tokens_estimate += raw;
            report.returned_tokens_estimate += returned;
            report.effective_returned_tokens_estimate += returned;
            if row.recovery_requested {
                report.effective_returned_tokens_estimate += raw;
            }
        }
        report.direct_saved_tokens_estimate = report
            .raw_tokens_estimate
            .saturating_sub(report.returned_tokens_estimate);
        report.effective_saved_tokens_estimate = report
            .raw_tokens_estimate
            .saturating_sub(report.effective_returned_tokens_estimate);
        report.direct_reduction_percent = percent(
            report.direct_saved_tokens_estimate,
            report.raw_tokens_estimate,
        );
        report.effective_reduction_percent = percent(
            report.effective_saved_tokens_estimate,
            report.raw_tokens_estimate,
        );
        Ok(report)
    }

    pub fn quality(&self, query: &RunQuery) -> Result<QualityReport> {
        let mut report = QualityReport::default();
        for row in self.executions(query)? {
            report.runs += 1;
            match row.confidence.as_deref() {
                Some("high") => report.high_confidence += 1,
                Some("medium") => report.medium_confidence += 1,
                Some("low") => report.low_confidence += 1,
                _ => {}
            }
            report.fallbacks += u64::from(row.raw_fallback);
            report.never_worse_fallbacks += u64::from(row.never_worse_fallback);
            report.parser_errors += u64::from(row.parser_error);
            report.raw_recoveries += u64::from(row.recovery_requested);
            report.detectable_reruns += u64::from(row.detectable_rerun);
            match row.exit_code_parity {
                Some(false) => report.exit_parity_failures += 1,
                None => report.exit_parity_unknown += 1,
                Some(true) => {}
            }
            match row.fixture_preservation {
                Some(false) => report.fixture_preservation_failures += 1,
                None => report.fixture_preservation_unknown += 1,
                Some(true) => {}
            }
        }
        report.fallback_rate_percent = percent(report.fallbacks, report.runs);
        report.raw_recovery_rate_percent = percent(report.raw_recoveries, report.runs);
        Ok(report)
    }

    pub fn sessions(&self, limit: LastCount) -> Result<Vec<SessionSummary>> {
        self.import_legacy_rows()?;
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(session_id, 'unattributed'), COUNT(*),
                    COALESCE(SUM(raw_tokens_estimate),0),
                    COALESCE(SUM(returned_tokens_estimate),0), MAX(started_at_ms)
             FROM contextdroid_executions
             GROUP BY COALESCE(session_id, 'unattributed')
             ORDER BY MAX(started_at_ms) DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.get() as i64], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                executions: row.get(1)?,
                raw_tokens_estimate: row.get(2)?,
                returned_tokens_estimate: row.get(3)?,
                latest_started_at_ms: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn schema_version(&self) -> Result<i64> {
        self.conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(Into::into)
    }

    pub fn recovery_for(&self, run_id: &str) -> Result<Option<bool>> {
        self.conn
            .query_row(
                "SELECT recovery_requested FROM contextdroid_executions WHERE run_id=?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn reset(&self) -> Result<()> {
        self.conn.execute_batch(
            "BEGIN;
             DELETE FROM contextdroid_executions;
             INSERT INTO analytics_meta(key,value) VALUES('legacy_import_disabled','1')
               ON CONFLICT(key) DO UPDATE SET value='1';
             COMMIT;",
        )?;
        // Migration checkpoints and this reset boundary intentionally survive reset.
        Ok(())
    }

    fn private_id(&self, namespace: &str, value: &str) -> Result<String> {
        let salt = self.install_salt()?;
        let value = if namespace == "project" {
            normalize_project_path(value)
        } else {
            value.to_string()
        };
        Ok(private_fingerprint(&format!("{namespace}:{salt}:{value}")))
    }

    fn install_salt(&self) -> Result<String> {
        if let Some(value) = self
            .conn
            .query_row(
                "SELECT value FROM analytics_meta WHERE key='install_salt'",
                [],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(value);
        }
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).context("failed to create analytics privacy salt")?;
        let salt = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.conn.execute(
            "INSERT OR IGNORE INTO analytics_meta(key,value) VALUES('install_salt',?1)",
            [&salt],
        )?;
        self.conn
            .query_row(
                "SELECT value FROM analytics_meta WHERE key='install_salt'",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn import_legacy_rows(&self) -> Result<()> {
        let disabled: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM analytics_meta
             WHERE key='legacy_import_disabled' AND value='1')",
            [],
            |row| row.get(0),
        )?;
        if disabled {
            return Ok(());
        }
        let mut imported = 0_i64;
        let mut skipped = 0_i64;
        if table_exists(&self.conn, "contextdroid_runs")? {
            let mut stmt = self.conn.prepare(
                "SELECT run_id, started_at, command, scope, command_family, parser, profile,
                        project_path, raw_bytes, returned_bytes, raw_lines, returned_lines,
                        raw_tokens_estimate, returned_tokens_estimate, duration_ms, exit_code,
                        signal, confidence, raw_fallback, recovery_requested, parser_error,
                        detectable_rerun, omission_preserved, omission_collapsed
                 FROM contextdroid_runs",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, u64>(8)?,
                        row.get::<_, u64>(9)?,
                        row.get::<_, u64>(10)?,
                        row.get::<_, u64>(11)?,
                        row.get::<_, u64>(12)?,
                        row.get::<_, u64>(13)?,
                        row.get::<_, u64>(14)?,
                        row.get::<_, Option<i32>>(15)?,
                        row.get::<_, Option<i32>>(16)?,
                        row.get::<_, Option<String>>(17)?,
                        row.get::<_, bool>(18)?,
                        row.get::<_, bool>(19)?,
                        row.get::<_, bool>(20)?,
                        row.get::<_, bool>(21)?,
                        row.get::<_, u64>(22)?,
                        row.get::<_, u64>(23)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(stmt);
            for row in rows {
                let mut record =
                    ExecutionRecordBuilder::new(format!("run-v1:{}", row.0), row.3, row.4)
                        .origin("contextdroid-v1")
                        .started_at_ms(parse_timestamp_ms(&row.1)?)
                        .operation(classify_operation(&row.2))
                        .project(Some(row.7))
                        .parser(row.5)
                        .profile(row.6)
                        .metrics(Some(row.8), Some(row.9), Some(row.12), Some(row.13))
                        .duration_ms(Some(row.14))
                        .build()?;
                record.raw_lines = Some(row.10);
                record.returned_lines = Some(row.11);
                record.exit_code = row.15;
                record.signal = row.16;
                record.confidence = row.17;
                record.raw_fallback = row.18;
                record.recovery_requested = row.19;
                record.parser_error = row.20;
                record.detectable_rerun = row.21;
                record.run_id = Some(row.0);
                record.omission_preserved = row.22;
                record.omission_collapsed = row.23;
                imported += self.insert_import(&record)?;
            }
        }
        let (count, ignored) = self.import_command_table("commands", "legacy-local")?;
        imported += count;
        skipped += ignored;
        let (count, ignored) = self.import_command_table("legacy_rtk_commands", "legacy-rtk")?;
        imported += count;
        skipped += ignored;
        self.conn.execute(
            "INSERT INTO analytics_migrations(name,completed_at_ms,imported_rows,skipped_rows)
             VALUES('canonical-v2',?1,?2,?3)
             ON CONFLICT(name) DO UPDATE SET completed_at_ms=?1,
               imported_rows=imported_rows+?2, skipped_rows=skipped_rows+?3",
            params![Utc::now().timestamp_millis(), imported, skipped],
        )?;
        Ok(())
    }

    fn insert_import(&self, record: &ExecutionRecord) -> Result<i64> {
        let existed: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM contextdroid_executions WHERE execution_key=?1)",
            [&record.execution_key],
            |row| row.get(0),
        )?;
        self.record_execution(record)?;
        Ok(i64::from(!existed))
    }

    fn import_command_table(&self, table: &str, origin: &str) -> Result<(i64, i64)> {
        if !table_exists(&self.conn, table)? {
            return Ok((0, 0));
        }
        let id_column = if table == "legacy_rtk_commands" {
            "legacy_id"
        } else {
            "id"
        };
        let project_expr = if column_exists(&self.conn, table, "project_path")? {
            "project_path"
        } else {
            "''"
        };
        let duration_expr = if column_exists(&self.conn, table, "exec_time_ms")? {
            "exec_time_ms"
        } else {
            "0"
        };
        let sql = format!(
            "SELECT {id_column},timestamp,original_cmd,rtk_cmd,input_tokens,output_tokens,
                    {duration_expr},{project_expr} FROM {table}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, u64>(4)?,
                    row.get::<_, u64>(5)?,
                    row.get::<_, u64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        let mut imported = 0;
        let mut skipped = 0;
        for (id, timestamp, original, _rewritten, raw_tokens, returned_tokens, duration, project) in
            rows
        {
            let family = classify_family(&original);
            let scope = classify_scope(&family);
            let started_at_ms = parse_timestamp_ms(&timestamp)?;
            if scope == "android"
                && self.has_durable_duplicate(
                    &family,
                    started_at_ms,
                    raw_tokens,
                    returned_tokens,
                )?
            {
                skipped += 1;
                continue;
            }
            let record = ExecutionRecordBuilder::new(format!("{origin}:{id}"), scope, family)
                .origin(origin)
                .started_at_ms(started_at_ms)
                .operation(classify_operation(&original))
                .project((!project.is_empty()).then_some(project))
                .metrics(None, None, Some(raw_tokens), Some(returned_tokens))
                .duration_ms(Some(duration))
                .build()?;
            imported += self.insert_import(&record)?;
        }
        Ok((imported, skipped))
    }

    fn has_durable_duplicate(
        &self,
        family: &str,
        completed_at_ms: i64,
        raw_tokens: u64,
        returned_tokens: u64,
    ) -> Result<bool> {
        self.conn
            .query_row(
                "SELECT EXISTS(
               SELECT 1 FROM contextdroid_executions
               WHERE record_origin='contextdroid-v1' AND scope='android'
                 AND command_family=?1 AND raw_tokens_estimate=?2
                 AND returned_tokens_estimate=?3
                 AND ABS((started_at_ms + COALESCE(duration_ms,0)) - ?4) <= 5000
             )",
                params![family, raw_tokens, returned_tokens, completed_at_ms],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_label(kind: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        bail!("invalid {kind} analytics label");
    }
    Ok(())
}

fn classify_family(command: &str) -> String {
    let lower = command.to_ascii_lowercase();
    if lower.contains("gradlew") || lower.starts_with("gradle ") {
        "gradle".into()
    } else if lower.starts_with("adb ") {
        "adb".into()
    } else {
        lower
            .split_whitespace()
            .next()
            .unwrap_or("other")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(80)
            .collect::<String>()
            .pipe_nonempty()
    }
}

trait NonEmptyLabel {
    fn pipe_nonempty(self) -> String;
}
impl NonEmptyLabel for String {
    fn pipe_nonempty(self) -> String {
        if self.is_empty() {
            "other".into()
        } else {
            self
        }
    }
}

fn classify_scope(family: &str) -> String {
    if matches!(family, "gradle" | "adb" | "logcat") {
        "android".into()
    } else {
        "general".into()
    }
}

fn classify_operation(command: &str) -> String {
    command
        .split_whitespace()
        .skip(1)
        .find(|token| !token.starts_with('-'))
        .unwrap_or("other")
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(80)
        .collect::<String>()
        .pipe_nonempty()
}

fn parse_timestamp_ms(value: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.timestamp_millis())
        .context("invalid analytics timestamp")
}

fn query_cutoff(query: &RunQuery) -> Result<Option<i64>> {
    if let Some(duration) = query.since {
        return Ok(Some(
            Utc::now()
                .timestamp_millis()
                .checked_sub(duration.millis())
                .context("duration cutoff overflow")?,
        ));
    }
    if let Some(days) = query.since_days {
        if days <= 0 {
            bail!("analytics duration must be positive");
        }
        let millis = days
            .checked_mul(PositiveDuration::DAY.millis())
            .context("duration cutoff overflow")?;
        return Ok(Some(
            Utc::now()
                .timestamp_millis()
                .checked_sub(millis)
                .context("duration cutoff overflow")?,
        ));
    }
    Ok(None)
}

fn private_fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn normalize_project_path(value: &str) -> String {
    let resolved = std::fs::canonicalize(value).unwrap_or_else(|_| PathBuf::from(value));
    let normalized = resolved.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        let normalized = normalized
            .strip_prefix(r"\\?\UNC\")
            .map(|path| format!(r"\\{path}"))
            .or_else(|| normalized.strip_prefix(r"\\?\").map(str::to_string))
            .unwrap_or(normalized);
        normalized.replace('/', r"\").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

fn estimate_tokens(value: &str) -> u64 {
    value.len().div_ceil(4) as u64
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 * 100.0 / total as f64
    }
}

pub fn analytics_db_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CONTEXTDROID_ANALYTICS_DB") {
        return Ok(PathBuf::from(path));
    }
    Ok(crate::product::data_dir()
        .context("cannot determine ContextDroid data directory")?
        .join(crate::product::ANALYTICS_DB))
}

pub fn record_silent(metadata: &RunMetadata) {
    if let Ok(analytics) = RunAnalytics::open_default() {
        let _ = analytics.record(metadata);
    }
}

pub fn record_compatibility_silent(
    original_command: &str,
    transformed_command: &str,
    input: &str,
    output: &str,
    duration_ms: u64,
) {
    if let Ok(analytics) = RunAnalytics::open_default() {
        let _ = analytics.record_compatibility(
            original_command,
            transformed_command,
            input,
            output,
            duration_ms,
        );
    }
}

pub fn record_passthrough_silent(
    original_command: &str,
    transformed_command: &str,
    duration_ms: u64,
) {
    if let Ok(analytics) = RunAnalytics::open_default() {
        let _ = analytics.record_passthrough(original_command, transformed_command, duration_ms);
    }
}

pub fn mark_recovery_silent(run_id: &str) {
    if let Ok(analytics) = RunAnalytics::open_default() {
        let _ = analytics.mark_recovery(run_id);
    }
}

pub fn parse_since(value: &str) -> Result<PositiveDuration> {
    value.parse()
}

pub fn validate_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .context("invalid analytics timestamp")
}

#[derive(Serialize)]
struct GainOutput<'a> {
    query: QueryOutput<'a>,
    summary: &'a GainReport,
    executions: &'a [ExecutionView],
}

#[derive(Serialize)]
struct QueryOutput<'a> {
    scope: &'a Option<String>,
    command: &'a Option<String>,
    project: &'a Option<String>,
    profile: &'a Option<String>,
    parser: &'a Option<String>,
    last: Option<usize>,
}

pub fn run_gain_cli(query: RunQuery, format: &str) -> Result<()> {
    let analytics = RunAnalytics::open_default()?;
    let executions = analytics.executions(&query)?;
    let report = analytics.gain(&query)?;
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&GainOutput {
                query: QueryOutput {
                    scope: &query.scope,
                    command: &query.command_family,
                    project: &query.project,
                    profile: &query.profile,
                    parser: &query.parser,
                    last: query.last.map(LastCount::get)
                },
                summary: &report,
                executions: &executions,
            })?
        );
        return Ok(());
    }
    if format == "csv" {
        println!("started_at_ms,scope,command,operation,parser,profile,raw_tokens_estimate,returned_tokens_estimate,duration_ms,exit_code");
        for row in executions {
            println!(
                "{},{},{},{},{},{},{},{},{},{}",
                row.started_at_ms,
                row.scope,
                row.command_family,
                row.operation,
                row.parser.unwrap_or_default(),
                row.profile,
                row.raw_tokens_estimate.unwrap_or_default(),
                row.returned_tokens_estimate.unwrap_or_default(),
                row.duration_ms.unwrap_or_default(),
                row.exit_code.map(|v| v.to_string()).unwrap_or_default()
            );
        }
        return Ok(());
    }
    if format != "text" {
        bail!("gain format must be text, json, or csv");
    }
    println!("ContextDroid output reduction (estimated tokens)");
    println!("Executions: {}", report.runs);
    println!("Raw: {}", report.raw_tokens_estimate);
    println!("Returned: {}", report.returned_tokens_estimate);
    println!(
        "Direct estimated reduction: {} ({:.1}%)",
        report.direct_saved_tokens_estimate, report.direct_reduction_percent
    );
    println!(
        "Effective estimated reduction after raw recoveries: {} ({:.1}%)",
        report.effective_saved_tokens_estimate, report.effective_reduction_percent
    );
    println!("These are command-output estimates, not complete model-session billing savings.");
    if query.last.is_some() {
        println!("Recent matching executions:");
        for row in executions {
            println!(
                "- {} {} {} parser={} profile={} exit={}",
                row.started_at_ms,
                row.scope,
                row.command_family,
                row.parser.as_deref().unwrap_or("unknown"),
                row.profile,
                row.exit_code
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into())
            );
        }
    }
    Ok(())
}

pub fn run_graph_cli(query: &RunQuery) -> Result<()> {
    let rows = RunAnalytics::open_default()?.executions(query)?;
    let mut daily = std::collections::BTreeMap::<String, u64>::new();
    for row in rows {
        let Some(timestamp) = DateTime::from_timestamp_millis(row.started_at_ms) else {
            continue;
        };
        let saved = row
            .raw_tokens_estimate
            .unwrap_or_default()
            .saturating_sub(row.returned_tokens_estimate.unwrap_or_default());
        *daily
            .entry(timestamp.format("%Y-%m-%d").to_string())
            .or_default() += saved;
    }
    println!("Daily direct estimated token reduction");
    if daily.is_empty() {
        println!("No matching executions.");
        return Ok(());
    }
    let maximum = daily.values().copied().max().unwrap_or(1).max(1);
    for (day, saved) in daily {
        let width = ((saved.saturating_mul(30) / maximum) as usize).max(usize::from(saved > 0));
        println!("{day} {:>9} {}", saved, "#".repeat(width));
    }
    Ok(())
}

pub fn run_quality_cli(query: RunQuery, format: &str) -> Result<()> {
    let report = RunAnalytics::open_default()?.quality(&query)?;
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if format != "text" {
        bail!("quality format must be text or json");
    }
    println!("ContextDroid quality proxies from canonical executions");
    println!("Executions: {}", report.runs);
    println!(
        "Observed confidence: high {}, medium {}, low {}",
        report.high_confidence, report.medium_confidence, report.low_confidence
    );
    println!(
        "Observed fallbacks: {} ({:.1}%), never-worse {}",
        report.fallbacks, report.fallback_rate_percent, report.never_worse_fallbacks
    );
    println!("Observed parser errors: {}", report.parser_errors);
    println!(
        "Observed raw recoveries: {} ({:.1}%)",
        report.raw_recoveries, report.raw_recovery_rate_percent
    );
    println!("Observed detectable reruns: {}", report.detectable_reruns);
    println!(
        "Exit-code parity: {} failures, {} unknown",
        report.exit_parity_failures, report.exit_parity_unknown
    );
    println!(
        "Fixture preservation: {} failures, {} unknown",
        report.fixture_preservation_failures, report.fixture_preservation_unknown
    );
    Ok(())
}

pub fn run_session_cli() -> Result<()> {
    let sessions = RunAnalytics::open_default()?.sessions(LastCount::new(10)?)?;
    if sessions.is_empty() {
        println!("No ContextDroid executions recorded.");
        return Ok(());
    }
    println!("ContextDroid canonical sessions (latest 10)");
    println!("Session                         Executions     Raw  Returned");
    for session in sessions {
        println!(
            "{:<31} {:>10} {:>7} {:>9}",
            session.session_id,
            session.executions,
            session.raw_tokens_estimate,
            session.returned_tokens_estimate
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::run_store::RunId;

    fn metadata(id: &str, raw: u64, returned: u64) -> RunMetadata {
        RunMetadata {
            schema_version: 1,
            run_id: RunId::parse(id).unwrap(),
            command: "./gradlew assembleDebug".into(),
            cwd: PathBuf::from("/project"),
            started_at: Utc::now().to_rfc3339(),
            finished_at: Some(Utc::now().to_rfc3339()),
            duration_ms: Some(12),
            exit_code: Some(1),
            signal: None,
            profile: "contextdroid-safe".into(),
            output_mode: "balanced".into(),
            parser: Some("android-gradle".into()),
            confidence: Some("high".into()),
            raw_fallback: false,
            never_worse_fallback: false,
            recovery_requested: false,
            stdout_bytes: raw,
            stderr_bytes: 0,
            stdout_sha256: String::new(),
            stderr_sha256: String::new(),
            complete: true,
            scope: "android".into(),
            command_family: "gradle".into(),
            project_path: "/project".into(),
            raw_bytes: raw,
            returned_bytes: returned,
            raw_lines: 10,
            returned_lines: 3,
            raw_tokens_estimate: raw / 4,
            returned_tokens_estimate: returned / 4,
            omission_preserved: 3,
            omission_collapsed: 7,
            parser_error: false,
            detectable_rerun: false,
            exit_code_parity: true,
            fixture_preservation: true,
        }
    }

    #[test]
    fn schema_and_gain_distinguish_direct_from_effective_estimates() {
        let analytics = RunAnalytics::in_memory().unwrap();
        assert_eq!(analytics.schema_version().unwrap(), SCHEMA_VERSION);
        let run = metadata("20260715-run-a", 400, 100);
        analytics.record(&run).unwrap();
        let direct = analytics.gain(&RunQuery::default()).unwrap();
        assert_eq!(direct.direct_saved_tokens_estimate, 75);
        analytics.mark_recovery(run.run_id.as_str()).unwrap();
        let effective = analytics.gain(&RunQuery::default()).unwrap();
        assert_eq!(effective.effective_saved_tokens_estimate, 0);
    }

    #[test]
    fn quality_does_not_claim_unmeasured_fixture_or_exit_parity() {
        let analytics = RunAnalytics::in_memory().unwrap();
        analytics
            .record(&metadata("20260715-run-b", 400, 400))
            .unwrap();
        let quality = analytics.quality(&RunQuery::default()).unwrap();
        assert_eq!(quality.exit_parity_unknown, 1);
        assert_eq!(quality.fixture_preservation_unknown, 1);
    }

    #[test]
    fn canonical_store_combines_android_and_general_without_double_counting() {
        let analytics = RunAnalytics::in_memory().unwrap();
        analytics
            .record(&metadata("20260715-android-canonical", 400, 100))
            .unwrap();
        analytics.conn.execute_batch(
            "CREATE TABLE commands (
               id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, original_cmd TEXT NOT NULL,
               rtk_cmd TEXT NOT NULL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               saved_tokens INTEGER NOT NULL, savings_pct REAL NOT NULL, exec_time_ms INTEGER NOT NULL,
               project_path TEXT NOT NULL);",
        ).unwrap();
        analytics
            .conn
            .execute(
                "INSERT INTO commands VALUES (1, ?1, 'git status', 'contextdroid git status',
             100, 20, 80, 80.0, 5, '/project')",
                [Utc::now().to_rfc3339()],
            )
            .unwrap();
        let all = analytics.gain(&RunQuery::default()).unwrap();
        let weekly = analytics
            .gain(&RunQuery {
                since: Some(PositiveDuration::WEEK),
                ..RunQuery::default()
            })
            .unwrap();
        let android = analytics
            .gain(&RunQuery {
                scope: Some("android".into()),
                ..RunQuery::default()
            })
            .unwrap();
        let general = analytics
            .gain(&RunQuery {
                scope: Some("general".into()),
                ..RunQuery::default()
            })
            .unwrap();
        assert_eq!(
            (all.runs, weekly.runs, android.runs, general.runs),
            (2, 2, 1, 1)
        );
    }

    #[test]
    fn legacy_path_like_commands_do_not_break_migration() {
        // Regression: legacy `commands` rows whose first argument is a path
        // (e.g. `ls -la /home/user/foo`) must not produce an operation label
        // containing '/' — classify_operation must strip disallowed
        // characters throughout the token, not just at the ends.
        let analytics = RunAnalytics::in_memory().unwrap();
        analytics
            .conn
            .execute_batch(
                "CREATE TABLE commands (
               id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, original_cmd TEXT NOT NULL,
               rtk_cmd TEXT NOT NULL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               saved_tokens INTEGER NOT NULL, savings_pct REAL NOT NULL, exec_time_ms INTEGER NOT NULL,
               project_path TEXT NOT NULL);",
            )
            .unwrap();
        analytics
            .conn
            .execute(
                "INSERT INTO commands VALUES (1, ?1, 'ls -la /home/user/some/deep/path',
             'contextdroid ls -la /home/user/some/deep/path', 100, 20, 80, 80.0, 5, '/project')",
                [Utc::now().to_rfc3339()],
            )
            .unwrap();

        let gain = analytics.gain(&RunQuery::default()).unwrap();
        assert_eq!(gain.runs, 1);

        let rows = analytics.executions(&RunQuery::default()).unwrap();
        assert!(rows.iter().all(|row| !row.operation.contains('/')));
    }

    #[test]
    fn classify_operation_strips_disallowed_characters_throughout() {
        assert_eq!(
            classify_operation("ls -la /home/user/some/deep/path"),
            "homeusersomedeeppath"
        );
        assert_eq!(classify_operation("gradlew assembleDebug"), "assembledebug");
        assert_eq!(classify_operation("git"), "other");
    }

    #[test]
    fn classify_family_strips_disallowed_characters_throughout() {
        assert_eq!(classify_family("./gradlew assembleDebug"), "gradle");
        assert_eq!(classify_family("/usr/bin/git status"), "usrbingit");
    }

    #[test]
    fn last_is_applied_after_orthogonal_filters() {
        let analytics = RunAnalytics::in_memory().unwrap();
        for (index, scope) in ["android", "general", "android"].into_iter().enumerate() {
            let record = ExecutionRecordBuilder::new(
                format!("last-{index}"),
                scope,
                if scope == "android" { "gradle" } else { "git" },
            )
            .started_at_ms(Utc::now().timestamp_millis() + index as i64)
            .build()
            .unwrap();
            analytics.record_execution(&record).unwrap();
        }
        let rows = analytics
            .executions(&RunQuery {
                scope: Some("android".into()),
                last: Some(LastCount::new(1).unwrap()),
                ..RunQuery::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scope, "android");
    }

    #[test]
    fn command_project_profile_parser_and_time_are_orthogonal_filters() {
        let analytics = RunAnalytics::in_memory().unwrap();
        let now = Utc::now().timestamp_millis();
        for (key, project, profile, parser, started_at_ms) in [
            (
                "matching",
                "/wanted",
                "contextdroid-safe",
                "android-gradle",
                now,
            ),
            (
                "wrong-project",
                "/other",
                "contextdroid-safe",
                "android-gradle",
                now,
            ),
            (
                "wrong-profile",
                "/wanted",
                "android-only",
                "android-gradle",
                now,
            ),
            (
                "wrong-parser",
                "/wanted",
                "contextdroid-safe",
                "other-parser",
                now,
            ),
            (
                "too-old",
                "/wanted",
                "contextdroid-safe",
                "android-gradle",
                now - PositiveDuration::WEEK.millis() - 1,
            ),
        ] {
            let record = ExecutionRecordBuilder::new(key, "android", "gradle")
                .project(Some(project.into()))
                .profile(profile)
                .parser(Some(parser.into()))
                .started_at_ms(started_at_ms)
                .build()
                .unwrap();
            analytics.record_execution(&record).unwrap();
        }
        let rows = analytics
            .executions(&RunQuery {
                scope: Some("android".into()),
                command_family: Some("gradle".into()),
                project: Some("/wanted".into()),
                profile: Some("contextdroid-safe".into()),
                parser: Some("android-gradle".into()),
                since: Some(PositiveDuration::WEEK),
                last: Some(LastCount::new(1).unwrap()),
                ..RunQuery::default()
            })
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].command_family, "gradle");
        assert_eq!(rows[0].parser.as_deref(), Some("android-gradle"));
    }

    #[test]
    fn android_legacy_completion_row_is_not_double_counted() {
        let analytics = RunAnalytics::in_memory().unwrap();
        let mut run = metadata("20260715-durable-dedup", 400, 100);
        let start = Utc::now();
        run.started_at = start.to_rfc3339();
        run.duration_ms = Some(1_000);
        analytics.record(&run).unwrap();
        analytics.conn.execute_batch(
            "CREATE TABLE commands (
               id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, original_cmd TEXT NOT NULL,
               rtk_cmd TEXT NOT NULL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               saved_tokens INTEGER NOT NULL, savings_pct REAL NOT NULL, exec_time_ms INTEGER NOT NULL,
               project_path TEXT NOT NULL);",
        ).unwrap();
        analytics
            .conn
            .execute(
                "INSERT INTO commands VALUES (1, ?1, './gradlew assembleDebug',
             'contextdroid gradlew assembleDebug', 100, 25, 75, 75.0, 1000, '/project')",
                [(start + chrono::Duration::seconds(1)).to_rfc3339()],
            )
            .unwrap();

        assert_eq!(analytics.gain(&RunQuery::default()).unwrap().runs, 1);
    }

    #[test]
    fn passthrough_records_unknown_sizes_and_compatibility_keys_do_not_collide() {
        let analytics = RunAnalytics::in_memory().unwrap();
        analytics
            .record_passthrough("adb logcat", "adb logcat", 10)
            .unwrap();
        analytics
            .record_compatibility("git status", "git status", "same", "same", 10)
            .unwrap();
        analytics
            .record_compatibility("git status", "git status", "same", "same", 10)
            .unwrap();
        let rows = analytics.executions(&RunQuery::default()).unwrap();
        assert_eq!(rows.len(), 3);
        let passthrough = rows.iter().find(|row| row.command_family == "adb").unwrap();
        assert_eq!(passthrough.raw_tokens_estimate, None);
        assert_eq!(passthrough.returned_tokens_estimate, None);
    }
}
