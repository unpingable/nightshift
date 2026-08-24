//! Read-only NQ-NG admission-provenance boundary.
//!
//! Nightshift accepts a delivered diagnostic only after the configured NQ-NG
//! source qualifies the exact canonical artifact bytes and retained local
//! history. Admission makes evidence eligible for Nightshift reasoning. It is
//! not freshness, reliance, AG authorization, or permission to act.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::diagnostic_execution_v2::{DiagnosticExecution, NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA};
use crate::diagnostic_posture::{DiagnosticInputStatus, DiagnosticInputs};

pub const NQ_ADMISSION_QUERY_SCHEMA_V1: &str = "nightshift.nq_admission_query.v1";
pub const NQ_ADMISSION_PROVENANCE_SCHEMA_V1: &str = "nq.diagnostic_admission_provenance.v1";

const NQ_ADMISSION_NONCLAIMS: [&str; 3] = [
    "admission establishes evidence eligibility only",
    "this provenance does not establish freshness, reliance, authorization, or action",
    "source and resolver honesty remain environmental",
];

fn require_token(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(format!("{name} must be a non-empty token"));
    }
    Ok(())
}

fn require_digest(name: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!("{name} must use sha256:<64 lowercase hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{name} must use sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn object_id<T: Serialize>(value: &T, field: &str) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    value
        .as_object_mut()
        .ok_or_else(|| "identity preimage is not an object".to_owned())?
        .remove(field);
    let canonical = serde_jcs::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn bytes_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Exact artifact binding sent to the configured NQ-NG source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionQueryV1 {
    pub schema: String,
    pub query_id: String,
    pub source_id: String,
    pub artifact_id: String,
    pub contract_schema: String,
    pub canonical_bytes_sha256: String,
    pub canonical_bytes_length: u64,
    pub run_id: String,
    pub completed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_semantic_id: Option<String>,
}

impl NqAdmissionQueryV1 {
    pub fn from_artifact(artifact: &DiagnosticExecution) -> Result<Self, String> {
        artifact.validate()?;
        let canonical = serde_jcs::to_vec(artifact).map_err(|error| error.to_string())?;
        let mut query = Self {
            schema: NQ_ADMISSION_QUERY_SCHEMA_V1.into(),
            query_id: String::new(),
            source_id: artifact.producer().node_id.clone(),
            artifact_id: artifact.artifact_id().into(),
            contract_schema: artifact.schema_name().into(),
            canonical_bytes_sha256: bytes_digest(&canonical),
            canonical_bytes_length: u64::try_from(canonical.len())
                .map_err(|_| "diagnostic artifact length exceeds u64".to_owned())?,
            run_id: artifact.run_id().into(),
            completed_at: artifact.completed_at().into(),
            profile_semantic_id: artifact.profile_semantic_id().map(str::to_owned),
        };
        query.query_id = object_id(&query, "query_id")?;
        query.validate()?;
        Ok(query)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != NQ_ADMISSION_QUERY_SCHEMA_V1 {
            return Err("unsupported NQ admission query schema".into());
        }
        require_digest("query_id", &self.query_id)?;
        require_token("source_id", &self.source_id)?;
        require_digest("artifact_id", &self.artifact_id)?;
        if !matches!(
            self.contract_schema.as_str(),
            crate::diagnostic_posture::NQ_DIAGNOSTIC_EXECUTION_SCHEMA
                | NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA
        ) {
            return Err("NQ admission query carries an unsupported artifact schema".into());
        }
        require_digest("canonical_bytes_sha256", &self.canonical_bytes_sha256)?;
        if self.canonical_bytes_length == 0 {
            return Err("NQ admission query carries empty artifact bytes".into());
        }
        require_token("run_id", &self.run_id)?;
        DateTime::parse_from_rfc3339(&self.completed_at)
            .map_err(|_| "NQ admission query completed_at is not RFC3339".to_owned())?;
        match (&*self.contract_schema, &self.profile_semantic_id) {
            (NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA, Some(value)) => {
                require_digest("profile_semantic_id", value)?;
            }
            (NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA, None) => {
                return Err("NQ v2 admission query lacks profile_semantic_id".into());
            }
            (_, None) => {}
            (_, Some(_)) => return Err("NQ v1 admission query carries v2 provenance".into()),
        }
        if self.query_id != object_id(self, "query_id")? {
            return Err("NQ admission query identity mismatch".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NqSourceDispositionV1 {
    AdmittedReport,
    GovernedRefusal,
    AcquisitionFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionSourceV1 {
    pub kind: String,
    pub source_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionArtifactV1 {
    pub artifact_id: String,
    pub contract_schema: String,
    pub canonical_bytes_sha256: String,
    pub canonical_bytes_length: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionOriginV1 {
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_id: Option<String>,
    pub completed_at: String,
    pub committed_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionProviderV1 {
    pub provider_intake_id: String,
    pub raw_sha256: String,
    pub provider_admission_id: String,
    pub source_admission_id: String,
    pub admission_context_digest: String,
    pub profile_semantic_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionJudgmentV1 {
    pub report_id: String,
    pub judgment_schema: String,
    pub judgment_digest: String,
}

/// Immutable NQ-NG source admission carried into Nightshift persistence.
///
/// The content hash detects substitution in transit and storage. It is not a
/// signature; source authenticity comes from acquiring this record through
/// the separately configured NQ command/source identity boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqAdmissionProvenanceV1 {
    pub schema: String,
    pub provenance_id: String,
    pub source: NqAdmissionSourceV1,
    pub artifact: NqAdmissionArtifactV1,
    pub origin: NqAdmissionOriginV1,
    pub provider: NqAdmissionProviderV1,
    pub disposition: NqSourceDispositionV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judgment: Option<NqAdmissionJudgmentV1>,
    pub nonclaims: Vec<String>,
}

impl NqAdmissionProvenanceV1 {
    pub fn computed_provenance_id(&self) -> Result<String, String> {
        object_id(self, "provenance_id")
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema = NQ_ADMISSION_PROVENANCE_SCHEMA_V1.into();
        self.provenance_id.clear();
        self.nonclaims = NQ_ADMISSION_NONCLAIMS
            .into_iter()
            .map(str::to_owned)
            .collect();
        self.provenance_id = self.computed_provenance_id()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != NQ_ADMISSION_PROVENANCE_SCHEMA_V1 {
            return Err("unsupported NQ admission provenance schema".into());
        }
        require_digest("provenance_id", &self.provenance_id)?;
        if self.source.kind != "local_nq_store" {
            return Err("NQ admission provenance is not local-source custody".into());
        }
        require_token("source_id", &self.source.source_id)?;
        require_digest("artifact_id", &self.artifact.artifact_id)?;
        if !matches!(
            self.artifact.contract_schema.as_str(),
            crate::diagnostic_posture::NQ_DIAGNOSTIC_EXECUTION_SCHEMA
                | NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA
        ) || self.artifact.canonical_bytes_length == 0
        {
            return Err("NQ admission artifact binding is invalid".into());
        }
        require_digest(
            "canonical_bytes_sha256",
            &self.artifact.canonical_bytes_sha256,
        )?;
        require_token("run_id", &self.origin.run_id)?;
        if self
            .origin
            .evaluation_id
            .as_ref()
            .is_some_and(String::is_empty)
            || DateTime::parse_from_rfc3339(&self.origin.completed_at).is_err()
            || DateTime::parse_from_rfc3339(&self.origin.committed_at).is_err()
        {
            return Err("NQ admission origin is invalid".into());
        }
        require_token("provider_intake_id", &self.provider.provider_intake_id)?;
        require_digest("raw_sha256", &self.provider.raw_sha256)?;
        require_digest(
            "provider_admission_id",
            &self.provider.provider_admission_id,
        )?;
        require_token("source_admission_id", &self.provider.source_admission_id)?;
        require_digest(
            "admission_context_digest",
            &self.provider.admission_context_digest,
        )?;
        require_digest("profile_semantic_id", &self.provider.profile_semantic_id)?;
        match (self.disposition, &self.judgment) {
            (NqSourceDispositionV1::AdmittedReport, Some(judgment)) => {
                require_token("report_id", &judgment.report_id)?;
                if judgment.judgment_schema != "nq-ng.judgment.v1" {
                    return Err("NQ admission judgment schema is unsupported".into());
                }
                require_digest("judgment_digest", &judgment.judgment_digest)?;
            }
            (NqSourceDispositionV1::AdmittedReport, None) => {
                return Err("admitted NQ report lacks its exact judgment".into());
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err("non-admitted NQ source carries a report judgment".into());
            }
        }
        let expected_nonclaims: Vec<_> = NQ_ADMISSION_NONCLAIMS
            .into_iter()
            .map(str::to_owned)
            .collect();
        if self.nonclaims != expected_nonclaims {
            return Err("NQ admission nonclaims differ from the closed contract".into());
        }
        if self.provenance_id != self.computed_provenance_id()? {
            return Err("NQ admission provenance identity mismatch".into());
        }
        Ok(())
    }

    pub fn validate_for(&self, query: &NqAdmissionQueryV1) -> Result<(), String> {
        query.validate()?;
        self.validate()?;
        if self.source.source_id != query.source_id
            || self.artifact.artifact_id != query.artifact_id
            || self.artifact.contract_schema != query.contract_schema
            || self.artifact.canonical_bytes_sha256 != query.canonical_bytes_sha256
            || self.artifact.canonical_bytes_length != query.canonical_bytes_length
            || self.origin.run_id != query.run_id
            || self.origin.completed_at != query.completed_at
        {
            return Err(
                "NQ admission provenance does not bind the exact diagnostic artifact".into(),
            );
        }
        if query
            .profile_semantic_id
            .as_ref()
            .is_some_and(|expected| expected != &self.provider.profile_semantic_id)
        {
            return Err("NQ admission provenance substitutes the evaluated profile".into());
        }
        Ok(())
    }
}

/// Read-only port to the configured NQ-NG admission-provenance authority.
pub trait NqAdmissionPortV1 {
    fn qualify(&mut self, query: &NqAdmissionQueryV1) -> Result<NqAdmissionProvenanceV1, String>;
}

/// Production command adapter. Paths locate the source; only `source_id`
/// participates in provenance identity.
#[derive(Clone, Debug)]
pub struct CommandNqAdmissionPortV1 {
    program: PathBuf,
    config: PathBuf,
    source_id: String,
}

impl CommandNqAdmissionPortV1 {
    pub fn new(
        program: impl Into<PathBuf>,
        config: impl Into<PathBuf>,
        source_id: String,
    ) -> Result<Self, String> {
        let program = program.into();
        if program.file_name().and_then(|name| name.to_str()) != Some("nq") {
            return Err("NQ admission adapter accepts only the nq executable".into());
        }
        require_token("NQ source_id", &source_id)?;
        Ok(Self {
            program,
            config: config.into(),
            source_id,
        })
    }

    pub fn program(&self) -> &Path {
        &self.program
    }
}

impl NqAdmissionPortV1 for CommandNqAdmissionPortV1 {
    fn qualify(&mut self, query: &NqAdmissionQueryV1) -> Result<NqAdmissionProvenanceV1, String> {
        query.validate()?;
        if query.source_id != self.source_id {
            return Err("diagnostic producer does not match configured NQ source".into());
        }
        let output = Command::new(&self.program)
            .arg("--config")
            .arg(&self.config)
            .arg("--json")
            .arg("diagnostics")
            .arg("qualify")
            .arg(&query.artifact_id)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("NQ admission query failed to start: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "configured NQ source refused admission provenance: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let mut deserializer = serde_json::Deserializer::from_slice(&output.stdout);
        let provenance =
            NqAdmissionProvenanceV1::deserialize(&mut deserializer).map_err(|error| {
                format!("configured NQ source returned invalid provenance: {error}")
            })?;
        deserializer
            .end()
            .map_err(|error| format!("configured NQ source returned trailing data: {error}"))?;
        provenance.validate_for(query)?;
        Ok(provenance)
    }
}

/// Qualify each unique delivered artifact before Nightshift claims a slot.
pub fn qualify_delivered_inputs(
    port: &mut impl NqAdmissionPortV1,
    inputs: &DiagnosticInputs,
) -> Result<Vec<NqAdmissionProvenanceV1>, String> {
    let mut queries = BTreeMap::new();
    for input in &inputs.inputs {
        if let DiagnosticInputStatus::Delivered { artifact } = &input.status {
            let query = NqAdmissionQueryV1::from_artifact(artifact)?;
            match queries.insert(query.artifact_id.clone(), query.clone()) {
                Some(previous) if previous != query => {
                    return Err(
                        "one artifact identity is bound to different diagnostic bytes".into(),
                    );
                }
                _ => {}
            }
        }
    }
    queries
        .into_values()
        .map(|query| {
            let provenance = port.qualify(&query)?;
            provenance.validate_for(&query)?;
            Ok(provenance)
        })
        .collect()
}

/// Validate that persisted provenance is a complete one-to-one cover of all
/// exact delivered artifacts. Order is canonical artifact-id order.
pub fn validate_admission_cover(
    inputs: &DiagnosticInputs,
    provenance: &[NqAdmissionProvenanceV1],
) -> Result<(), String> {
    let mut expected = BTreeMap::new();
    for input in &inputs.inputs {
        if let DiagnosticInputStatus::Delivered { artifact } = &input.status {
            let query = NqAdmissionQueryV1::from_artifact(artifact)?;
            expected.insert(query.artifact_id.clone(), query);
        }
    }
    if expected.len() != provenance.len() {
        return Err(
            "NQ admission provenance does not cover every delivered artifact exactly once".into(),
        );
    }
    for ((artifact_id, query), actual) in expected.into_iter().zip(provenance) {
        if actual.artifact.artifact_id != artifact_id {
            return Err("NQ admission provenance is not in canonical artifact order".into());
        }
        actual.validate_for(&query)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn inputs() -> DiagnosticInputs {
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
        ))
        .unwrap()
    }

    fn provenance(query: &NqAdmissionQueryV1) -> NqAdmissionProvenanceV1 {
        NqAdmissionProvenanceV1 {
            schema: String::new(),
            provenance_id: String::new(),
            source: NqAdmissionSourceV1 {
                kind: "local_nq_store".into(),
                source_id: query.source_id.clone(),
            },
            artifact: NqAdmissionArtifactV1 {
                artifact_id: query.artifact_id.clone(),
                contract_schema: query.contract_schema.clone(),
                canonical_bytes_sha256: query.canonical_bytes_sha256.clone(),
                canonical_bytes_length: query.canonical_bytes_length,
            },
            origin: NqAdmissionOriginV1 {
                run_id: query.run_id.clone(),
                evaluation_id: Some("evaluation:test".into()),
                completed_at: query.completed_at.clone(),
                committed_at: query.completed_at.clone(),
            },
            provider: NqAdmissionProviderV1 {
                provider_intake_id: "provider-intake:test".into(),
                raw_sha256: digest('1'),
                provider_admission_id: digest('2'),
                source_admission_id: "source-admission:test".into(),
                admission_context_digest: digest('3'),
                profile_semantic_id: query
                    .profile_semantic_id
                    .clone()
                    .unwrap_or_else(|| digest('4')),
            },
            disposition: NqSourceDispositionV1::AdmittedReport,
            judgment: Some(NqAdmissionJudgmentV1 {
                report_id: "report:test".into(),
                judgment_schema: "nq-ng.judgment.v1".into(),
                judgment_digest: digest('5'),
            }),
            nonclaims: Vec::new(),
        }
        .seal()
        .unwrap()
    }

    struct FixedPort;

    impl NqAdmissionPortV1 for FixedPort {
        fn qualify(
            &mut self,
            query: &NqAdmissionQueryV1,
        ) -> Result<NqAdmissionProvenanceV1, String> {
            Ok(provenance(query))
        }
    }

    #[test]
    fn exact_artifact_has_one_complete_admission_binding() {
        let inputs = inputs();
        let admitted = qualify_delivered_inputs(&mut FixedPort, &inputs).unwrap();
        assert_eq!(admitted.len(), 1);
        validate_admission_cover(&inputs, &admitted).unwrap();
        assert_eq!(
            admitted[0].source.source_id, "nq-node:fixture",
            "the source principal is preserved independently of its command path"
        );
    }

    #[test]
    fn resealed_provenance_for_different_bytes_is_still_rejected() {
        let inputs = inputs();
        let DiagnosticInputStatus::Delivered { artifact } = &inputs.inputs[0].status else {
            panic!("delivered fixture");
        };
        let query = NqAdmissionQueryV1::from_artifact(artifact).unwrap();
        let mut substituted = provenance(&query);
        substituted.artifact.canonical_bytes_sha256 = digest('e');
        substituted = substituted.seal().unwrap();
        assert!(
            substituted.validate_for(&query).is_err(),
            "a valid self-hash cannot turn an admission for different bytes into source provenance"
        );
    }

    #[test]
    fn configured_source_identity_is_not_inferred_from_the_locator() {
        let inputs = inputs();
        let DiagnosticInputStatus::Delivered { artifact } = &inputs.inputs[0].status else {
            panic!("delivered fixture");
        };
        let query = NqAdmissionQueryV1::from_artifact(artifact).unwrap();
        let mut port = CommandNqAdmissionPortV1::new(
            "/untrusted/route/nq",
            "/mutable/config.toml",
            "nq-store-genesis:different-source".into(),
        )
        .unwrap();
        assert!(port
            .qualify(&query)
            .unwrap_err()
            .contains("does not match configured NQ source"));
    }

    #[cfg(unix)]
    fn command_fixture(
        response: &[u8],
        exit_status: i32,
    ) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let response_path = root.path().join("response.json");
        let arguments_path = root.path().join("arguments.txt");
        let program = root.path().join("nq");
        std::fs::write(&response_path, response).unwrap();
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\ncat '{}'\nexit {}\n",
                arguments_path.display(),
                response_path.display(),
                exit_status
            ),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = root.path().join("nq.toml");
        (root, program, config, arguments_path)
    }

    #[cfg(unix)]
    #[test]
    fn production_command_adapter_invokes_the_closed_nq_ng_boundary() {
        let inputs = inputs();
        let DiagnosticInputStatus::Delivered { artifact } = &inputs.inputs[0].status else {
            panic!("delivered fixture");
        };
        let query = NqAdmissionQueryV1::from_artifact(artifact).unwrap();
        let expected = provenance(&query);
        let response = serde_json::to_vec(&expected).unwrap();
        let (_root, program, config, arguments_path) = command_fixture(&response, 0);
        let mut port =
            CommandNqAdmissionPortV1::new(&program, &config, query.source_id.clone()).unwrap();

        assert_eq!(port.qualify(&query).unwrap(), expected);
        let arguments = std::fs::read_to_string(arguments_path).unwrap();
        assert_eq!(
            arguments.lines().collect::<Vec<_>>(),
            [
                "--config",
                config.to_str().unwrap(),
                "--json",
                "diagnostics",
                "qualify",
                query.artifact_id.as_str(),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn command_adapter_refuses_malformed_wrong_schema_trailing_and_failed_output() {
        let inputs = inputs();
        let DiagnosticInputStatus::Delivered { artifact } = &inputs.inputs[0].status else {
            panic!("delivered fixture");
        };
        let query = NqAdmissionQueryV1::from_artifact(artifact).unwrap();

        let cases = [
            (b"not-json".to_vec(), 0, "invalid provenance"),
            (
                {
                    let mut value = serde_json::to_value(provenance(&query)).unwrap();
                    value["schema"] = serde_json::json!("nq.unknown.v1");
                    serde_json::to_vec(&value).unwrap()
                },
                0,
                "unsupported NQ admission provenance schema",
            ),
            (
                {
                    let mut bytes = serde_json::to_vec(&provenance(&query)).unwrap();
                    bytes.extend_from_slice(b"\n{}");
                    bytes
                },
                0,
                "trailing data",
            ),
            (Vec::new(), 23, "refused admission provenance"),
        ];

        for (response, exit_status, expected_error) in cases {
            let (_root, program, config, _) = command_fixture(&response, exit_status);
            let mut port =
                CommandNqAdmissionPortV1::new(program, config, query.source_id.clone()).unwrap();
            let error = port.qualify(&query).unwrap_err();
            assert!(
                error.contains(expected_error),
                "expected {expected_error:?}, got {error:?}"
            );
        }
    }
}
