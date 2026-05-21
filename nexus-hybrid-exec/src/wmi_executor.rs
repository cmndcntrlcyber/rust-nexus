//! WMI-based execution implementation (Windows only).
//!
//! Uses `Win32_Process.Create` to execute commands and captures their
//! output via a temporary file redirect.  Only compiled on Windows when
//! the `wmi` Cargo feature is enabled; on other platforms every method
//! returns `TaskExecutionError` so the fallback chain advances.

use crate::*;

pub struct WmiExecutor {
    config: HybridExecConfig,
}

impl WmiExecutor {
    pub fn new(config: HybridExecConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        #[cfg(all(feature = "wmi", target_os = "windows"))]
        {
            self.execute_real(request).await
        }
        #[cfg(not(all(feature = "wmi", target_os = "windows")))]
        {
            let _ = request;
            Err(NexusError::TaskExecutionError(
                "WMI executor requires Windows and the 'wmi' Cargo feature".into(),
            ))
        }
    }

    #[cfg(all(feature = "wmi", target_os = "windows"))]
    async fn execute_real(&self, request: &ExecutionRequest) -> Result<ExecutionResult> {
        use std::collections::HashMap;
        use wmi::{COMLibrary, WMIConnection, Variant};

        let start = std::time::Instant::now();
        let timeout_secs = request.timeout.unwrap_or(self.config.default_timeout);

        // Redirect output to a temp file so we can read it after the process exits.
        let out_path = std::env::temp_dir().join(format!("nexus_wmi_{}.txt", uuid::Uuid::new_v4()));
        let cmd_line = format!(
            "cmd.exe /C \"({}) > \"{}\" 2>&1\"",
            request.command,
            out_path.display()
        );

        // WMI work is synchronous; run on the blocking thread pool.
        let cmd_line_clone = cmd_line.clone();
        let target = request.target_endpoint.clone();
        let creds = request.credentials.clone();
        let store_creds = self.config.credentials_store.wmi_credentials.clone();
        let out_path_clone = out_path.clone();

        let join = tokio::task::spawn_blocking(move || -> std::result::Result<(bool, String, i32), String> {
            let com = COMLibrary::new().map_err(|e| e.to_string())?;

            let wmi_con = if target == "localhost" || target == "127.0.0.1" {
                WMIConnection::new(com).map_err(|e| e.to_string())?
            } else {
                let (user, pass, domain) = match creds {
                    Some(ExecutionCredentials::Wmi(c)) => (c.username, c.password, c.domain),
                    _ => {
                        let c = store_creds.get(&target)
                            .ok_or_else(|| format!("no WMI credentials for {}", target))?;
                        (c.username.clone(), c.password.clone(), c.domain.clone())
                    }
                };
                let namespace = format!("\\\\{}\\root\\cimv2", target);
                WMIConnection::with_credentials(
                    &namespace,
                    Some(&user),
                    Some(&pass),
                    domain.as_deref(),
                    com,
                )
                .map_err(|e| e.to_string())?
            };

            // Create the process via WMI.
            let mut props: HashMap<String, Variant> = HashMap::new();
            props.insert("CommandLine".into(), Variant::String(cmd_line_clone));
            let _result: Vec<HashMap<String, Variant>> = wmi_con
                .exec_query_native_wrapper(
                    &format!("CALL Win32_Process.Create(CommandLine=\"{}\")", cmd_line_clone.replace('"', "\\\"")),
                )
                .map_err(|e| e.to_string())?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            // Poll until the output file appears (process exited) or we time out.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
            while std::time::Instant::now() < deadline {
                if out_path_clone.exists() {
                    // Give the process a moment to flush and close the file.
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            let output = std::fs::read_to_string(&out_path_clone).unwrap_or_default();
            let _ = std::fs::remove_file(&out_path_clone);

            Ok((true, output, 0))
        });

        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs + 5),
            join,
        )
        .await
        {
            Ok(Ok(Ok((success, output, exit_code)))) => Ok(ExecutionResult {
                success,
                output,
                error: None,
                exit_code: Some(exit_code),
                execution_method: ExecutionProtocol::Wmi,
                target_endpoint: request.target_endpoint.clone(),
                duration: start.elapsed(),
                timestamp: chrono::Utc::now(),
            }),
            Ok(Ok(Err(e))) => Err(NexusError::TaskExecutionError(format!("WMI: {}", e))),
            Ok(Err(e)) => Err(NexusError::TaskExecutionError(format!("WMI task panic: {}", e))),
            Err(_) => Err(NexusError::TaskExecutionError("WMI execution timed out".into())),
        }
    }
}
