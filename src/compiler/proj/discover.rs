use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Parses `build.ax` looking for a `main:` directive to determine the project
/// entry point. Returns `src/main.ax` if no directive is found.
/// This is config-file reading, not compiler logic.
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

/// Resolves the parent directory of the project entry file.
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

/// Returns true if `path` is a `.ax` file but NOT a `.test.ax` test file.
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

/// Recursively walks `root`, collecting non-test `.ax` file paths into `out`.
fn discover_walk_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: discover: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("error: discover: bad dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            discover_walk_ax_files(&path, out)?;
            continue;
        }
        if is_project_ax_source(&path) {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

/// Recursively walks `root`, collecting ALL `.ax` file paths (including tests).
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

/// FFI: Returns the directory containing the project entry file.
/// Axon callers use this to locate the source root.
#[axon_export]
fn project_entry_root() -> String {
    project_entry_root_path().to_string_lossy().to_string()
}

/// FFI: Returns the default entry point path (`./src/main.ax`).
#[axon_export]
fn discover_entry() -> String {
    project_entry_main_path().to_string_lossy().into_owned()
}

/// FFI: Lists all non-test `.ax` files under `root`.
/// Returns newline-separated file paths, sorted. On error returns a
/// string starting with `error:`.
#[axon_export]
fn list_ax_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = discover_walk_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    files.join("\n")
}

/// FFI: Concatenates two strings. Used by Axon where `+` on strings
/// is not yet available.
#[axon_export]
fn string_concat(a: &str, b: &str) -> String {
    let mut result = String::with_capacity(a.len() + b.len());
    result.push_str(a);
    result.push_str(b);
    result
}

/// FFI: Reads a source file. On success returns file contents;
/// on failure returns a string starting with `error:`.
#[axon_export]
fn read_source_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("error: discover: cannot read {path}: {e}"),
    }
}

/// FFI: Returns the character at byte-offset `index` in `s`.
/// Returns empty string if out of bounds.
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

/// FFI: Returns the byte value at `index` in `s`, or -1 if out of bounds.
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

/// FFI: Converts a Unicode code point (i64) to a single-char String.
/// Returns empty string for invalid code points.
#[axon_export]
fn string_from_char(code: i64) -> String {
    if let Some(ch) = char::from_u32(code as u32) {
        ch.to_string()
    } else {
        String::new()
    }
}

/// FFI: Returns true if `s` starts with `prefix`.
#[axon_export]
fn string_starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// FFI: Returns true if `s` ends with `suffix`.
#[axon_export]
fn string_ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// FFI: Returns true if `haystack` contains `needle`.
#[axon_export]
fn string_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// FFI: Returns a substring of `s` starting at byte-offset `start`
/// with byte-length `len`. Returns empty string if start is out of bounds.
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

/// FFI: Splits `haystack` by `needle`, joining parts with `\x1f` (unit separator).
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

/// FFI: Counts non-overlapping occurrences of `needle` in `haystack`.
#[axon_export]
fn string_count(haystack: &str, needle: &str) -> i64 {
    haystack.matches(needle).count() as i64
}

/// FFI: Returns `s` with leading/trailing whitespace removed.
#[axon_export]
fn string_trim(s: &str) -> String {
    s.trim().to_string()
}

/// FFI: Returns true if strings `a` and `b` are equal.
#[axon_export]
fn string_eq(a: &str, b: &str) -> bool {
    a == b
}

/// FFI: Returns true if the filesystem path exists.
#[axon_export]
fn path_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// FFI: Returns true if the filesystem path is a directory.
#[axon_export]
fn path_is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// FFI: Lists ALL `.ax` files under `root` (including `.test.ax`).
/// Returns newline-separated file paths, sorted.
#[axon_export]
fn list_all_ax_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let mut files: Vec<String> = Vec::new();
    if let Err(err) = collect_all_ax_files(&root_path, &mut files) {
        return err;
    }
    files.sort();
    files.join("\n")
}

/// Recursively walks `root`, collecting `.test.ax` files into `out`.
fn discover_walk_test_ax_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: discover: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("error: discover: bad dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            discover_walk_test_ax_files(&path, out)?;
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if name.ends_with(".test.ax") {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

/// Recursively walks `root`, collecting `.ax` files under `tests/` directories.
fn discover_walk_tests_dir_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        collect_all_ax_files(&tests_dir, out)?;
    }
    Ok(())
}

/// FFI: Lists test files under `root`: `.test.ax` files under `src/` plus
/// `.ax` files under `tests/`. Returns newline-separated, sorted.
#[axon_export]
fn list_test_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let mut files: Vec<String> = Vec::new();
    let _ = discover_walk_test_ax_files(&root_path.join("src"), &mut files);
    let _ = discover_walk_tests_dir_files(&root_path, &mut files);
    files.sort();
    files.join("\n")
}

/// Recursively walks `root`, collecting `.rs` and `.go` sidecar files.
fn discover_walk_sidecar_files(root: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("error: discover: cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("error: discover: bad dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            discover_walk_sidecar_files(&path, out)?;
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "rs" || ext == "go" {
            out.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

/// FFI: Lists sidecar files (`.rs` and `.go`) under `root/src/`.
/// Returns newline-separated, sorted.
#[axon_export]
fn list_sidecar_files(root: &str) -> String {
    let root_path = normalize_root(root);
    let src_path = root_path.join("src");
    let mut files: Vec<String> = Vec::new();
    if src_path.is_dir() {
        if let Err(err) = discover_walk_sidecar_files(&src_path, &mut files) {
            return err;
        }
    }
    files.sort();
    files.join("\n")
}

/// FFI: Canonicalizes a filesystem path. Returns the resolved path or "error:...".
#[axon_export]
fn path_canonicalize(path: &str) -> String {
    match std::path::Path::new(path).canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => format!("error: {}", e),
    }
}
