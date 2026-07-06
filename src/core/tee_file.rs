//! Legacy file-based recovery ("tee" mode) — may be deprecated. Prefer the
//! sqlite recall store (`[retriever] mode = "sqlite"`); see retriever.rs.

use crate::core::config::Config;
use crate::core::constants::RTK_DATA_DIR;
use crate::core::retriever::RetrieverConfig;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn sanitize_slug(slug: &str) -> String {
    let sanitized: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.len() > 40 {
        sanitized[..40].to_string()
    } else {
        sanitized
    }
}

fn get_tee_dir(cfg: &RetrieverConfig) -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("RTK_TEE_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(ref dir) = cfg.tee_directory {
        return Some(dir.clone());
    }
    dirs::data_local_dir().map(|d| d.join(RTK_DATA_DIR).join("tee"))
}

fn cleanup_old_files(dir: &Path, max_files: usize) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();
    if entries.len() <= max_files {
        return;
    }
    entries.sort_by_key(|e| e.file_name());
    let to_remove = entries.len() - max_files;
    for entry in entries.iter().take(to_remove) {
        let _ = std::fs::remove_file(entry.path());
    }
}

fn write_tee_file(
    raw: &str,
    slug: &str,
    dir: &Path,
    max_file_size: usize,
    max_files: usize,
) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    let slug = sanitize_slug(slug);
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let filepath = dir.join(format!("{}_{}.log", epoch, slug));
    let content = if raw.len() > max_file_size {
        let boundary = raw
            .char_indices()
            .take_while(|(i, _)| *i < max_file_size)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!(
            "{}\n\n--- truncated at {} bytes ---",
            &raw[..boundary],
            max_file_size
        )
    } else {
        raw.to_string()
    };
    std::fs::write(&filepath, content).ok()?;
    cleanup_old_files(dir, max_files);
    Some(filepath)
}

fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn write(content: &str, slug: &str) -> Option<PathBuf> {
    let cfg = Config::load().ok()?.retriever;
    let dir = get_tee_dir(&cfg)?;
    write_tee_file(
        content,
        slug,
        &dir,
        cfg.tee_max_file_size,
        cfg.tee_max_files,
    )
}

pub fn tee_and_hint(raw: &str, slug: &str) -> Option<String> {
    let path = write(raw, slug)?;
    Some(format!("[full output: {}]", display_path(&path)))
}

pub fn force_tee_hint(content: &str, slug: &str) -> Option<String> {
    let path = write(content, slug)?;
    Some(format!("[full output: {}]", display_path(&path)))
}

pub fn force_tee_tail_hint(content: &str, slug: &str, line_offset: usize) -> Option<String> {
    let path = write(content, slug)?;
    Some(format!(
        "[see remaining: tail -n +{} {}]",
        line_offset,
        display_path(&path)
    ))
}
