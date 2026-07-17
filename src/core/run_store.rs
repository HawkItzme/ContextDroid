use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const SCHEMA_VERSION: u32 = 2;
static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

impl RunId {
    pub fn parse(value: &str) -> Result<Self> {
        let valid = !value.is_empty()
            && value != "."
            && value != ".."
            && value.len() <= 96
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'));
        if !valid {
            anyhow::bail!("invalid run ID");
        }
        Ok(Self(value.to_string()))
    }

    fn generate() -> Self {
        let now = Utc::now();
        let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix_seed = format!(
            "{}-{}-{}",
            now.timestamp_nanos_opt().unwrap_or_default(),
            std::process::id(),
            sequence
        );
        let suffix = format!("{:x}", Sha256::digest(suffix_seed.as_bytes()));
        Self(format!(
            "{}-{}",
            now.format("%Y%m%dT%H%M%S%.3fZ"),
            &suffix[..8]
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct RunStart {
    pub command: String,
    pub cwd: PathBuf,
    pub profile: String,
    pub output_mode: String,
}

#[derive(Debug, Clone, Default)]
pub struct FinalizeDetails {
    pub parser: Option<String>,
    pub confidence: Option<String>,
    pub raw_fallback: bool,
    pub never_worse_fallback: bool,
    pub parser_error: bool,
    pub omission_preserved: u64,
    pub omission_collapsed: u64,
    pub fixture_preservation: Option<bool>,
    pub exit_code_parity: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    ExitCode(i32),
    Signal(i32),
}

impl ProcessOutcome {
    pub fn from_status(status: ExitStatus) -> Self {
        if let Some(code) = status.code() {
            return Self::ExitCode(code);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Self::Signal(signal);
            }
        }
        Self::ExitCode(1)
    }

    pub fn exit_code(self) -> Option<i32> {
        match self {
            Self::ExitCode(code) => Some(code),
            Self::Signal(_) => None,
        }
    }

    pub fn signal(self) -> Option<i32> {
        match self {
            Self::ExitCode(_) => None,
            Self::Signal(signal) => Some(signal),
        }
    }

    pub fn shell_exit_code(self) -> i32 {
        match self {
            Self::ExitCode(code) => code,
            Self::Signal(signal) => 128 + signal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub schema_version: u32,
    pub run_id: RunId,
    pub command: String,
    pub cwd: PathBuf,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub profile: String,
    pub output_mode: String,
    pub parser: Option<String>,
    pub confidence: Option<String>,
    pub raw_fallback: bool,
    #[serde(default)]
    pub never_worse_fallback: bool,
    pub recovery_requested: bool,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub complete: bool,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub command_family: String,
    #[serde(default)]
    pub project_path: String,
    #[serde(default)]
    pub raw_bytes: u64,
    #[serde(default)]
    pub returned_bytes: u64,
    #[serde(default)]
    pub raw_lines: u64,
    #[serde(default)]
    pub returned_lines: u64,
    #[serde(default)]
    pub raw_tokens_estimate: u64,
    #[serde(default)]
    pub returned_tokens_estimate: u64,
    #[serde(default)]
    pub omission_preserved: u64,
    #[serde(default)]
    pub omission_collapsed: u64,
    #[serde(default)]
    pub parser_error: bool,
    #[serde(default)]
    pub detectable_rerun: bool,
    #[serde(default)]
    pub exit_code_parity: Option<bool>,
    #[serde(default)]
    pub fixture_preservation: Option<bool>,
}

pub struct RunStore {
    root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub max_age_days: u64,
    pub max_runs: usize,
    pub max_bytes: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_age_days: 7,
            max_runs: 200,
            max_bytes: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub kept_runs: usize,
    pub kept_bytes: u64,
    pub removed_runs: usize,
    pub removed_bytes: u64,
}

impl RunStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn default_store() -> Result<Self> {
        let root = if let Some(root) = std::env::var_os("CONTEXTDROID_RUNS_DIR") {
            PathBuf::from(root)
        } else {
            crate::product::data_dir()
                .context("cannot determine ContextDroid data directory")?
                .join("runs")
        };
        crate::core::secure_fs::reject_store_inside_repository(&root)?;
        Ok(Self::new(root))
    }

    pub fn start(&self, start: RunStart) -> Result<ActiveRun> {
        let now = Utc::now();
        let run_id = RunId::generate();
        let path = self
            .root
            .join(now.format("%Y").to_string())
            .join(now.format("%m").to_string())
            .join(now.format("%d").to_string())
            .join(run_id.as_str());
        crate::core::secure_fs::ensure_private_dir(&path)?;

        let stdout = create_raw_file(&path.join("stdout.log"))?;
        let stderr = create_raw_file(&path.join("stderr.log"))?;
        let project_path = start.cwd.to_string_lossy().into_owned();
        let metadata = RunMetadata {
            schema_version: SCHEMA_VERSION,
            run_id,
            command: start.command,
            cwd: start.cwd,
            started_at: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            finished_at: None,
            duration_ms: None,
            exit_code: None,
            signal: None,
            profile: start.profile,
            output_mode: start.output_mode,
            parser: None,
            confidence: None,
            raw_fallback: false,
            never_worse_fallback: false,
            recovery_requested: false,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: String::new(),
            stderr_sha256: String::new(),
            complete: false,
            scope: String::new(),
            command_family: String::new(),
            project_path,
            raw_bytes: 0,
            returned_bytes: 0,
            raw_lines: 0,
            returned_lines: 0,
            raw_tokens_estimate: 0,
            returned_tokens_estimate: 0,
            omission_preserved: 0,
            omission_collapsed: 0,
            parser_error: false,
            detectable_rerun: false,
            exit_code_parity: None,
            fixture_preservation: None,
        };
        write_json_atomic(&path.join("metadata.partial.json"), &metadata)?;

        Ok(ActiveRun {
            path,
            stdout,
            stderr,
            stdout_bytes: 0,
            stderr_bytes: 0,
            metadata,
            started: Instant::now(),
            finalized: false,
        })
    }

    pub fn load(&self, id: &RunId) -> Result<StoredRun> {
        let path = self.path_for(id)?;
        let complete = path.join("metadata.json");
        let partial = path.join("metadata.partial.json");
        let metadata_path = if complete.is_file() {
            complete
        } else {
            partial
        };
        let bytes = fs::read(&metadata_path)
            .with_context(|| format!("run not found or metadata unreadable: {}", id.as_str()))?;
        let mut metadata: RunMetadata =
            serde_json::from_slice(&bytes).context("run metadata is corrupt")?;
        if metadata.schema_version < 2 {
            metadata.exit_code_parity = None;
            metadata.fixture_preservation = None;
        }
        if metadata.run_id != *id {
            anyhow::bail!("run metadata ID does not match requested ID");
        }
        Ok(StoredRun { path, metadata })
    }

    pub fn mark_recovery_requested(&self, id: &RunId) -> Result<()> {
        let mut run = self.load(id)?;
        run.metadata.recovery_requested = true;
        let metadata_path = if run.metadata.complete {
            run.path.join("metadata.json")
        } else {
            run.path.join("metadata.partial.json")
        };
        write_json_replace(&metadata_path, &run.metadata)
    }

    fn path_for(&self, id: &RunId) -> Result<PathBuf> {
        let value = id.as_str();
        if value.len() < 8 || !value.as_bytes()[..8].iter().all(u8::is_ascii_digit) {
            anyhow::bail!("run ID has no valid date prefix");
        }
        Ok(self
            .root
            .join(&value[0..4])
            .join(&value[4..6])
            .join(&value[6..8])
            .join(value))
    }

    pub fn remove(&self, id: &RunId) -> Result<()> {
        let path = self.path_for(id)?;
        crate::core::secure_fs::reject_reparse_components(&path)?;
        if path.exists() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove run {}", id.as_str()))?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<(String, u64)>> {
        let mut runs = self.run_candidates()?;
        runs.sort_by_key(|run| std::cmp::Reverse(run.started_at));
        Ok(runs
            .into_iter()
            .filter_map(|run| {
                run.path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|id| (id.to_string(), run.bytes))
            })
            .collect())
    }

    pub fn purge_all(&self) -> Result<usize> {
        let runs = self.run_candidates()?;
        let count = runs.len();
        for run in runs {
            crate::core::secure_fs::reject_reparse_components(&run.path)?;
            fs::remove_dir_all(&run.path)
                .with_context(|| format!("failed to purge {}", run.path.display()))?;
        }
        Ok(count)
    }

    pub fn prune(&self, policy: RetentionPolicy) -> Result<PruneReport> {
        self.prune_at(policy, Utc::now())
    }

    fn prune_at(&self, policy: RetentionPolicy, now: chrono::DateTime<Utc>) -> Result<PruneReport> {
        let mut runs = self.run_candidates()?;
        runs.sort_by_key(|run| std::cmp::Reverse(run.started_at));
        let mut report = PruneReport::default();

        for run in runs {
            let age_days = now.signed_duration_since(run.started_at).num_days();
            let within_age = age_days <= policy.max_age_days as i64;
            let within_count = report.kept_runs < policy.max_runs;
            let within_bytes = report.kept_bytes.saturating_add(run.bytes) <= policy.max_bytes;
            if within_age && within_count && within_bytes {
                report.kept_runs += 1;
                report.kept_bytes = report.kept_bytes.saturating_add(run.bytes);
            } else {
                fs::remove_dir_all(&run.path).with_context(|| {
                    format!("failed to remove retained run: {}", run.path.display())
                })?;
                report.removed_runs += 1;
                report.removed_bytes = report.removed_bytes.saturating_add(run.bytes);
            }
        }
        Ok(report)
    }

    fn run_candidates(&self) -> Result<Vec<RunCandidate>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut candidates = Vec::new();
        for year in child_directories(&self.root)? {
            for month in child_directories(&year)? {
                for day in child_directories(&month)? {
                    for path in child_directories(&day)? {
                        let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
                            continue;
                        };
                        let Ok(run_id) = RunId::parse(id) else {
                            continue;
                        };
                        let Some(started_at) = parse_run_timestamp(&run_id) else {
                            continue;
                        };
                        candidates.push(RunCandidate {
                            bytes: directory_size(&path)?,
                            path,
                            started_at,
                        });
                    }
                }
            }
        }
        Ok(candidates)
    }
}

struct RunCandidate {
    path: PathBuf,
    started_at: chrono::DateTime<Utc>,
    bytes: u64,
}

fn child_directories(path: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to inspect run store: {}", path.display()))?
    {
        let entry = entry.context("failed to inspect run-store entry")?;
        if entry.file_type()?.is_dir() {
            directories.push(entry.path());
        }
    }
    Ok(directories)
}

fn parse_run_timestamp(id: &RunId) -> Option<chrono::DateTime<Utc>> {
    let timestamp = id.as_str().split_once('-')?.0;
    chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%S%.3fZ")
        .ok()
        .map(|value| value.and_utc())
}

fn directory_size(path: &Path) -> Result<u64> {
    let mut bytes = 0_u64;
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry.context("failed to inspect run artifact")?;
        if entry.file_type().is_file() {
            bytes = bytes.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(bytes)
}

pub struct ActiveRun {
    path: PathBuf,
    stdout: File,
    stderr: File,
    stdout_bytes: u64,
    stderr_bytes: u64,
    metadata: RunMetadata,
    started: Instant,
    finalized: bool,
}

impl ActiveRun {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> &RunId {
        &self.metadata.run_id
    }

    pub fn write_stdout(&mut self, bytes: &[u8]) -> Result<()> {
        self.stdout
            .write_all(bytes)
            .context("failed to persist stdout")?;
        self.stdout_bytes = self.stdout_bytes.saturating_add(bytes.len() as u64);
        Ok(())
    }

    pub fn write_stderr(&mut self, bytes: &[u8]) -> Result<()> {
        self.stderr
            .write_all(bytes)
            .context("failed to persist stderr")?;
        self.stderr_bytes = self.stderr_bytes.saturating_add(bytes.len() as u64);
        Ok(())
    }

    pub fn capture_command(
        &mut self,
        command: &mut Command,
        inherit_stdin: bool,
    ) -> Result<ProcessOutcome> {
        self.stdout
            .set_len(0)
            .context("failed to reset stdout artifact")?;
        self.stderr
            .set_len(0)
            .context("failed to reset stderr artifact")?;
        let stdout = self
            .stdout
            .try_clone()
            .context("failed to clone stdout artifact")?;
        let stderr = self
            .stderr
            .try_clone()
            .context("failed to clone stderr artifact")?;
        command.stdin(if inherit_stdin {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::from(stdout));
        command.stderr(Stdio::from(stderr));

        let status = command
            .status()
            .context("failed to execute captured command")?;
        self.sync_raw()?;
        self.stdout_bytes = fs::metadata(self.path.join("stdout.log"))?.len();
        self.stderr_bytes = fs::metadata(self.path.join("stderr.log"))?.len();
        Ok(ProcessOutcome::from_status(status))
    }

    pub fn read_stdout(&self) -> Result<Vec<u8>> {
        fs::read(self.path.join("stdout.log")).context("failed to read stdout artifact")
    }

    pub fn read_stderr(&self) -> Result<Vec<u8>> {
        fs::read(self.path.join("stderr.log")).context("failed to read stderr artifact")
    }

    pub fn sync_raw(&mut self) -> Result<()> {
        self.stdout.flush().context("failed to flush stdout")?;
        self.stderr.flush().context("failed to flush stderr")?;
        self.stdout.sync_all().context("failed to sync stdout")?;
        self.stderr.sync_all().context("failed to sync stderr")?;
        Ok(())
    }

    pub fn finalize(
        mut self,
        outcome: ProcessOutcome,
        diagnostics_json: &str,
        summary: &str,
        details: FinalizeDetails,
    ) -> Result<StoredRun> {
        let _: serde_json::Value = serde_json::from_str(diagnostics_json)
            .context("diagnostics artifact is not valid JSON")?;
        self.sync_raw()?;
        write_bytes_atomic(
            &self.path.join("diagnostics.json"),
            diagnostics_json.as_bytes(),
        )?;
        write_bytes_atomic(&self.path.join("summary.txt"), summary.as_bytes())?;

        self.metadata.finished_at = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));
        self.metadata.duration_ms = Some(self.started.elapsed().as_millis() as u64);
        self.metadata.exit_code = outcome.exit_code();
        self.metadata.signal = outcome.signal();
        self.metadata.parser = details.parser;
        self.metadata.confidence = details.confidence;
        self.metadata.raw_fallback = details.raw_fallback;
        self.metadata.never_worse_fallback = details.never_worse_fallback;
        self.metadata.stdout_bytes = self.stdout_bytes;
        self.metadata.stderr_bytes = self.stderr_bytes;
        self.metadata.stdout_sha256 = hash_file(&self.path.join("stdout.log"))?;
        self.metadata.stderr_sha256 = hash_file(&self.path.join("stderr.log"))?;
        let stdout = fs::read(self.path.join("stdout.log"))?;
        let stderr = fs::read(self.path.join("stderr.log"))?;
        let (scope, family) = command_scope_and_family(&self.metadata.command);
        self.metadata.scope = scope.to_string();
        self.metadata.command_family = family.to_string();
        self.metadata.raw_bytes = self.stdout_bytes.saturating_add(self.stderr_bytes);
        self.metadata.returned_bytes = summary.len() as u64;
        self.metadata.raw_lines = count_lines(&stdout).saturating_add(count_lines(&stderr));
        self.metadata.returned_lines = count_lines(summary.as_bytes());
        self.metadata.raw_tokens_estimate = estimate_tokens(self.metadata.raw_bytes);
        self.metadata.returned_tokens_estimate = estimate_tokens(self.metadata.returned_bytes);
        self.metadata.omission_preserved = details.omission_preserved;
        self.metadata.omission_collapsed = details.omission_collapsed;
        self.metadata.parser_error = details.parser_error;
        self.metadata.exit_code_parity = details.exit_code_parity;
        self.metadata.fixture_preservation = details.fixture_preservation;
        self.metadata.complete = true;
        write_json_atomic(&self.path.join("metadata.json"), &self.metadata)?;
        fs::remove_file(self.path.join("metadata.partial.json"))
            .context("failed to remove partial metadata")?;
        self.finalized = true;
        Ok(StoredRun {
            path: self.path.clone(),
            metadata: self.metadata.clone(),
        })
    }
}

fn count_lines(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let newlines = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
    newlines + u64::from(bytes.last() != Some(&b'\n'))
}

fn estimate_tokens(bytes: u64) -> u64 {
    bytes.saturating_add(3) / 4
}

fn command_scope_and_family(command: &str) -> (&'static str, &'static str) {
    let lower = command.to_ascii_lowercase();
    if lower.starts_with("gradle ") || lower.starts_with("gradlew ") || lower.contains("gradlew") {
        ("android", "gradle")
    } else if lower.starts_with("adb logcat") || lower.starts_with("logcat ") {
        ("android", "logcat")
    } else if lower.starts_with("adb ") {
        ("android", "adb")
    } else if lower.starts_with("git ") {
        ("general", "git")
    } else {
        ("general", "other")
    }
}

impl Drop for ActiveRun {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.sync_raw();
            self.metadata.stdout_bytes = self.stdout_bytes;
            self.metadata.stderr_bytes = self.stderr_bytes;
            let _ = write_json_atomic(&self.path.join("metadata.partial.json"), &self.metadata);
        }
    }
}

pub struct StoredRun {
    path: PathBuf,
    pub metadata: RunMetadata,
}

impl StoredRun {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_summary(&self) -> Result<String> {
        fs::read_to_string(self.path.join("summary.txt")).context("summary artifact is unavailable")
    }

    pub fn read_diagnostics(&self) -> Result<String> {
        fs::read_to_string(self.path.join("diagnostics.json"))
            .context("diagnostics artifact is unavailable")
    }

    pub fn read_stdout(&self) -> Result<Vec<u8>> {
        fs::read(self.path.join("stdout.log")).context("stdout artifact is unavailable")
    }

    pub fn read_stderr(&self) -> Result<Vec<u8>> {
        fs::read(self.path.join("stderr.log")).context("stderr artifact is unavailable")
    }
}

fn create_raw_file(path: &Path) -> Result<File> {
    crate::core::secure_fs::create_private_new(path)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).context("failed to serialize run metadata")?;
    write_bytes_atomic(path, &bytes)
}

fn write_json_replace(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).context("failed to serialize run metadata")?;
    crate::core::secure_fs::atomic_write(path, &bytes)
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    crate::core::secure_fs::atomic_write(path, bytes)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("failed to open raw artifact: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .context("failed to hash raw artifact")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn start_run(root: &std::path::Path) -> ActiveRun {
        let store = RunStore::new(root.to_path_buf());
        store
            .start(RunStart {
                command: "./gradlew assembleDebug".into(),
                cwd: std::path::PathBuf::from("/workspace/app"),
                profile: "contextdroid-safe".into(),
                output_mode: "balanced".into(),
            })
            .expect("start run")
    }

    #[test]
    fn test_run_id_rejects_path_traversal() {
        for invalid in ["../secret", "a/b", "a\\b", "", ".", ".."] {
            assert!(RunId::parse(invalid).is_err(), "accepted {invalid:?}");
        }
        assert!(RunId::parse("20260715T010203.004Z-a1b2c3d4").is_ok());
    }

    #[test]
    fn test_start_creates_separate_raw_streams_and_partial_metadata() {
        let temp = tempfile::tempdir().unwrap();

        let root = temp.path().canonicalize().unwrap();
        let run = start_run(&root);

        assert!(run.path().join("stdout.log").is_file());
        assert!(run.path().join("stderr.log").is_file());
        assert!(run.path().join("metadata.partial.json").is_file());
        assert!(!run.path().join("metadata.json").exists());
    }

    #[test]
    fn test_raw_streams_preserve_exact_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut run = start_run(&root);
        let stdout = b"first\r\nsecond\n\0tail";
        let stderr = b"error: \xff\xfe\n";

        run.write_stdout(stdout).unwrap();
        run.write_stderr(stderr).unwrap();
        run.sync_raw().unwrap();

        assert_eq!(fs::read(run.path().join("stdout.log")).unwrap(), stdout);
        assert_eq!(fs::read(run.path().join("stderr.log")).unwrap(), stderr);
    }

    #[test]
    fn test_finalize_writes_required_artifacts_and_checksums() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut run = start_run(&root);
        run.write_stdout(b"BUILD FAILED\n").unwrap();
        run.write_stderr(b"root cause\n").unwrap();

        let stored = run
            .finalize(
                ProcessOutcome::ExitCode(1),
                "{\"events\":[]}",
                "failure summary",
                FinalizeDetails {
                    parser: Some("gradle".into()),
                    confidence: Some("low".into()),
                    raw_fallback: true,
                    ..FinalizeDetails::default()
                },
            )
            .unwrap();

        for name in [
            "metadata.json",
            "diagnostics.json",
            "summary.txt",
            "stdout.log",
            "stderr.log",
        ] {
            assert!(stored.path().join(name).is_file(), "missing {name}");
        }
        assert!(!stored.path().join("metadata.partial.json").exists());
        let metadata: RunMetadata =
            serde_json::from_slice(&fs::read(stored.path().join("metadata.json")).unwrap())
                .unwrap();
        assert!(metadata.complete);
        assert_eq!(metadata.exit_code, Some(1));
        assert_eq!(metadata.signal, None);
        assert_eq!(metadata.stdout_bytes, 13);
        assert_eq!(metadata.stderr_bytes, 11);
        assert_eq!(metadata.stdout_sha256.len(), 64);
        assert_eq!(metadata.stderr_sha256.len(), 64);
        assert!(metadata.raw_fallback);
        assert_eq!(metadata.schema_version, 2);
        assert_eq!(metadata.exit_code_parity, None);
        assert_eq!(metadata.fixture_preservation, None);
        assert_eq!(stored.metadata.run_id, metadata.run_id);
    }

    #[test]
    fn test_process_outcome_keeps_signal_separate_from_exit_code() {
        let outcome = ProcessOutcome::Signal(9);

        assert_eq!(outcome.exit_code(), None);
        assert_eq!(outcome.signal(), Some(9));
        assert_eq!(outcome.shell_exit_code(), 137);
    }

    #[test]
    fn test_capture_command_persists_both_streams_before_returning() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut run = start_run(&root);
        let mut command = if cfg!(windows) {
            let mut command = std::process::Command::new("powershell.exe");
            command.args([
                "-NoProfile",
                "-Command",
                "[Console]::Out.Write('stdout-line'); [Console]::Error.Write('stderr-line'); exit 7",
            ]);
            command
        } else {
            let mut command = std::process::Command::new("sh");
            command.args([
                "-c",
                "printf 'stdout-line'; printf 'stderr-line' >&2; exit 7",
            ]);
            command
        };

        let outcome = run.capture_command(&mut command, false).unwrap();

        assert_eq!(outcome, ProcessOutcome::ExitCode(7));
        assert_eq!(run.read_stdout().unwrap(), b"stdout-line".to_vec());
        assert_eq!(run.read_stderr().unwrap(), b"stderr-line".to_vec());
    }

    #[test]
    fn test_store_loads_run_by_validated_id_and_marks_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let store = RunStore::new(temp.path().canonicalize().unwrap());
        let run = store
            .start(RunStart {
                command: "adb devices".into(),
                cwd: std::path::PathBuf::from("/workspace/app"),
                profile: "android-only".into(),
                output_mode: "balanced".into(),
            })
            .unwrap();
        let id = run.id().clone();
        let stored = run
            .finalize(
                ProcessOutcome::ExitCode(0),
                "{\"schema_version\":1,\"events\":[]}",
                "one device",
                FinalizeDetails::default(),
            )
            .unwrap();

        let loaded = store.load(&id).unwrap();
        assert_eq!(loaded.path(), stored.path());
        assert_eq!(loaded.read_summary().unwrap(), "one device");
        assert!(!loaded.metadata.recovery_requested);

        store.mark_recovery_requested(&id).unwrap();
        assert!(store.load(&id).unwrap().metadata.recovery_requested);
    }

    fn create_dummy_run(root: &std::path::Path, id: &str, bytes: usize) -> std::path::PathBuf {
        let path = root
            .join(&id[0..4])
            .join(&id[4..6])
            .join(&id[6..8])
            .join(id);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("stdout.log"), vec![b'x'; bytes]).unwrap();
        path
    }

    #[test]
    fn test_prune_enforces_age_and_count_limits_oldest_first() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let store = RunStore::new(root.clone());
        let old = create_dummy_run(&root, "20260701T000000.000Z-aaaaaaaa", 10);
        let recent = create_dummy_run(&root, "20260714T000000.000Z-bbbbbbbb", 10);
        let newest = create_dummy_run(&root, "20260715T000000.000Z-cccccccc", 10);
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let report = store
            .prune_at(
                RetentionPolicy {
                    max_age_days: 7,
                    max_runs: 2,
                    max_bytes: 1_000,
                },
                now,
            )
            .unwrap();

        assert_eq!(report.removed_runs, 1);
        assert!(!old.exists());
        assert!(recent.exists());
        assert!(newest.exists());
    }

    #[test]
    fn test_prune_enforces_total_byte_limit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let store = RunStore::new(root.clone());
        let older = create_dummy_run(&root, "20260714T000000.000Z-aaaaaaaa", 600);
        let newest = create_dummy_run(&root, "20260715T000000.000Z-bbbbbbbb", 600);
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let report = store
            .prune_at(
                RetentionPolicy {
                    max_age_days: 7,
                    max_runs: 200,
                    max_bytes: 1_000,
                },
                now,
            )
            .unwrap();

        assert_eq!(report.removed_runs, 1);
        assert_eq!(report.kept_runs, 1);
        assert!(!older.exists());
        assert!(newest.exists());
    }
}
