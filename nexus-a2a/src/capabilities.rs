//! v1.2 capability matrix gate (D-V1.2-caps), v1.3.5-extended with
//! per-operator scoping (D-V1.3-G), v1.4-rewritten on top of
//! nalgebra's dense `DMatrix<bool>` (D-V1.4-C / v2.1.2 Phase 4).
//!
//! ## Wire / config format
//!
//! JSON file at `~/.nexus/capabilities.json` (or per server config):
//!
//! ```jsonc
//! {
//!   "agents": {
//!     "<peer_id_hex>": {"skills": ["shell-session", "file-read"]},
//!     "*": {"skills": ["shell-session"]}
//!   },
//!   "operators": {
//!     "operator-alice": {"agents": ["ab12..."], "skills": ["shell-session"]},
//!     "*": {"agents": ["*"], "skills": ["shell-session"]}
//!   }
//! }
//! ```
//!
//! The `"*"` key acts as a wildcard default applied to any agent or
//! operator without an explicit entry. The wildcard skill `"*"` allows
//! every skill.
//!
//! ## v1.4 internal representation
//!
//! Parsed rules project into three dense matrices:
//!
//! - `agent_skills`: `[n_agents × n_skills]` — does agent `i` advertise skill `j`?
//! - `operator_agents`: `[n_operators × n_agents]` — may operator `i` target agent `j`?
//! - `operator_skills`: `[n_operators × n_skills]` — may operator `i` invoke skill `j`?
//!
//! Wildcards live in separate `HashSet<usize>` overlays so they don't
//! consume a sentinel row that would distort sizes. `verify_with_operator`
//! is a single triple-AND over scalar boolean lookups (matrix or wildcard).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};

/// Per-agent allow rules (parsed from JSON).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AgentEntry {
    skills: HashSet<String>,
}

/// v1.3.5 — per-operator scoping (D-V1.3-G).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OperatorEntry {
    #[serde(default)]
    agents: HashSet<String>,
    #[serde(default)]
    skills: HashSet<String>,
}

/// Decoded capability matrix file (the JSON shape).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CapabilityFile {
    agents: HashMap<String, AgentEntry>,
    #[serde(default)]
    operators: HashMap<String, OperatorEntry>,
}

/// v1.4 dense-matrix routing core.
///
/// Built from a [`CapabilityFile`] by [`compile`]. Wildcards stay as
/// `HashSet<usize>` overlays so the matrix dimensions reflect only
/// the explicitly-named entities.
#[derive(Debug, Clone)]
struct MatrixRouter {
    agent_index: HashMap<String, usize>,
    skill_index: HashMap<String, usize>,
    operator_index: HashMap<String, usize>,

    agent_skills: DMatrix<bool>,
    operator_agents: DMatrix<bool>,
    operator_skills: DMatrix<bool>,

    /// Skill indexes the wildcard `"*"` agent entry covers (`None` if no
    /// wildcard agent entry exists).
    wildcard_agent_skills: HashSet<usize>,
    /// The wildcard agent entry explicitly contains `"*"` as a skill
    /// (i.e. allows every skill).
    wildcard_agent_any_skill: bool,

    /// Agent indexes the wildcard `"*"` operator entry covers.
    wildcard_operator_agents: HashSet<usize>,
    /// Wildcard operator allows every agent.
    wildcard_operator_any_agent: bool,
    /// Skill indexes the wildcard `"*"` operator entry covers.
    wildcard_operator_skills: HashSet<usize>,
    /// Wildcard operator allows every skill.
    wildcard_operator_any_skill: bool,

    /// True iff the parsed file had a non-empty `operators` section
    /// (controls backward-compat fallback in `verify_with_operator`).
    has_operators_section: bool,
}

impl Default for MatrixRouter {
    fn default() -> Self {
        // Empty 1x1 matrices act as deny-by-default; index maps are
        // empty so no lookup succeeds.
        Self {
            agent_index: HashMap::new(),
            skill_index: HashMap::new(),
            operator_index: HashMap::new(),
            agent_skills: DMatrix::from_element(1, 1, false),
            operator_agents: DMatrix::from_element(1, 1, false),
            operator_skills: DMatrix::from_element(1, 1, false),
            wildcard_agent_skills: HashSet::new(),
            wildcard_agent_any_skill: false,
            wildcard_operator_agents: HashSet::new(),
            wildcard_operator_any_agent: false,
            wildcard_operator_skills: HashSet::new(),
            wildcard_operator_any_skill: false,
            has_operators_section: false,
        }
    }
}

impl MatrixRouter {
    fn compile(file: &CapabilityFile) -> Self {
        // -- Build index maps. Skill index sees skills from BOTH the
        //    agents section AND the operators section so wildcard
        //    skill lookups have a column to live in.
        let mut agent_index: HashMap<String, usize> = HashMap::new();
        let mut skill_index: HashMap<String, usize> = HashMap::new();
        let mut operator_index: HashMap<String, usize> = HashMap::new();

        for (agent_id, entry) in &file.agents {
            if agent_id != "*" {
                let next = agent_index.len();
                agent_index.entry(agent_id.clone()).or_insert(next);
            }
            for skill in &entry.skills {
                if skill != "*" {
                    let next = skill_index.len();
                    skill_index.entry(skill.clone()).or_insert(next);
                }
            }
        }

        for (operator_cn, entry) in &file.operators {
            if operator_cn != "*" {
                let next = operator_index.len();
                operator_index.entry(operator_cn.clone()).or_insert(next);
            }
            for agent in &entry.agents {
                if agent != "*" {
                    let next = agent_index.len();
                    agent_index.entry(agent.clone()).or_insert(next);
                }
            }
            for skill in &entry.skills {
                if skill != "*" {
                    let next = skill_index.len();
                    skill_index.entry(skill.clone()).or_insert(next);
                }
            }
        }

        let n_agents = agent_index.len();
        let n_skills = skill_index.len();
        let n_operators = operator_index.len();

        // -- Allocate matrices (nrows × ncols of bool).
        let mut agent_skills = DMatrix::from_element(n_agents.max(1), n_skills.max(1), false);
        let mut operator_agents = DMatrix::from_element(n_operators.max(1), n_agents.max(1), false);
        let mut operator_skills = DMatrix::from_element(n_operators.max(1), n_skills.max(1), false);

        let mut wildcard_agent_skills: HashSet<usize> = HashSet::new();
        let mut wildcard_agent_any_skill = false;
        let mut wildcard_operator_agents: HashSet<usize> = HashSet::new();
        let mut wildcard_operator_any_agent = false;
        let mut wildcard_operator_skills: HashSet<usize> = HashSet::new();
        let mut wildcard_operator_any_skill = false;

        // -- Project agent rules.
        for (agent_id, entry) in &file.agents {
            if agent_id == "*" {
                for skill in &entry.skills {
                    if skill == "*" {
                        wildcard_agent_any_skill = true;
                    } else if let Some(&j) = skill_index.get(skill) {
                        wildcard_agent_skills.insert(j);
                    }
                }
            } else if let Some(&i) = agent_index.get(agent_id) {
                for skill in &entry.skills {
                    if skill == "*" {
                        // Agent grants every skill: fill the agent's row.
                        for j in 0..n_skills {
                            agent_skills[(i, j)] = true;
                        }
                    } else if let Some(&j) = skill_index.get(skill) {
                        agent_skills[(i, j)] = true;
                    }
                }
            }
        }

        // -- Project operator rules.
        for (operator_cn, entry) in &file.operators {
            if operator_cn == "*" {
                for agent in &entry.agents {
                    if agent == "*" {
                        wildcard_operator_any_agent = true;
                    } else if let Some(&j) = agent_index.get(agent) {
                        wildcard_operator_agents.insert(j);
                    }
                }
                for skill in &entry.skills {
                    if skill == "*" {
                        wildcard_operator_any_skill = true;
                    } else if let Some(&j) = skill_index.get(skill) {
                        wildcard_operator_skills.insert(j);
                    }
                }
            } else if let Some(&i) = operator_index.get(operator_cn) {
                for agent in &entry.agents {
                    if agent == "*" {
                        for j in 0..n_agents {
                            operator_agents[(i, j)] = true;
                        }
                    } else if let Some(&j) = agent_index.get(agent) {
                        operator_agents[(i, j)] = true;
                    }
                }
                for skill in &entry.skills {
                    if skill == "*" {
                        for j in 0..n_skills {
                            operator_skills[(i, j)] = true;
                        }
                    } else if let Some(&j) = skill_index.get(skill) {
                        operator_skills[(i, j)] = true;
                    }
                }
            }
        }

        MatrixRouter {
            agent_index,
            skill_index,
            operator_index,
            agent_skills,
            operator_agents,
            operator_skills,
            wildcard_agent_skills,
            wildcard_agent_any_skill,
            wildcard_operator_agents,
            wildcard_operator_any_agent,
            wildcard_operator_skills,
            wildcard_operator_any_skill,
            has_operators_section: !file.operators.is_empty(),
        }
    }

    /// Agent-side check: does `agent` advertise `skill`?
    fn allows_agent_skill(&self, agent_id: &str, skill_id: &str) -> bool {
        let skill_idx = self.skill_index.get(skill_id).copied();
        // Explicit named entry?
        if let Some(&agent_idx) = self.agent_index.get(agent_id) {
            if let Some(j) = skill_idx {
                if self.agent_skills[(agent_idx, j)] {
                    return true;
                }
            }
        }
        // Wildcard agent entry?
        if self.wildcard_agent_any_skill {
            return true;
        }
        if let Some(j) = skill_idx {
            if self.wildcard_agent_skills.contains(&j) {
                return true;
            }
        }
        false
    }

    /// Operator gate: may `operator` reach `agent` for `skill`?
    fn allows_operator_triple(&self, operator_cn: &str, agent_id: &str, skill_id: &str) -> bool {
        let agent_idx = self.agent_index.get(agent_id).copied();
        let skill_idx = self.skill_index.get(skill_id).copied();

        // Try the named operator entry first.
        if let Some(&op_idx) = self.operator_index.get(operator_cn) {
            let agent_ok = match agent_idx {
                Some(j) => self.operator_agents[(op_idx, j)],
                None => false,
            };
            let skill_ok = match skill_idx {
                Some(j) => self.operator_skills[(op_idx, j)],
                None => false,
            };
            if agent_ok && skill_ok {
                return true;
            }
        }

        // Wildcard operator entry covers any operator without an
        // explicit row. Note: an explicit operator row that didn't
        // match falls back to the wildcard (matches v1.3 semantics).
        let wildcard_agent_ok = self.wildcard_operator_any_agent
            || agent_idx
                .map(|j| self.wildcard_operator_agents.contains(&j))
                .unwrap_or(false);
        let wildcard_skill_ok = self.wildcard_operator_any_skill
            || skill_idx
                .map(|j| self.wildcard_operator_skills.contains(&j))
                .unwrap_or(false);
        wildcard_agent_ok && wildcard_skill_ok
    }
}

/// Capability check used by [`crate::server::A2aServer::with_capability_check`].
///
/// v1.4: stores a compiled [`MatrixRouter`] plus the raw
/// [`CapabilityFile`] so `reload` can refresh from disk and recompile.
#[derive(Debug, Clone, Default)]
pub struct CapabilityCheck {
    file: CapabilityFile,
    router: MatrixRouter,
}

// Default derives via #[derive(Default)] above use MatrixRouter's
// manual Default impl.

/// Errors loading / verifying.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    /// Filesystem error reading the capabilities JSON.
    #[error("read capabilities file {path}: {err}")]
    Io {
        /// File path.
        path: String,
        /// Inner io error.
        err: std::io::Error,
    },
    /// JSON parse error.
    #[error("parse capabilities JSON: {0}")]
    Parse(#[from] serde_json::Error),
    /// Operator → agent → skill triple was not allowed.
    #[error("capability denied: skill `{skill}` not allowed for agent `{agent}`")]
    Denied {
        /// Target agent peer-id (hex).
        agent: String,
        /// Skill id that was checked.
        skill: String,
    },
    /// v1.3.5 — operator denied for the (agent, skill) target.
    #[error("capability denied: operator `{operator}` not allowed to use skill `{skill}` against agent `{agent}`")]
    OperatorDenied {
        /// Operator CN.
        operator: String,
        /// Target agent peer-id (hex).
        agent: String,
        /// Skill id.
        skill: String,
    },
}

impl CapabilityCheck {
    /// Empty (denies everything). Useful as a hard-deny default.
    #[must_use]
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Allow every check (permissive default for the loopback example).
    #[must_use]
    pub fn allow_all() -> Self {
        let mut file = CapabilityFile::default();
        let mut entry = AgentEntry::default();
        entry.skills.insert("*".to_string());
        file.agents.insert("*".to_string(), entry);
        let router = MatrixRouter::compile(&file);
        Self { file, router }
    }

    /// Load from a JSON file on disk.
    pub fn from_json_file(path: &Path) -> Result<Self, CapabilityError> {
        let bytes = std::fs::read(path).map_err(|err| CapabilityError::Io {
            path: path.display().to_string(),
            err,
        })?;
        let file: CapabilityFile = serde_json::from_slice(&bytes)?;
        let router = MatrixRouter::compile(&file);
        Ok(Self { file, router })
    }

    /// v1.3.5: reload the matrix in place from `path`. Returns `Err`
    /// without mutating self if the file fails to parse — operators get
    /// to discover a typo before the policy goes live. The intended
    /// invocation path is a SIGHUP handler (operator runs
    /// `systemctl reload nexus-server`).
    pub fn reload(&mut self, path: &Path) -> Result<(), CapabilityError> {
        let fresh = Self::from_json_file(path)?;
        self.file = fresh.file;
        self.router = fresh.router;
        Ok(())
    }

    /// Parse from a JSON string.
    pub fn from_json_str(s: &str) -> Result<Self, CapabilityError> {
        let file: CapabilityFile = serde_json::from_str(s)?;
        let router = MatrixRouter::compile(&file);
        Ok(Self { file, router })
    }

    /// Check whether `skill` may be invoked against `agent_peer_id_hex`.
    /// Agent-side check only — see [`Self::verify_with_operator`] for
    /// the v1.3 per-operator-scoped check.
    pub fn verify(&self, agent_peer_id_hex: &str, skill: &str) -> Result<(), CapabilityError> {
        if self.router.allows_agent_skill(agent_peer_id_hex, skill) {
            Ok(())
        } else {
            Err(CapabilityError::Denied {
                agent: agent_peer_id_hex.to_string(),
                skill: skill.to_string(),
            })
        }
    }

    /// v1.3.5 — per-operator scoping (D-V1.3-G), v1.4-implemented over
    /// the MatrixRouter.
    pub fn verify_with_operator(
        &self,
        operator_cn: &str,
        agent_peer_id_hex: &str,
        skill: &str,
    ) -> Result<(), CapabilityError> {
        // Agent-side gate is non-negotiable.
        self.verify(agent_peer_id_hex, skill)?;

        // Pre-v1.3 capability files: no operators section. Allow the
        // existing single-tier gate to be authoritative.
        if !self.router.has_operators_section {
            return Ok(());
        }

        if self
            .router
            .allows_operator_triple(operator_cn, agent_peer_id_hex, skill)
        {
            Ok(())
        } else {
            Err(CapabilityError::OperatorDenied {
                operator: operator_cn.to_string(),
                agent: agent_peer_id_hex.to_string(),
                skill: skill.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "agents": {
            "ab12": {"skills": ["shell-session", "file-read"]},
            "*": {"skills": ["shell-session"]}
        }
    }"#;

    #[test]
    fn parse_and_verify_per_agent() {
        let check = CapabilityCheck::from_json_str(SAMPLE).expect("parse");
        check.verify("ab12", "shell-session").expect("allowed");
        check.verify("ab12", "file-read").expect("allowed");
        check.verify("ab12", "exec-bof").expect_err("denied");
    }

    #[test]
    fn wildcard_default_applies() {
        let check = CapabilityCheck::from_json_str(SAMPLE).expect("parse");
        check.verify("ff99", "shell-session").expect("wildcard");
        check.verify("ff99", "file-read").expect_err("denied");
    }

    #[test]
    fn deny_all_denies() {
        let check = CapabilityCheck::deny_all();
        check.verify("aa", "shell-session").expect_err("denied");
    }

    #[test]
    fn allow_all_allows() {
        let check = CapabilityCheck::allow_all();
        check.verify("aa", "shell-session").expect("allow_all");
        check
            .verify("xx", "anything")
            .expect("allow_all wildcard skill");
    }

    // v1.3.5 — per-operator scoping (D-V1.3-G).

    const SAMPLE_V1_3: &str = r#"{
        "agents": {
            "ab12": {"skills": ["shell-session"]},
            "ff99": {"skills": ["shell-session"]}
        },
        "operators": {
            "operator-alice": {"agents": ["ab12"], "skills": ["shell-session"]},
            "operator-bob":   {"agents": ["*"], "skills": ["shell-session"]}
        }
    }"#;

    #[test]
    fn operator_scoped_allow() {
        let check = CapabilityCheck::from_json_str(SAMPLE_V1_3).expect("parse");
        check
            .verify_with_operator("operator-alice", "ab12", "shell-session")
            .expect("alice can target ab12");
    }

    #[test]
    fn operator_scoped_deny_by_agent() {
        let check = CapabilityCheck::from_json_str(SAMPLE_V1_3).expect("parse");
        let err = check
            .verify_with_operator("operator-alice", "ff99", "shell-session")
            .expect_err("alice cannot target ff99");
        matches!(err, CapabilityError::OperatorDenied { .. });
    }

    #[test]
    fn operator_wildcard_agent_allows() {
        let check = CapabilityCheck::from_json_str(SAMPLE_V1_3).expect("parse");
        check
            .verify_with_operator("operator-bob", "ab12", "shell-session")
            .expect("bob has wildcard agent");
        check
            .verify_with_operator("operator-bob", "ff99", "shell-session")
            .expect("bob has wildcard agent");
    }

    #[test]
    fn pre_v1_3_file_still_works() {
        // SAMPLE has no `operators` section. verify_with_operator
        // should fall back to the agent-only check.
        let check = CapabilityCheck::from_json_str(SAMPLE).expect("parse");
        check
            .verify_with_operator("any-operator", "ab12", "shell-session")
            .expect("pre-v1.3 fallback");
    }

    #[test]
    fn reload_swaps_policy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("capabilities.json");
        std::fs::write(&path, SAMPLE).expect("write v1.0");
        let mut check = CapabilityCheck::from_json_file(&path).expect("load v1.0");
        check.verify("ab12", "shell-session").expect("v1.0 allows");

        std::fs::write(&path, r#"{"agents":{}}"#).expect("write empty");
        check.reload(&path).expect("reload");
        check
            .verify("ab12", "shell-session")
            .expect_err("after reload the policy denies");
    }

    // v1.4.6 — matrix-router stress test (D-V1.4-C).

    #[test]
    fn matrix_router_scale_envelope() {
        // 100 agents × 8 skills, 50 operators. Smaller than the
        // documented 1k × 10k × 64 envelope but enough to exercise
        // the matrix construction + lookup paths in CI.
        let mut file = CapabilityFile::default();
        for i in 0..100 {
            let mut e = AgentEntry::default();
            for s in 0..8 {
                e.skills.insert(format!("skill-{s}"));
            }
            file.agents.insert(format!("agent-{i:03}"), e);
        }
        for i in 0..50 {
            let mut e = OperatorEntry::default();
            e.agents.insert(format!("agent-{:03}", i));
            e.skills.insert("skill-0".to_string());
            file.operators.insert(format!("operator-{i:02}"), e);
        }
        let json = serde_json::to_string(&file).expect("ser");
        let check = CapabilityCheck::from_json_str(&json).expect("parse");

        // Inside-envelope allow.
        check
            .verify_with_operator("operator-00", "agent-000", "skill-0")
            .expect("allowed");
        // Outside-envelope deny (wrong agent for this operator).
        check
            .verify_with_operator("operator-00", "agent-001", "skill-0")
            .expect_err("denied: wrong agent");
        // Outside-envelope deny (wrong skill).
        check
            .verify_with_operator("operator-00", "agent-000", "skill-7")
            .expect_err("denied: wrong skill for this operator");
    }
}
