use nexus_common::*;
use std::env;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub username: String,
    pub process_id: u32,
    pub process_name: String,
    pub primary_ip: String,
}

impl SystemInfo {
    pub async fn collect() -> Result<Self> {
        Ok(Self {
            hostname: Self::get_hostname(),
            os_name: Self::get_os_name(),
            os_version: Self::get_os_version(),
            architecture: Self::get_architecture(),
            username: Self::get_username(),
            process_id: std::process::id(),
            process_name: Self::get_process_name(),
            primary_ip: Self::get_primary_ip().await,
        })
    }

    fn get_hostname() -> String {
        hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    fn get_os_name() -> String {
        #[cfg(target_os = "windows")]
        return "Windows".to_string();

        #[cfg(target_os = "linux")]
        return "Linux".to_string();

        #[cfg(target_os = "macos")]
        return "macOS".to_string();

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return "Unknown".to_string();
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "windows")]
        {
            // Try to get Windows version via WMI or registry
            "Windows 10+".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/version")
                .unwrap_or_else(|_| "Linux Unknown".to_string())
                .lines()
                .next()
                .unwrap_or("Linux Unknown")
                .to_string()
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        "Unknown".to_string()
    }

    fn get_architecture() -> String {
        #[cfg(target_arch = "x86_64")]
        return "x86_64".to_string();

        #[cfg(target_arch = "x86")]
        return "x86".to_string();

        #[cfg(target_arch = "arm")]
        return "arm".to_string();

        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "arm",
            target_arch = "aarch64"
        )))]
        return "unknown".to_string();
    }

    fn get_username() -> String {
        env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn get_process_name() -> String {
        env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    async fn get_primary_ip() -> String {
        match local_ip_address::local_ip() {
            Ok(ip) => ip.to_string(),
            Err(_) => "127.0.0.1".to_string(),
        }
    }
}
