//! `audit_verify` — v1.2 CLI that walks the BLAKE3-chained audit log
//! produced by [`nexus_a2a::audit::FileSink`] and reports tampering.
//!
//! Usage:
//!
//! ```text
//! audit_verify ~/.nexus/audit.log
//! ```
//!
//! Exit code 0 on intact chain, 1 on tampering / parse error.

use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use nexus_a2a::audit::{verify_chain, AuditRecord};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: audit_verify <path-to-audit.log>");
        return ExitCode::from(2);
    };

    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("audit_verify: open {path}: {err}");
            return ExitCode::from(1);
        }
    };
    let reader = BufReader::new(file);

    let mut records: Vec<AuditRecord> = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(err) => {
                eprintln!("audit_verify: read line {i}: {err}");
                return ExitCode::from(1);
            }
        };
        let rec: AuditRecord = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("audit_verify: parse line {i}: {err}");
                return ExitCode::from(1);
            }
        };
        records.push(rec);
    }

    match verify_chain(&records) {
        Ok(()) => {
            println!("audit_verify: OK ({} records)", records.len());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("audit_verify: TAMPERED ({err})");
            ExitCode::from(1)
        }
    }
}
