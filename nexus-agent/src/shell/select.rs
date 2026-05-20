//! Host-OS-aware shell selection.
//!
//! - **Windows:** `powershell.exe -NoLogo` if on PATH; else `cmd.exe /Q`.
//! - **Linux / macOS / Other Unix:** `$SHELL` if set; else `/bin/bash` if it
//!   exists; else `/bin/sh`.

use std::path::{Path, PathBuf};

use nexus_common::OsKind;

/// A resolved shell invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    /// Program path or basename.
    pub program: String,
    /// Command-line arguments.
    pub args: Vec<String>,
}

impl ShellCommand {
    /// Parse a whitespace-delimited command line.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split_whitespace();
        let program = parts.next()?.to_string();
        let args = parts.map(str::to_string).collect();
        Some(Self { program, args })
    }
}

/// Stateless namespace for shell-selection helpers.
pub struct ShellSelect;

impl ShellSelect {
    /// Return the appropriate shell with an optional override.
    #[must_use]
    pub fn for_host_with_override(override_line: Option<&str>) -> ShellCommand {
        if let Some(line) = override_line {
            if let Some(cmd) = ShellCommand::parse(line) {
                return cmd;
            }
        }
        Self::for_host()
    }

    /// Return the appropriate shell for the current host OS.
    #[must_use]
    pub fn for_host() -> ShellCommand {
        match OsKind::detect() {
            OsKind::Windows => Self::for_windows(),
            OsKind::Linux | OsKind::MacOS | OsKind::Other => Self::for_unix(),
        }
    }

    fn for_windows() -> ShellCommand {
        if has_on_path("powershell.exe") {
            ShellCommand {
                program: "powershell.exe".into(),
                args: vec!["-NoLogo".into()],
            }
        } else {
            ShellCommand {
                program: "cmd.exe".into(),
                args: vec!["/Q".into()],
            }
        }
    }

    fn for_unix() -> ShellCommand {
        if let Some(shell) = std::env::var_os("SHELL") {
            let path = PathBuf::from(&shell);
            if !path.as_os_str().is_empty() {
                return ShellCommand {
                    program: shell.to_string_lossy().into_owned(),
                    args: vec![],
                };
            }
        }
        if Path::new("/bin/bash").exists() {
            return ShellCommand {
                program: "/bin/bash".into(),
                args: vec![],
            };
        }
        ShellCommand {
            program: "/bin/sh".into(),
            args: vec![],
        }
    }
}

fn has_on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let cmd = ShellCommand::parse("/bin/zsh").expect("parse");
        assert_eq!(cmd.program, "/bin/zsh");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn parse_with_args() {
        let cmd = ShellCommand::parse("/usr/bin/env bash -l").expect("parse");
        assert_eq!(cmd.program, "/usr/bin/env");
        assert_eq!(cmd.args, vec!["bash".to_string(), "-l".to_string()]);
    }

    #[test]
    fn for_host_returns_non_empty_program() {
        let cmd = ShellSelect::for_host();
        assert!(!cmd.program.is_empty(), "no shell selected");
    }

    #[test]
    fn override_takes_precedence() {
        let cmd = ShellSelect::for_host_with_override(Some("/bin/cat"));
        assert_eq!(cmd.program, "/bin/cat");
    }

    #[test]
    fn blank_override_falls_back_to_host() {
        let a = ShellSelect::for_host_with_override(Some(""));
        let b = ShellSelect::for_host();
        assert_eq!(a, b);
    }
}
