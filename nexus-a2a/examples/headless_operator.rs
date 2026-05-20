//! `headless_operator` — small CLI that dials a running C2 server, opens a
//! shell-session against the first registered agent, runs `whoami`, exits 0
//! if the marker shows up in the response stream within 10s.
//!
//! Used by `scripts/demo.sh` to produce a PASS/FAIL signal.

use std::process::ExitCode;
use std::time::Duration;

use futures::StreamExt;
use nexus_a2a::framing::{bytes_request, control_request, ShellControl};
use nexus_a2a::pb;
use nexus_a2a::A2aClient;
use tokio::time::{timeout, Instant};

const USAGE: &str = "\
headless_operator — v1.1 demo client

OPTIONS:
    --c2 <ADDR>             C2 gRPC URL (default http://127.0.0.1:50051)
    --insecure-network      Allow non-loopback addresses
    --target-agent <HEX>    Hex peer id of the agent to target
    --probe <STR>           Shell command (default: \"whoami\\n\")
    --marker <STR>          Substring to wait for in the response
    --timeout-secs <N>      Total time budget (default 10)
    -h, --help              Print this help
";

#[tokio::main]
async fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .try_init();

    let args = match parse_args() {
        Ok(a) => a,
        Err(err) => {
            eprintln!("headless_operator: {err}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let probe_marker = args
        .marker
        .clone()
        .unwrap_or_else(|| default_marker(&args.probe));

    let mut client = match A2aClient::connect(&args.c2_addr, args.insecure_network).await {
        Ok(c) => c,
        Err(err) => {
            eprintln!("[demo] connect to {} failed: {err:#}", args.c2_addr);
            return ExitCode::from(3);
        }
    };
    if let Ok(card) = client.get_agent_card().await {
        eprintln!("[demo] connected to {} ({})", card.name, card.version);
    }

    let (tx, mut rx) = match client.open_streaming_message().await {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[demo] open shell-session failed: {err}");
            return ExitCode::from(4);
        }
    };

    let task_id = "headless-demo";
    let open = ShellControl::ShellOpen {
        cols: 80,
        rows: 24,
        shell: None,
        target_agent_id: args.target_agent.clone(),
    };
    if tx
        .send(control_request(task_id, &open).expect("encode open"))
        .await
        .is_err()
    {
        eprintln!("[demo] send shell-open failed: stream closed");
        return ExitCode::from(5);
    }
    if tx
        .send(bytes_request(task_id, args.probe.into_bytes()))
        .await
        .is_err()
    {
        eprintln!("[demo] send probe bytes failed: stream closed");
        return ExitCode::from(5);
    }

    let deadline = Instant::now() + Duration::from_secs(args.timeout_secs);
    let mut captured: Vec<u8> = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            eprintln!(
                "[demo] FAIL: timeout after {}s, no marker {:?}; captured:\n---\n{}\n---",
                args.timeout_secs,
                probe_marker,
                String::from_utf8_lossy(&captured),
            );
            return ExitCode::from(6);
        }
        match timeout(remaining, rx.next()).await {
            Ok(Some(Ok(resp))) => {
                if let Some(pb::stream_response::Payload::Message(msg)) = resp.payload {
                    for part in msg.parts {
                        if let Some(pb::part::Part::File(bytes)) = part.part {
                            captured.extend_from_slice(&bytes);
                            if String::from_utf8_lossy(&captured).contains(&probe_marker) {
                                println!("{}", String::from_utf8_lossy(&captured).trim_end());
                                println!("[demo] PASS — marker observed");
                                drop(tx);
                                return ExitCode::SUCCESS;
                            }
                        }
                    }
                }
            }
            Ok(Some(Err(status))) => {
                eprintln!("[demo] FAIL: status err: {status:?}");
                return ExitCode::from(8);
            }
            Ok(None) => {
                eprintln!(
                    "[demo] FAIL: stream closed before marker; captured:\n---\n{}\n---",
                    String::from_utf8_lossy(&captured)
                );
                return ExitCode::from(7);
            }
            Err(_) => continue,
        }
    }
}

struct Args {
    c2_addr: String,
    insecure_network: bool,
    target_agent: Option<String>,
    probe: String,
    marker: Option<String>,
    timeout_secs: u64,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        c2_addr: "http://127.0.0.1:50051".to_string(),
        insecure_network: false,
        target_agent: None,
        probe: "whoami\n".to_string(),
        marker: None,
        timeout_secs: 10,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--c2" => {
                i += 1;
                out.c2_addr = args.get(i).cloned().ok_or("--c2 requires a value")?;
            }
            "--insecure-network" => out.insecure_network = true,
            "--target-agent" => {
                i += 1;
                out.target_agent = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("--target-agent requires a value")?,
                );
            }
            "--probe" => {
                i += 1;
                let s = args.get(i).cloned().ok_or("--probe requires a value")?;
                out.probe = if s.ends_with('\n') {
                    s
                } else {
                    format!("{s}\n")
                };
            }
            "--marker" => {
                i += 1;
                out.marker = Some(args.get(i).cloned().ok_or("--marker requires a value")?);
            }
            "--timeout-secs" => {
                i += 1;
                let s = args.get(i).ok_or("--timeout-secs requires a value")?;
                out.timeout_secs = s.parse().map_err(|e| format!("--timeout-secs: {e}"))?;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
        i += 1;
    }
    Ok(out)
}

fn default_marker(probe: &str) -> String {
    let trimmed = probe.trim();
    if trimmed == "whoami" {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .ok()
            .unwrap_or_else(|| "uid=".to_string())
    } else {
        trimmed.to_string()
    }
}
