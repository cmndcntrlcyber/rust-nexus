//! `/proc` filesystem observer (WS1.5 Phase 1.5c).
//!
//! Captures kernel-level context for a process by reading its `/proc`
//! entries. No elevated permissions are required — unprivileged reads
//! of `/proc/{pid}/status`, `/proc/{pid}/fd/`, `/proc/{pid}/net/tcp`,
//! etc. All reads are best-effort: if any file is unreadable the
//! observer returns partial data rather than failing.

use nexus_common::kernel_context::KernelContext;

/// Reads `/proc/{pid}/...` files to build a [`KernelContext`] snapshot.
pub struct ProcObserver;

impl ProcObserver {
    /// Create a new observer.
    pub fn new() -> Self {
        Self
    }

    /// Observe the given PID and return a [`KernelContext`] populated
    /// from the proc filesystem. Fields that could not be read are
    /// left at their default (zero) values.
    pub fn observe(&self, pid: u32) -> KernelContext {
        let mut ctx = KernelContext::default();

        // /proc/{pid}/status  -> VmRSS, VmSize, Threads
        self.read_status(pid, &mut ctx);

        // /proc/{pid}/fd/ -> open file descriptor count
        self.read_fd_count(pid, &mut ctx);

        // /proc/{pid}/net/tcp + udp -> network connection count
        self.read_net_connections(pid, &mut ctx);

        // /proc/{pid}/maps -> memory region count (as files_touched proxy)
        self.read_maps(pid, &mut ctx);

        ctx
    }

    // ── /proc/{pid}/status ───────────────────────────────────────

    fn read_status(&self, pid: u32, ctx: &mut KernelContext) {
        let path = format!("/proc/{}/status", pid);
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(path = %path, error = %e, "failed to read proc status");
                return;
            }
        };

        for line in contents.lines() {
            if let Some(value) = line.strip_prefix("VmRSS:") {
                ctx.memory_allocated_bytes = parse_kb_value(value);
            } else if let Some(value) = line.strip_prefix("Threads:") {
                if let Ok(t) = value.trim().parse::<u32>() {
                    ctx.threads_created = t;
                }
            } else if let Some(value) = line.strip_prefix("voluntary_ctxt_switches:") {
                if let Ok(v) = value.trim().parse::<u64>() {
                    // Use context switches as a rough proxy for syscall count.
                    ctx.syscall_count += v;
                }
            } else if let Some(value) = line.strip_prefix("nonvoluntary_ctxt_switches:") {
                if let Ok(v) = value.trim().parse::<u64>() {
                    ctx.syscall_count += v;
                }
            }
        }
    }

    // ── /proc/{pid}/fd/ ──────────────────────────────────────────

    fn read_fd_count(&self, pid: u32, ctx: &mut KernelContext) {
        let path = format!("/proc/{}/fd", pid);
        match std::fs::read_dir(&path) {
            Ok(entries) => {
                let count = entries.count() as u32;
                ctx.files_touched = count;
            }
            Err(e) => {
                tracing::debug!(path = %path, error = %e, "failed to read fd dir");
            }
        }
    }

    // ── /proc/{pid}/net/tcp + udp ────────────────────────────────

    fn read_net_connections(&self, pid: u32, ctx: &mut KernelContext) {
        let mut total: u32 = 0;

        for proto in &["tcp", "udp", "tcp6", "udp6"] {
            let path = format!("/proc/{}/net/{}", pid, proto);
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    // First line is a header; each subsequent line is a connection.
                    let count = contents.lines().skip(1).count();
                    total = total.saturating_add(count as u32);
                }
                Err(e) => {
                    tracing::debug!(path = %path, error = %e, "failed to read net file");
                }
            }
        }

        ctx.network_connections = total;
    }

    // ── /proc/{pid}/maps ─────────────────────────────────────────

    fn read_maps(&self, pid: u32, ctx: &mut KernelContext) {
        let path = format!("/proc/{}/maps", pid);
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                // Count process_events as a proxy — each mapped region is a
                // process-level event. We add them as processes_spawned for
                // the memory-region snapshot count.
                let regions = contents.lines().count() as u32;
                // Store region count in processes_spawned as a reasonable
                // aggregate field; the full process_events vec gives detail.
                ctx.processes_spawned = regions;
            }
            Err(e) => {
                tracing::debug!(path = %path, error = %e, "failed to read maps");
            }
        }
    }
}

impl Default for ProcObserver {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a value like `"  12345 kB"` into bytes.
fn parse_kb_value(s: &str) -> u64 {
    let trimmed = s.trim();
    // Strip trailing "kB" or "KB" and parse.
    let numeric = trimmed
        .trim_end_matches("kB")
        .trim_end_matches("KB")
        .trim();
    match numeric.parse::<u64>() {
        Ok(kb) => kb * 1024,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kb_value_works() {
        assert_eq!(parse_kb_value("  12345 kB"), 12345 * 1024);
        assert_eq!(parse_kb_value("0 kB"), 0);
        assert_eq!(parse_kb_value(""), 0);
        assert_eq!(parse_kb_value("not_a_number kB"), 0);
    }

    #[test]
    fn observe_self() {
        let observer = ProcObserver::new();
        let ctx = observer.observe(std::process::id());
        // On any Linux system, the current process should have at
        // least one thread and some memory allocated.
        // If not on Linux, the fields will be zero (graceful fallback).
        if cfg!(target_os = "linux") {
            assert!(ctx.memory_allocated_bytes > 0, "expected nonzero VmRSS");
            assert!(ctx.threads_created > 0, "expected at least 1 thread");
        }
    }
}
