//! Gov-Agent - Governance and Compliance Monitoring Agent
//!
//! This agent collects system information, validates security controls,
//! audits persistence mechanisms, and executes compliance checks for
//! governance frameworks like NIST, CIS, ISO 27001, and more.

use gov_common::*;
use log::{info, error};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

mod agent;
mod asset;
mod communication;
mod compliance_executor;
mod execution;
mod persistence_audit;
mod security_validation;

use agent::NexusAgent;
use asset::AssetInventory;
use compliance_executor::{ComplianceExecutor, ComplianceCheckLibrary};
use persistence_audit::PersistenceAuditor;
use security_validation::SecurityValidator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    info!("Gov-Agent starting...");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--scan") | Some("-s") => {
            run_compliance_scan().await?;
        }
        Some("--inventory") | Some("-i") => {
            run_asset_inventory().await?;
        }
        Some("--validate") | Some("-v") => {
            run_security_validation().await?;
        }
        Some("--audit") | Some("-a") => {
            run_persistence_audit().await?;
        }
        Some("--daemon") | Some("-d") => {
            run_daemon_mode(args.get(2).cloned()).await?;
        }
        Some("--help") | Some("-h") => {
            print_usage();
        }
        _ => {
            // Default: run full compliance scan
            run_compliance_scan().await?;
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Gov-Agent - Governance and Compliance Monitoring Agent");
    println!();
    println!("Usage: gov-agent [OPTION] [SERVER_ADDRESS]");
    println!();
    println!("Options:");
    println!("  -s, --scan       Run full compliance scan");
    println!("  -i, --inventory  Collect asset inventory");
    println!("  -v, --validate   Run security validation checks");
    println!("  -a, --audit      Audit persistence mechanisms");
    println!("  -d, --daemon     Run in daemon mode (connect to server)");
    println!("  -h, --help       Show this help message");
    println!();
    println!("Examples:");
    println!("  gov-agent --scan");
    println!("  gov-agent --daemon 192.168.1.100:8443");
}

/// Run a full compliance scan
async fn run_compliance_scan() -> Result<()> {
    info!("Running full compliance scan...");

    // Collect asset inventory
    info!("Collecting asset inventory...");
    let inventory = AssetInventory::collect().await?;
    println!("=== Asset Inventory ===");
    println!("Hostname: {}", inventory.hostname);
    println!("OS: {} {}", inventory.os_name, inventory.os_version);
    println!("Architecture: {}", inventory.architecture);
    println!("CPU Cores: {}", inventory.cpu_count);
    println!("Memory: {} MB", inventory.total_memory_mb);
    println!();

    // Run security validation
    info!("Running security validation...");
    let validator = SecurityValidator::new();
    let assessments = validator.run_all_validations().await;
    let score = SecurityValidator::calculate_security_score(&assessments);

    println!("=== Security Validation ===");
    for assessment in &assessments {
        let status_icon = match assessment.status {
            security_validation::AssessmentStatus::Pass => "[PASS]",
            security_validation::AssessmentStatus::Fail => "[FAIL]",
            _ => "[----]",
        };
        println!("{} {}: {}", status_icon, assessment.control_id, assessment.control_name);
        for finding in &assessment.findings {
            println!("       - {}", finding);
        }
    }
    println!();
    println!("Security Score: {:.1}%", score);
    println!();

    // Run persistence audit
    info!("Running persistence audit...");
    let auditor = PersistenceAuditor::new();
    let findings = auditor.audit_all().await;
    let summary = PersistenceAuditor::summarize_findings(&findings);

    println!("=== Persistence Audit ===");
    println!("Total entries found: {}", summary.total_findings);
    println!("Known good: {}", summary.known_good_count);
    println!("Unknown (review needed): {}", summary.unknown_count);
    println!("High risk: {}", summary.high_count);
    println!("Critical risk: {}", summary.critical_count);
    println!();

    // Run compliance checks
    info!("Running compliance checks...");
    let executor = ComplianceExecutor::new();

    #[cfg(target_os = "linux")]
    let checks = ComplianceCheckLibrary::cis_linux_checks();

    #[cfg(target_os = "windows")]
    let checks = ComplianceCheckLibrary::cis_windows_checks();

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let checks = Vec::new();

    let results = executor.execute_checks(&checks).await;

    println!("=== Compliance Checks ===");
    for result in &results {
        let status_str = match result.status {
            compliance_executor::CheckStatus::Pass => "[PASS]",
            compliance_executor::CheckStatus::Fail => "[FAIL]",
            compliance_executor::CheckStatus::Error => "[ERR ]",
            compliance_executor::CheckStatus::NotApplicable => "[N/A ]",
            compliance_executor::CheckStatus::Skipped => "[SKIP]",
        };
        println!("{} {}", status_str, result.check_id);
    }

    let passed = results.iter().filter(|r| r.status == compliance_executor::CheckStatus::Pass).count();
    let total = results.len();
    if total > 0 {
        println!();
        println!("Compliance: {}/{} checks passed ({:.1}%)", passed, total, (passed as f64 / total as f64) * 100.0);
    }

    info!("Compliance scan complete.");
    Ok(())
}

/// Collect and display asset inventory
async fn run_asset_inventory() -> Result<()> {
    info!("Collecting asset inventory...");

    let inventory = AssetInventory::collect().await?;

    println!("=== System Information ===");
    println!("Hostname: {}", inventory.hostname);
    println!("OS: {} {}", inventory.os_name, inventory.os_version);
    println!("Architecture: {}", inventory.architecture);
    println!("Username: {}", inventory.username);
    println!("Process ID: {}", inventory.process_id);
    println!("Primary IP: {}", inventory.primary_ip);
    println!("CPU Cores: {}", inventory.cpu_count);
    println!("Total Memory: {} MB", inventory.total_memory_mb);
    println!();

    println!("=== Security Tools ===");
    let active_tools = inventory.get_active_security_tools();
    if active_tools.is_empty() {
        println!("No active security tools detected!");
    } else {
        for tool in active_tools {
            println!("  [ACTIVE] {} ({:?})", tool.name, tool.tool_type);
        }
    }
    println!();

    println!("=== Running Services ===");
    for service in inventory.running_services.iter().take(20) {
        let status = match service.status {
            asset::ServiceStatus::Running => "Running",
            asset::ServiceStatus::Stopped => "Stopped",
            _ => "Unknown",
        };
        println!("  {} - {}", service.name, status);
    }
    if inventory.running_services.len() > 20 {
        println!("  ... and {} more", inventory.running_services.len() - 20);
    }

    Ok(())
}

/// Run security validation checks
async fn run_security_validation() -> Result<()> {
    info!("Running security validation...");

    let validator = SecurityValidator::new();
    let assessments = validator.run_all_validations().await;

    println!("=== Security Control Validation ===");
    println!();

    for assessment in &assessments {
        let status_str = match assessment.status {
            security_validation::AssessmentStatus::Pass => "PASS",
            security_validation::AssessmentStatus::Fail => "FAIL",
            security_validation::AssessmentStatus::NotApplicable => "N/A",
            security_validation::AssessmentStatus::Error => "ERROR",
            security_validation::AssessmentStatus::ManualReviewRequired => "REVIEW",
        };

        println!("[{}] {} - {}", status_str, assessment.control_id, assessment.control_name);

        for finding in &assessment.findings {
            println!("    Finding: {}", finding);
        }

        if !assessment.framework_mappings.is_empty() {
            println!("    Frameworks:");
            for mapping in &assessment.framework_mappings {
                println!("      - {} {}: {}", mapping.framework, mapping.control_id, mapping.control_name);
            }
        }
        println!();
    }

    let score = SecurityValidator::calculate_security_score(&assessments);
    println!("Overall Security Score: {:.1}%", score);

    Ok(())
}

/// Run persistence mechanism audit
async fn run_persistence_audit() -> Result<()> {
    info!("Running persistence audit...");

    let auditor = PersistenceAuditor::new();
    let findings = auditor.audit_all().await;

    println!("=== Persistence Mechanism Audit ===");
    println!();

    // Group by risk level
    let critical: Vec<_> = findings.iter().filter(|f| f.risk_level == persistence_audit::RiskLevel::Critical).collect();
    let high: Vec<_> = findings.iter().filter(|f| f.risk_level == persistence_audit::RiskLevel::High).collect();
    let medium: Vec<_> = findings.iter().filter(|f| f.risk_level == persistence_audit::RiskLevel::Medium && !f.is_known_good).collect();

    if !critical.is_empty() {
        println!("[CRITICAL RISK]");
        for finding in &critical {
            println!("  Location: {}", finding.location);
            println!("  Value: {}", finding.value);
            println!("  Recommendation: {}", finding.recommendation);
            println!();
        }
    }

    if !high.is_empty() {
        println!("[HIGH RISK]");
        for finding in &high {
            println!("  Location: {}", finding.location);
            println!("  Value: {}", finding.value);
            println!("  Recommendation: {}", finding.recommendation);
            println!();
        }
    }

    if !medium.is_empty() {
        println!("[MEDIUM RISK - Review Recommended]");
        for finding in medium.iter().take(10) {
            println!("  {} - {}", finding.location, finding.value);
        }
        if medium.len() > 10 {
            println!("  ... and {} more", medium.len() - 10);
        }
        println!();
    }

    let summary = PersistenceAuditor::summarize_findings(&findings);
    println!("=== Summary ===");
    println!("Total entries: {}", summary.total_findings);
    println!("Known good: {}", summary.known_good_count);
    println!("Unknown (need review): {}", summary.unknown_count);
    println!("Critical risk: {}", summary.critical_count);
    println!("High risk: {}", summary.high_count);
    println!("Medium risk: {}", summary.medium_count);
    println!("Low risk: {}", summary.low_count);

    Ok(())
}

/// Run in daemon mode, connecting to a governance server
async fn run_daemon_mode(server_addr: Option<String>) -> Result<()> {
    let server = server_addr.unwrap_or_else(|| "127.0.0.1:8443".to_string());
    info!("Starting daemon mode, connecting to {}...", server);

    let encryption_key = [0u8; 32]; // TODO: Load from config

    let mut agent = NexusAgent::new(server.clone(), encryption_key).await?;

    info!("Connected to governance server at {}", server);

    // Main agent loop
    loop {
        match agent.run_cycle().await {
            Ok(_) => {
                // Wait before next cycle
                sleep(Duration::from_secs(60)).await;
            }
            Err(e) => {
                error!("Agent cycle error: {}", e);
                // Retry after delay
                sleep(Duration::from_secs(120)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_asset_inventory() {
        let inventory = AssetInventory::collect().await;
        assert!(inventory.is_ok());
        let inv = inventory.unwrap();
        assert!(!inv.hostname.is_empty());
    }

    #[tokio::test]
    async fn test_security_validator() {
        let validator = SecurityValidator::new();
        let assessments = validator.run_all_validations().await;
        assert!(!assessments.is_empty());
    }

    #[tokio::test]
    async fn test_persistence_auditor() {
        let auditor = PersistenceAuditor::new();
        let findings = auditor.audit_all().await;
        // Should find at least some persistence entries on any system
        println!("Found {} persistence entries", findings.len());
    }
}
