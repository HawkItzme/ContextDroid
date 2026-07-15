//! Local-only analytics for durable ContextDroid runs.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::run_store::RunMetadata;

const SCHEMA_VERSION: i64 = 1;

pub struct RunAnalytics {
    conn: Connection,
}

#[derive(Debug, Clone, Default)]
pub struct RunQuery {
    pub scope: Option<String>,
    pub command_family: Option<String>,
    pub project: Option<String>,
    pub since_days: Option<i64>,
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
    pub parser_errors: u64,
    pub raw_recoveries: u64,
    pub detectable_reruns: u64,
    pub exit_parity_failures: u64,
    pub fixture_preservation_failures: u64,
    pub fallback_rate_percent: f64,
    pub raw_recovery_rate_percent: f64,
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
             CREATE TABLE IF NOT EXISTS contextdroid_runs (
               run_id TEXT PRIMARY KEY,
               started_at TEXT NOT NULL,
               command TEXT NOT NULL,
               scope TEXT NOT NULL,
               command_family TEXT NOT NULL,
               parser TEXT,
               profile TEXT NOT NULL,
               project_path TEXT NOT NULL,
               raw_bytes INTEGER NOT NULL,
               returned_bytes INTEGER NOT NULL,
               raw_lines INTEGER NOT NULL,
               returned_lines INTEGER NOT NULL,
               raw_tokens_estimate INTEGER NOT NULL,
               returned_tokens_estimate INTEGER NOT NULL,
               duration_ms INTEGER NOT NULL,
               exit_code INTEGER,
               signal INTEGER,
               confidence TEXT,
               raw_fallback INTEGER NOT NULL,
               recovery_requested INTEGER NOT NULL,
               parser_error INTEGER NOT NULL,
               detectable_rerun INTEGER NOT NULL DEFAULT 0,
               exit_code_parity INTEGER NOT NULL,
               fixture_preservation INTEGER NOT NULL,
               omission_preserved INTEGER NOT NULL,
               omission_collapsed INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_contextdroid_runs_started
               ON contextdroid_runs(started_at);
             CREATE INDEX IF NOT EXISTS idx_contextdroid_runs_family
               ON contextdroid_runs(command_family, started_at);
             CREATE INDEX IF NOT EXISTS idx_contextdroid_runs_project
               ON contextdroid_runs(project_path, started_at);",
        )?;
        self.conn
            .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    pub fn record(&self, metadata: &RunMetadata) -> Result<()> {
        self.conn.execute(
            "INSERT INTO contextdroid_runs (
               run_id, started_at, command, scope, command_family, parser, profile, project_path,
               raw_bytes, returned_bytes, raw_lines, returned_lines,
               raw_tokens_estimate, returned_tokens_estimate, duration_ms,
               exit_code, signal, confidence, raw_fallback, recovery_requested,
               parser_error, detectable_rerun, exit_code_parity, fixture_preservation,
               omission_preserved, omission_collapsed
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
               ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26
             )
             ON CONFLICT(run_id) DO UPDATE SET
               recovery_requested=excluded.recovery_requested,
               returned_bytes=excluded.returned_bytes,
               returned_lines=excluded.returned_lines,
               returned_tokens_estimate=excluded.returned_tokens_estimate",
            params![
                metadata.run_id.as_str(),
                metadata.started_at,
                metadata.command,
                metadata.scope,
                metadata.command_family,
                metadata.parser,
                metadata.profile,
                metadata.project_path,
                metadata.raw_bytes,
                metadata.returned_bytes,
                metadata.raw_lines,
                metadata.returned_lines,
                metadata.raw_tokens_estimate,
                metadata.returned_tokens_estimate,
                metadata.duration_ms.unwrap_or_default(),
                metadata.exit_code,
                metadata.signal,
                metadata.confidence,
                metadata.raw_fallback,
                metadata.recovery_requested,
                metadata.parser_error,
                metadata.detectable_rerun,
                metadata.exit_code_parity,
                metadata.fixture_preservation,
                metadata.omission_preserved,
                metadata.omission_collapsed,
            ],
        )?;
        Ok(())
    }

    pub fn mark_recovery(&self, run_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE contextdroid_runs SET recovery_requested=1 WHERE run_id=?1",
            [run_id],
        )?;
        Ok(())
    }

    pub fn gain(&self, query: &RunQuery) -> Result<GainReport> {
        let rows = self.filtered_rows(query)?;
        let mut report = GainReport::default();
        for row in rows {
            report.runs += 1;
            report.raw_tokens_estimate += row.raw_tokens;
            report.returned_tokens_estimate += row.returned_tokens;
            report.effective_returned_tokens_estimate += row.returned_tokens;
            if row.recovery_requested {
                report.effective_returned_tokens_estimate += row.raw_tokens;
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
        let rows = self.filtered_rows(query)?;
        let mut report = QualityReport::default();
        for row in rows {
            report.runs += 1;
            match row.confidence.as_deref() {
                Some("high") => report.high_confidence += 1,
                Some("medium") => report.medium_confidence += 1,
                Some("low") => report.low_confidence += 1,
                _ => {}
            }
            report.fallbacks += u64::from(row.raw_fallback);
            report.parser_errors += u64::from(row.parser_error);
            report.raw_recoveries += u64::from(row.recovery_requested);
            report.detectable_reruns += u64::from(row.detectable_rerun);
            report.exit_parity_failures += u64::from(!row.exit_code_parity);
            report.fixture_preservation_failures += u64::from(!row.fixture_preservation);
        }
        report.fallback_rate_percent = percent(report.fallbacks, report.runs);
        report.raw_recovery_rate_percent = percent(report.raw_recoveries, report.runs);
        Ok(report)
    }

    fn filtered_rows(&self, query: &RunQuery) -> Result<Vec<AnalyticsRow>> {
        let since = query
            .since_days
            .map(|days| (Utc::now() - Duration::days(days)).to_rfc3339());
        let mut stmt = self.conn.prepare(
            "SELECT raw_tokens_estimate, returned_tokens_estimate, confidence,
                    raw_fallback, recovery_requested, parser_error, detectable_rerun,
                    exit_code_parity, fixture_preservation
             FROM contextdroid_runs
             WHERE (?1 IS NULL OR scope = ?1)
               AND (?2 IS NULL OR command_family = ?2 OR parser = ?2)
               AND (?3 IS NULL OR project_path = ?3 OR project_path GLOB (?3 || '/*')
                    OR project_path GLOB (?3 || '\\*'))
               AND (?4 IS NULL OR started_at >= ?4)
             ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map(
            params![
                query.scope.as_deref(),
                query.command_family.as_deref(),
                query.project.as_deref(),
                since.as_deref()
            ],
            |row| {
                Ok(AnalyticsRow {
                    raw_tokens: row.get(0)?,
                    returned_tokens: row.get(1)?,
                    confidence: row.get(2)?,
                    raw_fallback: row.get(3)?,
                    recovery_requested: row.get(4)?,
                    parser_error: row.get(5)?,
                    detectable_rerun: row.get(6)?,
                    exit_code_parity: row.get(7)?,
                    fixture_preservation: row.get(8)?,
                })
            },
        )?;
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
                "SELECT recovery_requested FROM contextdroid_runs WHERE run_id=?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}

struct AnalyticsRow {
    raw_tokens: u64,
    returned_tokens: u64,
    confidence: Option<String>,
    raw_fallback: bool,
    recovery_requested: bool,
    parser_error: bool,
    detectable_rerun: bool,
    exit_code_parity: bool,
    fixture_preservation: bool,
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

pub fn mark_recovery_silent(run_id: &str) {
    if let Ok(analytics) = RunAnalytics::open_default() {
        let _ = analytics.mark_recovery(run_id);
    }
}

pub fn parse_since(value: &str) -> Result<i64> {
    let days = value
        .strip_suffix('d')
        .unwrap_or(value)
        .parse::<i64>()
        .context("--since must be a positive day count such as 7d")?;
    if days <= 0 {
        anyhow::bail!("--since must be positive");
    }
    Ok(days)
}

pub fn validate_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .context("invalid analytics timestamp")
}

pub fn run_gain_cli(
    scope: Option<String>,
    command_family: Option<String>,
    project: Option<String>,
    since: Option<String>,
    format: &str,
) -> Result<()> {
    let query = cli_query(scope, command_family, project, since)?;
    let report = RunAnalytics::open_default()?.gain(&query)?;
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if format != "text" {
        anyhow::bail!("run-centric gain supports text or json output");
    }
    println!("ContextDroid output reduction (estimated tokens)");
    println!("Runs: {}", report.runs);
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
    Ok(())
}

pub fn run_quality_cli(
    scope: Option<String>,
    command_family: Option<String>,
    project: Option<String>,
    since: Option<String>,
    format: &str,
) -> Result<()> {
    let query = cli_query(scope, command_family, project, since)?;
    let report = RunAnalytics::open_default()?.quality(&query)?;
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if format != "text" {
        anyhow::bail!("quality supports text or json output");
    }
    println!("ContextDroid quality proxies");
    println!("Runs: {}", report.runs);
    println!(
        "Confidence: high {}, medium {}, low {}",
        report.high_confidence, report.medium_confidence, report.low_confidence
    );
    println!(
        "Fallbacks: {} ({:.1}%)",
        report.fallbacks, report.fallback_rate_percent
    );
    println!("Parser errors: {}", report.parser_errors);
    println!(
        "Raw recoveries: {} ({:.1}%)",
        report.raw_recoveries, report.raw_recovery_rate_percent
    );
    println!("Detectable reruns: {}", report.detectable_reruns);
    println!("Exit-code parity failures: {}", report.exit_parity_failures);
    println!(
        "Fixture preservation failures: {}",
        report.fixture_preservation_failures
    );
    Ok(())
}

fn cli_query(
    scope: Option<String>,
    command_family: Option<String>,
    project: Option<String>,
    since: Option<String>,
) -> Result<RunQuery> {
    let project = project
        .map(|value| {
            let path = if value == "." {
                std::env::current_dir()?
            } else {
                PathBuf::from(value)
            };
            Ok::<_, anyhow::Error>(
                path.canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .transpose()?;
    Ok(RunQuery {
        scope,
        command_family,
        project,
        since_days: since.as_deref().map(parse_since).transpose()?,
    })
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
        assert_eq!(direct.effective_saved_tokens_estimate, 75);

        analytics.mark_recovery(run.run_id.as_str()).unwrap();
        let effective = analytics.gain(&RunQuery::default()).unwrap();
        assert_eq!(effective.direct_saved_tokens_estimate, 75);
        assert_eq!(effective.effective_saved_tokens_estimate, 0);
    }

    #[test]
    fn quality_reports_confidence_fallback_and_recovery() {
        let analytics = RunAnalytics::in_memory().unwrap();
        let mut run = metadata("20260715-run-b", 400, 400);
        run.confidence = Some("low".into());
        run.raw_fallback = true;
        run.parser_error = true;
        run.recovery_requested = true;
        analytics.record(&run).unwrap();
        let quality = analytics.quality(&RunQuery::default()).unwrap();
        assert_eq!(quality.low_confidence, 1);
        assert_eq!(quality.fallbacks, 1);
        assert_eq!(quality.parser_errors, 1);
        assert_eq!(quality.raw_recoveries, 1);
    }

    #[test]
    fn filters_by_family_project_and_since() {
        let analytics = RunAnalytics::in_memory().unwrap();
        analytics
            .record(&metadata("20260715-run-c", 400, 100))
            .unwrap();
        let report = analytics
            .gain(&RunQuery {
                scope: Some("android".into()),
                project: Some("/project".into()),
                since_days: Some(7),
                ..RunQuery::default()
            })
            .unwrap();
        assert_eq!(report.runs, 1);
        let empty = analytics
            .gain(&RunQuery {
                scope: Some("git".into()),
                ..RunQuery::default()
            })
            .unwrap();
        assert_eq!(empty.runs, 0);
    }

    #[test]
    fn since_parser_rejects_non_positive_values() {
        assert_eq!(parse_since("7d").unwrap(), 7);
        assert!(parse_since("0d").is_err());
        assert!(parse_since("soon").is_err());
    }
}
