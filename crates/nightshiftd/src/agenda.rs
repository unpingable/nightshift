//! Agenda — a declared deferred intention.
//!
//! v1 MVP field budget per DESIGN.md. Fields not active in v1 are
//! represented here but defaulted or unused by the v1 pipeline.

use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowFamily {
    Ops,
    Code,
    Publication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    Observe,
    Advise,
    Stage,
    Request,
    Apply,
    Publish,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Nq,
    Git,
    Fs,
    Continuity,
    Governor,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClass {
    Discover,
    Read,
    Propose,
    Stage,
    Mutate,
    Publish,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticalityClass {
    Standard,
    BusinessCritical,
    Safety,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedRole {
    ObservationCritical,
    ControlPlaneCritical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTarget {
    RepairProposal,
    Diff,
    Report,
    Packet,
    PublicationUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisMode {
    Singleton,
    SelfCheck,
    Conference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CadenceKind {
    Scheduled,
    Event,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cadence {
    pub kind: CadenceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<TriggerSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSpec {
    pub source: EvidenceSource,
    #[serde(default)]
    pub filter: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub repos: Vec<String>,
}

impl Scope {
    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty()
            && self.services.is_empty()
            && self.paths.is_empty()
            && self.repos.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcilerConfig {
    pub required: bool,
    #[serde(default)]
    pub invalidates_if: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_wall_seconds: u64,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub max_mcp_calls: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorBinding {
    pub required_above: AuthorityLevel,
    #[serde(default)]
    pub policy_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criticality {
    pub class: CriticalityClass,
    #[serde(with = "humantime_serde_duration")]
    pub re_alert_after: Duration,
    #[serde(with = "humantime_serde_duration")]
    pub silence_max_duration: Duration,
    #[serde(default)]
    pub protected_role: Option<ProtectedRole>,
    #[serde(default, with = "humantime_serde_duration_opt")]
    pub ack_due_by: Option<Duration>,
    #[serde(default)]
    pub handoff_required: bool,
    #[serde(default = "default_business_hours_ok")]
    pub business_hours_okay: bool,
}

fn default_business_hours_ok() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisConfig {
    pub mode: DiagnosisMode,
    #[serde(default)]
    pub conference_triggers: serde_json::Value,
}

/// The top-level agenda declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agenda {
    pub agenda_version: u32,
    pub agenda_id: String,
    pub workflow_family: WorkflowFamily,
    pub owner: String,
    pub cadence: Cadence,
    pub scope: Scope,
    pub artifact_target: ArtifactTarget,
    pub promotion_ceiling: AuthorityLevel,
    pub reconciler: ReconcilerConfig,
    pub allowed_evidence_sources: Vec<EvidenceSource>,
    pub allowed_tool_classes: Vec<ToolClass>,
    pub budget: Budget,
    pub governor_binding: GovernorBinding,
    pub criticality: Criticality,
    pub diagnosis: DiagnosisConfig,
}

/// Parse a humantime-like duration (e.g. "4h", "24h", "72h").
/// Minimal v1 implementation: supports s/m/h/d suffixes.
mod humantime_serde_duration {
    use chrono::Duration;
    use serde::{de, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        let secs = d.num_seconds();
        s.serialize_str(&format!("{}s", secs))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        use serde::Deserialize;
        let s = String::deserialize(d)?;
        super::parse_duration(&s).map_err(de::Error::custom)
    }
}

mod humantime_serde_duration_opt {
    use chrono::Duration;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Option<Duration>, s: S) -> Result<S::Ok, S::Error> {
        match d {
            Some(dur) => s.serialize_str(&format!("{}s", dur.num_seconds())),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
        let s = Option::<String>::deserialize(d)?;
        match s {
            Some(s) => super::parse_duration(&s).map(Some).map_err(de::Error::custom),
            None => Ok(None),
        }
    }
}

pub(crate) fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration".into());
    }
    let (num, suffix) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| format!("no suffix in duration: {s}"))?,
    );
    let n: i64 = num.parse().map_err(|e| format!("bad number in {s}: {e}"))?;
    let d = match suffix {
        "s" => Duration::seconds(n),
        "m" => Duration::minutes(n),
        "h" => Duration::hours(n),
        "d" => Duration::days(n),
        other => return Err(format!("unknown duration suffix: {other}")),
    };
    Ok(d)
}
