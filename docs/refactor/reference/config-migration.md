# ⚙️ Configuration Migration

> Guide for nexus.toml configuration changes.

## 📋 Overview

This document tracks configuration changes across the transformation phases.

## 🆕 New Configuration Sections

### Phase 1: LitterBox

```toml
[litterbox]
enabled = true
auto_deploy = true
instances_per_region = 2
max_instances_per_region = 5

[litterbox.deployment]
docker_setup_timeout = 3600
nginx_proxy_enabled = true
ssl_termination = true

[litterbox.regions]
us_east = { enabled = true, priority = "high" }
us_west = { enabled = true, priority = "high" }
eu_central = { enabled = true, priority = "medium" }

[litterbox.analysis]
static_analysis_enabled = true
dynamic_analysis_enabled = true
high_priority_threshold = 0.8
timeout_seconds = 3600

[litterbox.integration]
reverse_shell_detector = true
auto_submit_detections = true
min_confidence_threshold = 0.7
```

### Phase 1-2: Detection

```toml
[detection]
enabled = true
signature_engine = true
behavioral_analysis = true
process_correlation = true

[detection.thresholds]
alert_confidence = 0.7
auto_response_confidence = 0.9
```

### Phase 3: SOC Platform

```toml
[soc_platform]
enabled = true
dashboard_port = 8443
api_port = 9443

[soc_platform.response]
auto_isolation_enabled = false
auto_quarantine_enabled = true
```

### Phase 4: SIEM

```toml
[siem.splunk]
enabled = true
hec_endpoint = "https://splunk.example.com:8088"
hec_token_env = "SPLUNK_HEC_TOKEN"

[siem.qradar]
enabled = false

[siem.sentinel]
enabled = false
```

## ⚠️ Deprecated Sections

| Section | Status | Replacement |
|---------|--------|-------------|
| `[security.anti_analysis]` | Deprecated | `[detection.evasion_detection]` |
| `[domains.rotation_interval]` | Removed | Not needed for SOC |

## 🔄 Migration Steps

<!-- TODO: Add migration steps -->

---
**Version**: 0.1.0 (scaffold)
**Last Updated**: 2024-12-19
**Maintained By**: Configuration Agent
