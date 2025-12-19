//! Signature patterns for threat detection
//!
//! Contains pattern definitions and pattern set management.

use serde::{Deserialize, Serialize};
use crate::types::Severity;

/// Type of pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// Regular expression pattern
    Regex,
    /// Exact string match
    Exact,
    /// Substring contains match
    Contains,
    /// YARA-style rule (future)
    Yara,
}

/// A detection pattern/signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique pattern identifier
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// The pattern string (regex, exact match, etc.)
    pub pattern: String,
    /// Severity if pattern matches
    pub severity: Severity,
    /// Human-readable description
    pub description: String,
    /// MITRE ATT&CK technique IDs
    pub mitre_techniques: Vec<String>,
    /// Category tags
    pub tags: Vec<String>,
    /// Whether pattern is enabled
    pub enabled: bool,
}

impl Pattern {
    /// Create a new pattern
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        pattern_type: PatternType,
        pattern: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pattern_type,
            pattern: pattern.into(),
            severity,
            description: String::new(),
            mitre_techniques: Vec::new(),
            tags: Vec::new(),
            enabled: true,
        }
    }

    /// Create a regex pattern
    pub fn regex(
        id: impl Into<String>,
        name: impl Into<String>,
        pattern: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self::new(id, name, PatternType::Regex, pattern, severity)
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add MITRE technique
    pub fn with_mitre(mut self, technique: impl Into<String>) -> Self {
        self.mitre_techniques.push(technique.into());
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// A collection of patterns
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternSet {
    /// Name of the pattern set
    pub name: String,
    /// Version of the pattern set
    pub version: String,
    /// Patterns in this set
    pub patterns: Vec<Pattern>,
}

impl PatternSet {
    /// Create a new empty pattern set
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            patterns: Vec::new(),
        }
    }

    /// Add a pattern to the set
    pub fn add(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    /// Get enabled patterns only
    pub fn enabled_patterns(&self) -> impl Iterator<Item = &Pattern> {
        self.patterns.iter().filter(|p| p.enabled)
    }

    /// Get patterns by tag
    pub fn patterns_by_tag(&self, tag: &str) -> Vec<&Pattern> {
        let tag = tag.to_string();
        self.patterns.iter().filter(|p| p.tags.contains(&tag)).collect()
    }

    /// Load default reverse shell patterns
    ///
    /// Comprehensive set of 35+ reverse shell detection patterns covering:
    /// - Bash/Shell variants
    /// - Netcat variants
    /// - Python variants
    /// - Perl variants
    /// - PHP variants
    /// - Ruby variants
    /// - PowerShell variants
    /// - Java variants
    /// - Other interpreters (Node.js, Lua, etc.)
    /// - Network tools (socat, telnet, etc.)
    pub fn default_reverse_shell_patterns() -> Self {
        let mut set = Self::new("reverse-shell-patterns", "2.0.0");

        // ==================== BASH/SHELL PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-BASH-001",
                "Bash /dev/tcp Reverse Shell",
                r"bash\s+-i\s+>&\s*/dev/tcp/",
                Severity::Critical,
            )
            .with_description("Bash reverse shell using /dev/tcp pseudo-device")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("bash"),
        );

        set.add(
            Pattern::regex(
                "RS-BASH-002",
                "Bash /dev/udp Reverse Shell",
                r"bash\s+-i\s+>&\s*/dev/udp/",
                Severity::Critical,
            )
            .with_description("Bash reverse shell using /dev/udp pseudo-device")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("bash"),
        );

        set.add(
            Pattern::regex(
                "RS-BASH-003",
                "Bash exec Redirect",
                r"exec\s+\d+<>/dev/tcp/",
                Severity::Critical,
            )
            .with_description("Bash exec with file descriptor redirect to /dev/tcp")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("bash"),
        );

        set.add(
            Pattern::regex(
                "RS-BASH-004",
                "Bash Process Substitution Shell",
                r"0<&\d+;\s*exec\s+\d+<>",
                Severity::High,
            )
            .with_description("Bash reverse shell using process substitution")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("bash"),
        );

        set.add(
            Pattern::regex(
                "RS-BASH-005",
                "Mkfifo Reverse Shell",
                r"mkfifo\s+\S+\s*;\s*(nc|cat)",
                Severity::Critical,
            )
            .with_description("Named pipe (mkfifo) based reverse shell")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("bash")
            .with_tag("mkfifo"),
        );

        // ==================== NETCAT PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-NC-001",
                "Netcat -e Reverse Shell",
                r"n(c|cat)\s+.*(-e|--exec)\s+(/bin/)?(ba)?sh",
                Severity::Critical,
            )
            .with_description("Netcat reverse shell with -e flag for shell execution")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("netcat"),
        );

        set.add(
            Pattern::regex(
                "RS-NC-002",
                "Netcat -c Reverse Shell",
                r"n(c|cat)\s+.*-c\s+(/bin/)?(ba)?sh",
                Severity::Critical,
            )
            .with_description("Netcat reverse shell with -c flag")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("netcat"),
        );

        set.add(
            Pattern::regex(
                "RS-NC-003",
                "Netcat Piped Shell",
                r"n(c|cat)\s+\S+\s+\d+\s*\|\s*(/bin/)?(ba)?sh",
                Severity::High,
            )
            .with_description("Netcat with piped shell output")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("netcat"),
        );

        set.add(
            Pattern::regex(
                "RS-NC-004",
                "Ncat SSL Reverse Shell",
                r"ncat\s+--ssl\s+.*(-e|--exec)",
                Severity::Critical,
            )
            .with_description("Ncat encrypted reverse shell using SSL")
            .with_mitre("T1059.004")
            .with_mitre("T1573")
            .with_tag("reverse-shell")
            .with_tag("netcat")
            .with_tag("encrypted"),
        );

        // ==================== PYTHON PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-PY-001",
                "Python Socket Reverse Shell",
                r"python[23]?\s+.*socket\.socket.*connect",
                Severity::Critical,
            )
            .with_description("Python reverse shell using socket module")
            .with_mitre("T1059.006")
            .with_tag("reverse-shell")
            .with_tag("python"),
        );

        set.add(
            Pattern::regex(
                "RS-PY-002",
                "Python pty Spawn",
                r"python[23]?\s+.*pty\.spawn",
                Severity::High,
            )
            .with_description("Python PTY spawn for interactive shell")
            .with_mitre("T1059.006")
            .with_tag("reverse-shell")
            .with_tag("python"),
        );

        set.add(
            Pattern::regex(
                "RS-PY-003",
                "Python subprocess Shell",
                r"python[23]?\s+.*subprocess\.(call|Popen|run).*shell\s*=\s*True",
                Severity::High,
            )
            .with_description("Python subprocess with shell=True")
            .with_mitre("T1059.006")
            .with_tag("reverse-shell")
            .with_tag("python"),
        );

        set.add(
            Pattern::regex(
                "RS-PY-004",
                "Python os.system Shell",
                r"python[23]?\s+.*os\.system.*(/bin/)?(ba)?sh",
                Severity::High,
            )
            .with_description("Python os.system executing shell")
            .with_mitre("T1059.006")
            .with_tag("reverse-shell")
            .with_tag("python"),
        );

        set.add(
            Pattern::regex(
                "RS-PY-005",
                "Python Base64 Encoded Command",
                r"python[23]?\s+.*base64\s*\.\s*b64decode",
                Severity::Medium,
            )
            .with_description("Python executing base64 decoded payload")
            .with_mitre("T1059.006")
            .with_mitre("T1027")
            .with_tag("reverse-shell")
            .with_tag("python")
            .with_tag("encoded"),
        );

        // ==================== PERL PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-PERL-001",
                "Perl Socket Reverse Shell",
                r"perl\s+.*socket\s*\(.*INET",
                Severity::Critical,
            )
            .with_description("Perl reverse shell using Socket module")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("perl"),
        );

        set.add(
            Pattern::regex(
                "RS-PERL-002",
                "Perl exec Shell",
                r#"perl\s+.*exec\s*\(?\s*['"]?(/bin/)?(ba)?sh"#,
                Severity::High,
            )
            .with_description("Perl exec calling shell")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("perl"),
        );

        set.add(
            Pattern::regex(
                "RS-PERL-003",
                "Perl Backtick Execution",
                r"perl\s+-e\s+.*`.*`",
                Severity::Medium,
            )
            .with_description("Perl backtick command execution")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("perl"),
        );

        // ==================== PHP PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-PHP-001",
                "PHP fsockopen Reverse Shell",
                r"php\s+.*fsockopen\s*\(",
                Severity::Critical,
            )
            .with_description("PHP reverse shell using fsockopen")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "RS-PHP-002",
                "PHP exec Shell",
                r"php\s+.*(exec|system|passthru|shell_exec)\s*\(",
                Severity::High,
            )
            .with_description("PHP command execution function")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "RS-PHP-003",
                "PHP proc_open",
                r"php\s+.*proc_open\s*\(",
                Severity::High,
            )
            .with_description("PHP proc_open for process execution")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "RS-PHP-004",
                "PHP popen Shell",
                r"php\s+.*popen\s*\(.*(/bin/)?(ba)?sh",
                Severity::Critical,
            )
            .with_description("PHP popen executing shell")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("php"),
        );

        // ==================== RUBY PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-RUBY-001",
                "Ruby Socket Reverse Shell",
                r"ruby\s+.*TCPSocket\.new",
                Severity::Critical,
            )
            .with_description("Ruby reverse shell using TCPSocket")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("ruby"),
        );

        set.add(
            Pattern::regex(
                "RS-RUBY-002",
                "Ruby exec Shell",
                r#"ruby\s+.*exec\s+['"]?(/bin/)?(ba)?sh"#,
                Severity::High,
            )
            .with_description("Ruby exec calling shell")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("ruby"),
        );

        set.add(
            Pattern::regex(
                "RS-RUBY-003",
                "Ruby Backtick Execution",
                r"ruby\s+-e\s+.*`.*`",
                Severity::Medium,
            )
            .with_description("Ruby backtick command execution")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("ruby"),
        );

        // ==================== POWERSHELL PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-PS-001",
                "PowerShell TCPClient",
                r"(?i)powershell.*TCPClient.*GetStream",
                Severity::Critical,
            )
            .with_description("PowerShell reverse shell using TCPClient")
            .with_mitre("T1059.001")
            .with_tag("reverse-shell")
            .with_tag("powershell"),
        );

        set.add(
            Pattern::regex(
                "RS-PS-002",
                "PowerShell Invoke-Expression Download",
                r"(?i)powershell.*(IEX|Invoke-Expression).*Download",
                Severity::Critical,
            )
            .with_description("PowerShell download and execute pattern")
            .with_mitre("T1059.001")
            .with_mitre("T1105")
            .with_tag("reverse-shell")
            .with_tag("powershell"),
        );

        set.add(
            Pattern::regex(
                "RS-PS-003",
                "PowerShell Base64 Encoded Command",
                r"(?i)powershell.*-e(nc(odedcommand)?)?.*[A-Za-z0-9+/=]{50,}",
                Severity::High,
            )
            .with_description("PowerShell with base64 encoded command")
            .with_mitre("T1059.001")
            .with_mitre("T1027")
            .with_tag("reverse-shell")
            .with_tag("powershell")
            .with_tag("encoded"),
        );

        set.add(
            Pattern::regex(
                "RS-PS-004",
                "PowerShell Socket Connection",
                r"(?i)powershell.*\[System\.Net\.Sockets",
                Severity::High,
            )
            .with_description("PowerShell using .NET Socket classes")
            .with_mitre("T1059.001")
            .with_tag("reverse-shell")
            .with_tag("powershell"),
        );

        set.add(
            Pattern::regex(
                "RS-PS-005",
                "PowerShell Hidden Window",
                r"(?i)powershell.*-w(indowstyle)?\s+(hidden|h)",
                Severity::Medium,
            )
            .with_description("PowerShell running with hidden window")
            .with_mitre("T1059.001")
            .with_mitre("T1564")
            .with_tag("reverse-shell")
            .with_tag("powershell")
            .with_tag("evasion"),
        );

        // ==================== JAVA PATTERNS ====================

        set.add(
            Pattern::regex(
                "RS-JAVA-001",
                "Java Runtime exec",
                r"java\s+.*Runtime.*exec.*(/bin/)?(ba)?sh",
                Severity::High,
            )
            .with_description("Java Runtime.exec calling shell")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("java"),
        );

        set.add(
            Pattern::regex(
                "RS-JAVA-002",
                "Java ProcessBuilder",
                r"ProcessBuilder.*(/bin/)?(ba)?sh",
                Severity::High,
            )
            .with_description("Java ProcessBuilder executing shell")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("java"),
        );

        // ==================== OTHER INTERPRETERS ====================

        set.add(
            Pattern::regex(
                "RS-NODE-001",
                "Node.js Child Process Shell",
                r"node\s+.*child_process.*spawn.*sh",
                Severity::High,
            )
            .with_description("Node.js child_process spawning shell")
            .with_mitre("T1059.007")
            .with_tag("reverse-shell")
            .with_tag("nodejs"),
        );

        set.add(
            Pattern::regex(
                "RS-NODE-002",
                "Node.js net Socket",
                r"node\s+.*require.*net.*Socket",
                Severity::High,
            )
            .with_description("Node.js net module for socket connection")
            .with_mitre("T1059.007")
            .with_tag("reverse-shell")
            .with_tag("nodejs"),
        );

        set.add(
            Pattern::regex(
                "RS-LUA-001",
                "Lua os.execute Shell",
                r"lua\s+.*os\.execute.*sh",
                Severity::High,
            )
            .with_description("Lua os.execute calling shell")
            .with_mitre("T1059")
            .with_tag("reverse-shell")
            .with_tag("lua"),
        );

        // ==================== NETWORK TOOLS ====================

        set.add(
            Pattern::regex(
                "RS-SOCAT-001",
                "Socat Reverse Shell",
                r"socat\s+.*exec:.*sh.*pty",
                Severity::Critical,
            )
            .with_description("Socat reverse shell with PTY")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("socat"),
        );

        set.add(
            Pattern::regex(
                "RS-SOCAT-002",
                "Socat TCP Connection",
                r"socat\s+tcp.*exec:",
                Severity::High,
            )
            .with_description("Socat TCP connection with exec")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("socat"),
        );

        set.add(
            Pattern::regex(
                "RS-TELNET-001",
                "Telnet Reverse Shell",
                r"telnet\s+\S+\s+\d+\s*\|.*sh.*\|.*telnet",
                Severity::High,
            )
            .with_description("Telnet-based reverse shell")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("telnet"),
        );

        set.add(
            Pattern::regex(
                "RS-AWK-001",
                "AWK Reverse Shell",
                r"awk\s+.*inet.*\|.*sh.*\|",
                Severity::High,
            )
            .with_description("AWK network-based reverse shell")
            .with_mitre("T1059.004")
            .with_tag("reverse-shell")
            .with_tag("awk"),
        );

        set.add(
            Pattern::regex(
                "RS-OPENSSL-001",
                "OpenSSL Reverse Shell",
                r"openssl\s+s_client.*\|.*sh.*\|.*openssl",
                Severity::Critical,
            )
            .with_description("OpenSSL encrypted reverse shell")
            .with_mitre("T1059.004")
            .with_mitre("T1573")
            .with_tag("reverse-shell")
            .with_tag("openssl")
            .with_tag("encrypted"),
        );

        set
    }

    /// Load web shell detection patterns
    pub fn default_webshell_patterns() -> Self {
        let mut set = Self::new("webshell-patterns", "1.0.0");

        set.add(
            Pattern::regex(
                "WS-PHP-001",
                "PHP eval POST",
                r"eval\s*\(\s*\$_(POST|GET|REQUEST)",
                Severity::Critical,
            )
            .with_description("PHP eval with user input - classic webshell")
            .with_mitre("T1505.003")
            .with_tag("webshell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "WS-PHP-002",
                "PHP assert Webshell",
                r"assert\s*\(\s*\$_(POST|GET|REQUEST)",
                Severity::Critical,
            )
            .with_description("PHP assert with user input")
            .with_mitre("T1505.003")
            .with_tag("webshell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "WS-PHP-003",
                "PHP preg_replace /e",
                r#"preg_replace\s*\(.*\/.*e['"]"#,
                Severity::Critical,
            )
            .with_description("PHP preg_replace with /e modifier for code execution")
            .with_mitre("T1505.003")
            .with_tag("webshell")
            .with_tag("php"),
        );

        set.add(
            Pattern::regex(
                "WS-JSP-001",
                "JSP Runtime exec",
                r"Runtime\.getRuntime\(\)\.exec\s*\(",
                Severity::High,
            )
            .with_description("JSP Runtime exec webshell pattern")
            .with_mitre("T1505.003")
            .with_tag("webshell")
            .with_tag("jsp"),
        );

        set.add(
            Pattern::regex(
                "WS-ASPX-001",
                "ASPX Process Start",
                r"Process\.Start\s*\(",
                Severity::High,
            )
            .with_description("ASP.NET Process.Start webshell pattern")
            .with_mitre("T1505.003")
            .with_tag("webshell")
            .with_tag("aspx"),
        );

        set
    }

    /// Load credential access detection patterns
    pub fn default_credential_patterns() -> Self {
        let mut set = Self::new("credential-patterns", "1.0.0");

        set.add(
            Pattern::regex(
                "CRED-001",
                "Mimikatz Execution",
                r"(?i)mimikatz|sekurlsa|logonpasswords|kerberos::list",
                Severity::Critical,
            )
            .with_description("Mimikatz credential dumping tool")
            .with_mitre("T1003.001")
            .with_tag("credential-access")
            .with_tag("mimikatz"),
        );

        set.add(
            Pattern::regex(
                "CRED-002",
                "Procdump LSASS",
                r"(?i)procdump.*lsass",
                Severity::Critical,
            )
            .with_description("Procdump targeting LSASS process")
            .with_mitre("T1003.001")
            .with_tag("credential-access")
            .with_tag("lsass"),
        );

        set.add(
            Pattern::regex(
                "CRED-003",
                "Comsvcs.dll MiniDump",
                r"(?i)comsvcs\.dll.*minidump",
                Severity::Critical,
            )
            .with_description("LSASS dump using comsvcs.dll")
            .with_mitre("T1003.001")
            .with_tag("credential-access")
            .with_tag("lsass"),
        );

        set.add(
            Pattern::regex(
                "CRED-004",
                "Reg Save SAM",
                r"(?i)reg\s+save\s+.*(sam|security|system)",
                Severity::High,
            )
            .with_description("Registry hive export for credential extraction")
            .with_mitre("T1003.002")
            .with_tag("credential-access")
            .with_tag("registry"),
        );

        set.add(
            Pattern::regex(
                "CRED-005",
                "NTDS.dit Access",
                r"(?i)ntds\.dit|esentutl.*ntds",
                Severity::Critical,
            )
            .with_description("Active Directory database access")
            .with_mitre("T1003.003")
            .with_tag("credential-access")
            .with_tag("ad"),
        );

        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::regex(
            "TEST-001",
            "Test Pattern",
            r"test.*pattern",
            Severity::Medium,
        )
        .with_description("A test pattern")
        .with_mitre("T1234")
        .with_tag("test");

        assert_eq!(pattern.id, "TEST-001");
        assert_eq!(pattern.severity, Severity::Medium);
        assert!(pattern.enabled);
        assert!(pattern.mitre_techniques.contains(&"T1234".to_string()));
    }

    #[test]
    fn test_reverse_shell_pattern_set() {
        let set = PatternSet::default_reverse_shell_patterns();
        assert!(!set.patterns.is_empty());
        // Should have 35+ patterns
        assert!(set.patterns.len() >= 30, "Expected 30+ patterns, got {}", set.patterns.len());

        // Check various language tags
        let bash_patterns = set.patterns_by_tag("bash");
        assert!(!bash_patterns.is_empty(), "Should have bash patterns");

        let python_patterns = set.patterns_by_tag("python");
        assert!(!python_patterns.is_empty(), "Should have python patterns");

        let powershell_patterns = set.patterns_by_tag("powershell");
        assert!(!powershell_patterns.is_empty(), "Should have powershell patterns");

        let netcat_patterns = set.patterns_by_tag("netcat");
        assert!(!netcat_patterns.is_empty(), "Should have netcat patterns");
    }

    #[test]
    fn test_webshell_pattern_set() {
        let set = PatternSet::default_webshell_patterns();
        assert!(!set.patterns.is_empty());
        assert!(set.patterns.len() >= 5);

        let php_patterns = set.patterns_by_tag("php");
        assert!(!php_patterns.is_empty());
    }

    #[test]
    fn test_credential_pattern_set() {
        let set = PatternSet::default_credential_patterns();
        assert!(!set.patterns.is_empty());
        assert!(set.patterns.len() >= 5);

        let lsass_patterns = set.patterns_by_tag("lsass");
        assert!(!lsass_patterns.is_empty());
    }

    #[test]
    fn test_all_patterns_have_mitre() {
        let set = PatternSet::default_reverse_shell_patterns();
        for pattern in &set.patterns {
            assert!(
                !pattern.mitre_techniques.is_empty(),
                "Pattern {} should have MITRE technique",
                pattern.id
            );
        }
    }

    #[test]
    fn test_patterns_by_severity() {
        let set = PatternSet::default_reverse_shell_patterns();
        let critical_count = set.patterns.iter().filter(|p| p.severity == Severity::Critical).count();
        let high_count = set.patterns.iter().filter(|p| p.severity == Severity::High).count();

        assert!(critical_count > 0, "Should have critical severity patterns");
        assert!(high_count > 0, "Should have high severity patterns");
    }
}
