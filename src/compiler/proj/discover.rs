use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn project_entry_main_path() -> PathBuf {
    let default_main = PathBuf::from("src/main.ax");
    let Ok(build_ax) = std::fs::read_to_string("build.ax") else {
        return default_main;
    };
    for line in build_ax.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("main:") else {
            continue;
        };
        let mut value = rest.trim();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = &value[1..value.len() - 1];
        }
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    default_main
}

fn project_entry_root_path() -> PathBuf {
    let main = project_entry_main_path();
    let resolved = if main.is_absolute() {
        main
    } else {
        PathBuf::from(".").join(main)
    };
    resolved
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn is_project_ax_source(path: &Path) -> bool {
    if path.extension().and_then(|e| e.to_str()) != Some("ax") {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    !name.ends_with(".test.ax")
}

fn normalize_root(path: &str) -> PathBuf {
    if path.is_empty() || path == "." {
        PathBuf::from(".")
    } else {
        PathBuf::from(path)
    }
}

fn walk_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: discover: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("error: discover: bad dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            walk_ax_files(&path, out)?;
            continue;
        }
        if is_project_ax_source(&path) {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn collect_all_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: ir: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("error: ir: bad dir entry: {e}"))?
            .path();
        if path.is_dir() {
            collect_all_ax_files(&path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("ax") {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[axon_export]
fn project_entry_root() -> String {
    project_entry_root_path().to_string_lossy().to_string()
}

#[axon_export]
fn discover_entry() -> String {
    "./src/main.ax".to_string()
}

#[axon_export]
fn list_ax_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = walk_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    files.join("\n")
}

#[axon_export]
fn read_source_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("error: discover: cannot read {path}: {e}"),
    }
}

#[axon_export]
fn string_char_at(s: &str, index: i64) -> String {
    let bytes = s.as_bytes();
    let i = index as usize;
    if i >= bytes.len() {
        return String::new();
    }
    let b = bytes[i];
    if b.is_ascii() {
        (b as char).to_string()
    } else {
        let ch = s.chars().nth(i).unwrap_or('\0');
        ch.to_string()
    }
}

#[axon_export]
fn string_byte_at(s: &str, index: i64) -> i64 {
    let bytes = s.as_bytes();
    let i = index as usize;
    if i >= bytes.len() {
        -1
    } else {
        bytes[i] as i64
    }
}

#[axon_export]
fn string_from_char(code: i64) -> String {
    if let Some(ch) = char::from_u32(code as u32) {
        ch.to_string()
    } else {
        String::new()
    }
}

#[axon_export]
fn string_starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

#[axon_export]
fn string_ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

#[axon_export]
fn string_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[axon_export]
fn string_sub(s: &str, start: i64, len: i64) -> String {
    let start = start as usize;
    let len = len as usize;
    if start >= s.len() {
        return String::new();
    }
    let end = (start + len).min(s.len());
    s[start..end].to_string()
}

#[axon_export]
fn string_split(haystack: &str, needle: &str) -> String {
    let parts: Vec<&str> = haystack.split(needle).collect();
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push('\x1f');
        }
        out.push_str(part);
    }
    out
}

#[axon_export]
fn string_count(haystack: &str, needle: &str) -> i64 {
    haystack.matches(needle).count() as i64
}

#[axon_export]
fn string_trim(s: &str) -> String {
    s.trim().to_string()
}

#[axon_export]
fn string_eq(a: &str, b: &str) -> bool {
    a == b
}

#[axon_export]
fn path_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[axon_export]
fn path_is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}
