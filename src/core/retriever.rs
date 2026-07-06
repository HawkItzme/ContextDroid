//! Content-addressed recall store backing `rtk recall`.

use super::constants::{RECALL_DB, RTK_DATA_DIR};
use crate::core::config::Config;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_ENTRY_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_MAX_ENTRIES: usize = 200;
const DEFAULT_RETENTION_DAYS: u32 = 30;
pub const MIN_FAILURE_BYTES: usize = 500;
const HASH_HEX_LEN: usize = 12;
const DEFAULT_TEE_MAX_FILES: usize = 20;
const DEFAULT_TEE_MAX_FILE_SIZE: usize = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryMode {
    #[default]
    Sqlite,
    Tee,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetrieverConfig {
    pub mode: RecoveryMode,
    pub max_entry_bytes: usize,
    pub max_entries: usize,
    pub retention_days: u32,
    pub compression: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_path: Option<PathBuf>,
    pub tee_max_files: usize,
    pub tee_max_file_size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tee_directory: Option<PathBuf>,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        Self {
            mode: RecoveryMode::Sqlite,
            max_entry_bytes: DEFAULT_MAX_ENTRY_BYTES,
            max_entries: DEFAULT_MAX_ENTRIES,
            retention_days: DEFAULT_RETENTION_DAYS,
            compression: true,
            database_path: None,
            tee_max_files: DEFAULT_TEE_MAX_FILES,
            tee_max_file_size: DEFAULT_TEE_MAX_FILE_SIZE,
            tee_directory: None,
        }
    }
}

pub struct StoredRef {
    pub hash: String,
    pub hidden_lines: usize,
}

pub enum Stored {
    Saved(StoredRef),
    Unavailable,
    Empty,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn content_hash(command: &str, content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(command.as_bytes());
    hasher.update([0u8]);
    hasher.update(content);
    let hex = format!("{:x}", hasher.finalize());
    hex[..HASH_HEX_LEN].to_string()
}

fn count_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
    if *bytes.last().unwrap() == b'\n' {
        newlines
    } else {
        newlines + 1
    }
}

fn slice_from_line(bytes: &[u8], from: usize) -> &[u8] {
    if from <= 1 {
        return bytes;
    }
    let mut seen = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            seen += 1;
            if seen == from - 1 {
                return &bytes[i + 1..];
            }
        }
    }
    &[]
}

fn slice_first_lines(bytes: &[u8], n: usize) -> &[u8] {
    if n == 0 {
        return &[];
    }
    let mut seen = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            seen += 1;
            if seen == n {
                return &bytes[..=i];
            }
        }
    }
    bytes
}

fn grep_bytes(input: &[u8], pattern: &str) -> Vec<u8> {
    use regex::bytes::Regex;
    let re = Regex::new(pattern)
        .or_else(|_| Regex::new(&regex::escape(pattern)))
        .ok();
    let Some(re) = re else {
        return input.to_vec();
    };
    let mut out = Vec::new();
    for line in input.split(|&b| b == b'\n') {
        if re.is_match(line) {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }
    out
}

fn gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).context("gzip write")?;
    enc.finish().context("gzip finish")
}

fn gunzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    GzDecoder::new(data)
        .read_to_end(&mut out)
        .context("gunzip")?;
    Ok(out)
}

fn db_path(cfg: &RetrieverConfig) -> Result<PathBuf> {
    if let Ok(p) = std::env::var("RTK_RECALL_DB") {
        return Ok(PathBuf::from(p));
    }
    if let Some(ref p) = cfg.database_path {
        return Ok(p.clone());
    }
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    Ok(data_dir.join(RTK_DATA_DIR).join(RECALL_DB))
}

fn open(cfg: &RetrieverConfig) -> Result<Connection> {
    let path = db_path(cfg)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
    // best-effort: NFS / read-only filesystems may reject WAL
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;");
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS recall (
            hash        TEXT PRIMARY KEY,
            command     TEXT NOT NULL,
            cwd         TEXT,
            exit_code   INTEGER,
            created_at  INTEGER NOT NULL,
            total_lines INTEGER NOT NULL,
            shown_upto  INTEGER NOT NULL,
            byte_size   INTEGER NOT NULL,
            truncated   INTEGER NOT NULL,
            codec       TEXT NOT NULL,
            blob        BLOB NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_recall_command ON recall(command, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_recall_created ON recall(created_at);",
    )
    .context("init recall schema")
}

fn evict(conn: &Connection, cfg: &RetrieverConfig) {
    if cfg.retention_days > 0 {
        let cutoff = now_secs() - (cfg.retention_days as i64) * 86_400;
        let _ = conn.execute("DELETE FROM recall WHERE created_at < ?1", params![cutoff]);
    }
    if cfg.max_entries > 0 {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM recall", [], |r| r.get(0))
            .unwrap_or(0);
        let excess = count - cfg.max_entries as i64;
        if excess > 0 {
            let _ = conn.execute(
                "DELETE FROM recall WHERE hash IN (
                    SELECT hash FROM recall ORDER BY created_at ASC, hash ASC LIMIT ?1
                )",
                params![excess],
            );
        }
    }
}

pub fn store(content: &[u8], command: &str, exit_code: i32, shown_upto: usize) -> Stored {
    if content.is_empty() {
        return Stored::Empty;
    }
    let Ok(config) = Config::load() else {
        return Stored::Unavailable;
    };
    match store_inner(
        &config.retriever,
        content,
        command,
        exit_code,
        shown_upto.max(1),
    ) {
        Ok(r) => Stored::Saved(r),
        Err(_) => Stored::Unavailable,
    }
}

fn store_inner(
    cfg: &RetrieverConfig,
    content: &[u8],
    command: &str,
    exit_code: i32,
    shown_upto: usize,
) -> Result<StoredRef> {
    let total_lines = count_lines(content);
    let (payload, truncated) = if content.len() > cfg.max_entry_bytes {
        (&content[..cfg.max_entry_bytes], true)
    } else {
        (content, false)
    };
    let hash = content_hash(command, content);
    let (blob, codec): (Vec<u8>, &str) = if cfg.compression {
        match gzip(payload) {
            Ok(z) => (z, "gzip"),
            Err(_) => (payload.to_vec(), "raw"),
        }
    } else {
        (payload.to_vec(), "raw")
    };
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().into_owned());

    let conn = open(cfg)?;
    conn.execute(
        "INSERT OR REPLACE INTO recall
         (hash, command, cwd, exit_code, created_at, total_lines, shown_upto, byte_size, truncated, codec, blob)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            hash,
            command,
            cwd,
            exit_code,
            now_secs(),
            total_lines as i64,
            shown_upto as i64,
            content.len() as i64,
            truncated as i64,
            codec,
            blob
        ],
    )
    .context("insert recall row")?;
    evict(&conn, cfg);

    Ok(StoredRef {
        hash,
        hidden_lines: total_lines.saturating_sub(shown_upto.saturating_sub(1)),
    })
}

struct Row {
    shown_upto: usize,
    truncated: bool,
    codec: String,
    blob: Vec<u8>,
}

fn map_row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        shown_upto: r.get::<_, i64>(0)? as usize,
        truncated: r.get::<_, i64>(1)? != 0,
        codec: r.get(2)?,
        blob: r.get(3)?,
    })
}

const SELECT_COLS: &str = "shown_upto, truncated, codec, blob";

fn load_by_hash(conn: &Connection, hash: &str) -> Result<Option<Row>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM recall WHERE hash = ?1 OR hash LIKE ?1 || '%' \
         ORDER BY (hash = ?1) DESC, length(hash) ASC LIMIT 1"
    );
    Ok(conn.query_row(&sql, params![hash], map_row).optional()?)
}

fn load_latest_by_command(conn: &Connection, command: &str) -> Result<Option<Row>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM recall WHERE command = ?1 OR command LIKE '%' || ?1 || '%' \
         ORDER BY created_at DESC LIMIT 1"
    );
    Ok(conn.query_row(&sql, params![command], map_row).optional()?)
}

fn decode(row: &Row) -> Result<Vec<u8>> {
    match row.codec.as_str() {
        "gzip" => gunzip(&row.blob),
        _ => Ok(row.blob.clone()),
    }
}

pub struct RecallArgs<'a> {
    pub hash: Option<&'a str>,
    pub command: Option<&'a str>,
    pub full: bool,
    pub from: Option<usize>,
    pub lines: Option<usize>,
    pub grep: Option<&'a str>,
    pub list: bool,
}

pub fn run_recall(args: RecallArgs) -> Result<i32> {
    let cfg = Config::load().unwrap_or_default().retriever;
    let conn = match open(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rtk recall: store unavailable: {e}");
            return Ok(1);
        }
    };

    if args.list {
        return list_entries(&conn);
    }

    let row = match (args.hash, args.command) {
        (Some(h), _) => load_by_hash(&conn, h)?,
        (None, Some(c)) => load_latest_by_command(&conn, c)?,
        (None, None) => {
            eprintln!("rtk recall: provide a <hash>, --command <cmd>, or --list");
            return Ok(2);
        }
    };

    let Some(row) = row else {
        eprintln!("rtk recall: no matching entry (try `rtk recall --list`)");
        return Ok(1);
    };

    let full = decode(&row)?;
    let sliced: Vec<u8> = if args.full {
        full.clone()
    } else if let Some(n) = args.from {
        slice_from_line(&full, n).to_vec()
    } else if let Some(n) = args.lines {
        slice_first_lines(&full, n).to_vec()
    } else {
        slice_from_line(&full, row.shown_upto).to_vec()
    };
    let out = match args.grep {
        Some(pat) => grep_bytes(&sliced, pat),
        None => sliced,
    };

    let stdout = std::io::stdout();
    let _ = stdout.lock().write_all(&out);

    if row.truncated && args.full {
        eprintln!(
            "rtk recall: note: output exceeded the {}-byte cap and was stored truncated",
            cfg.max_entry_bytes
        );
    }
    Ok(0)
}

fn list_entries(conn: &Connection) -> Result<i32> {
    let mut stmt = conn.prepare(
        "SELECT hash, command, total_lines, shown_upto, exit_code, truncated \
         FROM recall ORDER BY created_at DESC LIMIT 50",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;

    println!(
        "{:<14} {:<26} {:>7} {:>7} {:>5} TRUNC",
        "HASH", "COMMAND", "LINES", "HIDDEN", "EXIT"
    );
    let mut n = 0;
    for row in rows {
        let (hash, command, total, shown, exit, truncated) = row?;
        let hidden = total.saturating_sub(shown.saturating_sub(1)).max(0);
        let cmd = if command.chars().count() > 26 {
            let head: String = command.chars().take(25).collect();
            format!("{head}…")
        } else {
            command
        };
        println!(
            "{:<14} {:<26} {:>7} {:>7} {:>5} {}",
            hash,
            cmd,
            total,
            hidden,
            exit.map(|e| e.to_string()).unwrap_or_else(|| "-".into()),
            if truncated != 0 { "yes" } else { "" }
        );
        n += 1;
    }
    if n == 0 {
        println!("(no recall entries)");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cfg(dir: &std::path::Path) -> RetrieverConfig {
        RetrieverConfig {
            database_path: Some(dir.join("recall_test.db")),
            ..RetrieverConfig::default()
        }
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"abc"), 1);
        assert_eq!(count_lines(b"a\nb\nc"), 3);
        assert_eq!(count_lines(b"a\nb\nc\n"), 3);
        assert_eq!(count_lines(b"\n"), 1);
    }

    #[test]
    fn test_slice_from_line() {
        let b = b"l1\nl2\nl3\n";
        assert_eq!(slice_from_line(b, 1), b);
        assert_eq!(slice_from_line(b, 2), b"l2\nl3\n");
        assert_eq!(slice_from_line(b, 3), b"l3\n");
        assert_eq!(slice_from_line(b, 4), b"");
        assert_eq!(slice_from_line(b, 99), b"");
    }

    #[test]
    fn test_slice_first_lines() {
        let b = b"l1\nl2\nl3\n";
        assert_eq!(slice_first_lines(b, 0), b"");
        assert_eq!(slice_first_lines(b, 1), b"l1\n");
        assert_eq!(slice_first_lines(b, 2), b"l1\nl2\n");
        assert_eq!(slice_first_lines(b, 99), b);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let a = content_hash("cmd", b"output");
        assert_eq!(a, content_hash("cmd", b"output"));
        assert_eq!(a.len(), HASH_HEX_LEN);
        assert_ne!(a, content_hash("cmd2", b"output"));
        assert_ne!(a, content_hash("cmd", b"output2"));
    }

    #[test]
    fn test_grep_bytes() {
        let input = b"alpha\nbeta\ngamma\n";
        assert_eq!(grep_bytes(input, "et"), b"beta\n");
        assert_eq!(grep_bytes(input, "^g"), b"gamma\n");
    }

    #[test]
    fn test_gzip_roundtrip_arbitrary_bytes() {
        let cases: Vec<Vec<u8>> = vec![
            b"hello\n".to_vec(),
            vec![0xff, 0xfe, 0x00, 0x01, 0x80],
            b"crlf\r\nline\r\n".to_vec(),
            b"lone\rcr".to_vec(),
            "emoji😀漢字".as_bytes().to_vec(),
            b"no trailing newline".to_vec(),
            (0u8..=255).collect(),
        ];
        for c in cases {
            let z = gzip(&c).expect("gzip");
            assert_eq!(gunzip(&z).expect("gunzip"), c, "gzip must be byte-exact");
        }
    }

    #[test]
    fn test_store_fetch_byte_faithful() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = temp_cfg(dir.path());
        let mut nasty = Vec::new();
        nasty.extend_from_slice(b"line1\r\n");
        nasty.extend_from_slice(&[0xff, 0x00, 0xfe]);
        nasty.extend_from_slice("漢字\n".as_bytes());
        nasty.extend_from_slice(b"no-eol-tail");

        let stored = store_inner(&cfg, &nasty, "nasty-cmd", 0, 1).expect("store");
        let conn = open(&cfg).unwrap();
        let row = load_by_hash(&conn, &stored.hash).unwrap().expect("row");
        assert_eq!(
            decode(&row).unwrap(),
            nasty,
            "stored bytes must round-trip exactly"
        );
    }

    #[test]
    fn test_store_fetch_raw_codec() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetrieverConfig {
            compression: false,
            ..temp_cfg(dir.path())
        };
        let data = vec![0u8, 1, 2, 255, b'\n', b'x'];
        let stored = store_inner(&cfg, &data, "c", 0, 1).unwrap();
        let conn = open(&cfg).unwrap();
        let row = load_by_hash(&conn, &stored.hash).unwrap().unwrap();
        assert_eq!(row.codec, "raw");
        assert_eq!(decode(&row).unwrap(), data);
    }

    #[test]
    fn test_delta_recall_returns_only_missed() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = temp_cfg(dir.path());
        let content = b"i1\ni2\ni3\ni4\ni5\n";
        let stored = store_inner(&cfg, content, "list", 0, 3).unwrap();
        assert_eq!(stored.hidden_lines, 3);
        let conn = open(&cfg).unwrap();
        let row = load_by_hash(&conn, &stored.hash).unwrap().unwrap();
        let full = decode(&row).unwrap();
        assert_eq!(slice_from_line(&full, row.shown_upto), b"i3\ni4\ni5\n");
    }

    #[test]
    fn test_truncation_cap_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetrieverConfig {
            max_entry_bytes: 10,
            ..temp_cfg(dir.path())
        };
        let big = vec![b'a'; 100];
        let stored = store_inner(&cfg, &big, "big", 0, 1).unwrap();
        let conn = open(&cfg).unwrap();
        let row = load_by_hash(&conn, &stored.hash).unwrap().unwrap();
        assert!(row.truncated);
        assert_eq!(decode(&row).unwrap().len(), 10);
    }

    #[test]
    fn test_fifo_count_eviction() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetrieverConfig {
            max_entries: 3,
            retention_days: 0,
            ..temp_cfg(dir.path())
        };
        for i in 0..5 {
            let content = format!("output-{i}");
            store_inner(&cfg, content.as_bytes(), &format!("cmd{i}"), 0, 1).unwrap();
        }
        let conn = open(&cfg).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM recall", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3, "FIFO cap should retain only max_entries");
    }

    #[test]
    fn test_dedup_same_content_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = temp_cfg(dir.path());
        let a = store_inner(&cfg, b"same output\n", "cmd", 0, 1).unwrap();
        let b = store_inner(&cfg, b"same output\n", "cmd", 0, 1).unwrap();
        assert_eq!(a.hash, b.hash);
        let conn = open(&cfg).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM recall", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "identical output must dedupe to one row");
    }

    #[test]
    fn test_load_by_hash_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = temp_cfg(dir.path());
        let stored = store_inner(&cfg, b"hello world\n", "cmd", 0, 1).unwrap();
        let conn = open(&cfg).unwrap();
        let prefix = &stored.hash[..6];
        assert!(load_by_hash(&conn, prefix).unwrap().is_some());
    }
}
