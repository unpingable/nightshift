use nightshiftd::nq_admission::{
    NqAdmissionArtifactV1, NqAdmissionJudgmentV1, NqAdmissionOriginV1, NqAdmissionPortV1,
    NqAdmissionProvenanceV1, NqAdmissionProviderV1, NqAdmissionQueryV1, NqAdmissionSourceV1,
    NqSourceDispositionV1,
};

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

/// In-process test authority. Production uses the configured `nq` command
/// adapter; this fixture only lets downstream tests exercise an exact binding.
#[derive(Clone, Copy)]
pub struct TestNqAdmissionPort;

impl NqAdmissionPortV1 for TestNqAdmissionPort {
    fn qualify(&mut self, query: &NqAdmissionQueryV1) -> Result<NqAdmissionProvenanceV1, String> {
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
    }
}
