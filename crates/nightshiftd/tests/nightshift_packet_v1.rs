use chrono::{Duration, TimeZone as _, Utc};
use nightshiftd::packet::{
    AuthoringIdentityV1, CampaignIdentityV1, CanonicalizationV1, ExactWorkRefV1,
    GlobalConstraintsV1, ModelRoutingV1, NightshiftPacketV1, PacketError, PredecessorRefV1,
    RepositoryCustodyV1, SourceEvidenceRefV1, SwitchyardRegistrationV1, WorkItemV1, WorkerBudgetV1,
    EXACT_WORK_PROPOSAL_SCHEMA_V1, NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1,
    NIGHTSHIFT_PACKET_SCHEMA_V1,
};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::process::Command;

type PacketMutation = Box<dyn Fn(&mut NightshiftPacketV1)>;

fn fixture() -> NightshiftPacketV1 {
    let created_at = Utc.with_ymd_and_hms(2026, 8, 29, 16, 0, 0).unwrap();
    let mut packet = NightshiftPacketV1 {
        schema: NIGHTSHIFT_PACKET_SCHEMA_V1.into(),
        packet_id: "nightshift-20260829-test".into(),
        packet_digest: String::new(),
        created_at,
        current_until: Utc.with_ymd_and_hms(2026, 8, 30, 12, 0, 0).unwrap(),
        authoring: AuthoringIdentityV1 {
            agent: "codex".into(),
            session: "test-session".into(),
            authority_basis: "operator prompt".into(),
        },
        canonicalization: CanonicalizationV1 {
            algorithm: "RFC8785-JCS".into(),
            digest_algorithm: "SHA-256".into(),
            digest_preimage: NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1.into(),
        },
        source_evidence: vec![SourceEvidenceRefV1 {
            repository: "nightshift".into(),
            branch: "main".into(),
            commit: "a".repeat(40),
            path: "README.md".into(),
            file_digest: format!("sha256:{}", "b".repeat(64)),
            predecessor_classification: "OBSERVED".into(),
        }],
        repository_custody: vec![RepositoryCustodyV1 {
            repository: "nightshift".into(),
            path: "/tmp/nightshift".into(),
            branch: "main".into(),
            commit: "a".repeat(40),
            remote: Some("origin".into()),
            remote_commit: Some("a".repeat(40)),
            worktree_clean: true,
            discrepancy: None,
        }],
        global_constraints: GlobalConstraintsV1 {
            allowed_actions: vec!["local tests".into()],
            forbidden_actions: vec!["production activation".into()],
            invariants: vec!["packet is non-authorizing".into()],
        },
        work_items: vec![WorkItemV1 {
            id: "packet".into(),
            track: "nightshift".into(),
            campaign: CampaignIdentityV1 {
                codename: "TEST-ORRERY".into(),
                canonical_slug: "test-orientation-packet".into(),
            },
            predecessor_lineage: vec![PredecessorRefV1 {
                campaign: "PREDECESSOR".into(),
                classification: "OBSERVED".into(),
                commit: "a".repeat(40),
            }],
            dependencies: vec![],
            exact_work_refs: vec![ExactWorkRefV1 {
                contract_kind: "exact_work_proposal_v1".into(),
                contract_schema: EXACT_WORK_PROPOSAL_SCHEMA_V1.into(),
                repository: "ag_ng".into(),
                branch: "campaign/example".into(),
                commit: "c".repeat(40),
                path: "proposal.json".into(),
                proposal_ref: format!("sha256:{}", "d".repeat(64)),
            }],
            entry_predicates: vec!["operator prompt present".into()],
            allowed_mutation_surfaces: vec!["docs/".into()],
            forbidden_actions: vec!["merge".into()],
            acceptance_tests: vec!["digest validates".into()],
            stop_conditions: vec!["protected approval required".into()],
            expected_receipts: vec!["validation receipt".into()],
            closeout_requirements: vec!["clean worktree".into()],
            model_routing: ModelRoutingV1 {
                class: "small".into(),
                reason: "deterministic fixture".into(),
                maximum_mutating_workers: 1,
            },
        }],
        worker_budget: WorkerBudgetV1 {
            maximum_concurrent_mutating_workers: 4,
            recursive_worker_swarms_forbidden: true,
            reserve_posture: "preserve approximately 15 percent where observable".into(),
        },
        human_question_criteria: vec!["protected approval unavailable".into()],
        switchyard: SwitchyardRegistrationV1 {
            alias: "nightshift-test".into(),
            plan_ref: String::new(),
            transport_fields: vec!["alias".into(), "plan_ref".into(), "nonce".into()],
        },
    };
    packet.seal().unwrap();
    packet
}

#[test]
fn valid_fixture_and_rendering_are_non_authorizing() {
    let packet = fixture();
    let receipt = packet.validate_at(packet.created_at).unwrap();
    assert_eq!(receipt.authority_effect, "NONE");
    assert!(packet.render_markdown().contains("non-authorizing"));
}

#[test]
fn integrity_validation_is_independent_of_evaluation_time() {
    let packet = fixture();
    packet.validate_integrity().unwrap();
    assert_eq!(
        packet.validate_at(packet.current_until + Duration::seconds(1)),
        Err(PacketError::NotCurrent)
    );
}

#[test]
fn normative_mutation_changes_digest() {
    let packet = fixture();
    let mut changed = packet.clone();
    changed.work_items[0]
        .entry_predicates
        .push("new predicate".into());
    assert_ne!(
        packet.computed_digest().unwrap(),
        changed.computed_digest().unwrap()
    );
}

#[test]
fn digest_uses_packet_specific_v1_domain_frame() {
    let packet = fixture();
    assert_eq!(
        packet.packet_digest,
        "sha256:ace1ba2ba61cf9429adf7875cade221190628282c853c2e9eaa781abed73dd10"
    );
}

#[test]
fn checked_in_closed_schema_identity_is_pinned() {
    let schema = include_bytes!("../../../schemas/nightshift.orientation-packet.v1.schema.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(schema)),
        "6b71b4ec182811c376c4b852bc6ae540e1c063d5db43d1cacefaeead9636c50f"
    );
}

#[test]
fn closed_schema_empty_string_and_collection_cases_fail_closed() {
    let cases: Vec<(&str, PacketMutation)> = vec![
        (
            "global_constraints.invariants",
            Box::new(|packet| packet.global_constraints.invariants.clear()),
        ),
        (
            "worker_budget.reserve_posture",
            Box::new(|packet| packet.worker_budget.reserve_posture.clear()),
        ),
        (
            "work_items.track",
            Box::new(|packet| packet.work_items[0].track.clear()),
        ),
        (
            "work_items.model_routing.class",
            Box::new(|packet| packet.work_items[0].model_routing.class.clear()),
        ),
        (
            "work_items.model_routing.reason",
            Box::new(|packet| packet.work_items[0].model_routing.reason.clear()),
        ),
        (
            "source_evidence.predecessor_classification",
            Box::new(|packet| packet.source_evidence[0].predecessor_classification.clear()),
        ),
        (
            "predecessor_lineage.classification",
            Box::new(|packet| {
                packet.work_items[0].predecessor_lineage[0]
                    .classification
                    .clear()
            }),
        ),
        (
            "work_items.acceptance_tests",
            Box::new(|packet| packet.work_items[0].acceptance_tests[0].clear()),
        ),
    ];

    for (field, mutate) in cases {
        let mut packet = fixture();
        mutate(&mut packet);
        assert_eq!(
            packet.validate_at(packet.created_at),
            Err(PacketError::InvalidField(field)),
            "case {field}"
        );
    }
}

#[test]
fn seal_refuses_schema_invalid_draft() {
    let mut packet = fixture();
    packet.packet_digest.clear();
    packet.switchyard.plan_ref.clear();
    packet.worker_budget.reserve_posture.clear();
    assert_eq!(
        packet.seal(),
        Err(PacketError::InvalidField("worker_budget.reserve_posture"))
    );
}

#[test]
fn cli_seal_output_is_exact_canonical_input_for_validate() {
    let directory = tempfile::tempdir().unwrap();
    let draft_path = directory.path().join("packet.draft.json");
    let sealed_path = directory.path().join("packet.v1.json");
    let mut draft = fixture();
    draft.packet_digest.clear();
    draft.switchyard.plan_ref.clear();
    std::fs::write(&draft_path, serde_json::to_vec_pretty(&draft).unwrap()).unwrap();

    let seal = Command::new(env!("CARGO_BIN_EXE_nightshift"))
        .args(["packet", "seal", "--packet"])
        .arg(&draft_path)
        .output()
        .unwrap();
    assert!(
        seal.status.success(),
        "{}",
        String::from_utf8_lossy(&seal.stderr)
    );
    assert!(!seal.stdout.ends_with(b"\n"));
    let sealed = NightshiftPacketV1::from_slice(&seal.stdout).unwrap();
    assert_eq!(seal.stdout, sealed.canonical_bytes().unwrap());
    std::fs::write(&sealed_path, &seal.stdout).unwrap();

    let validate = Command::new(env!("CARGO_BIN_EXE_nightshift"))
        .args(["packet", "validate", "--packet"])
        .arg(&sealed_path)
        .args(["--evaluated-at", "2026-08-29T16:00:00Z"])
        .output()
        .unwrap();
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn unknown_work_item_fails_closed() {
    let mut packet = fixture();
    packet.work_items[0].dependencies.push("absent".into());
    packet.seal().unwrap();
    assert_eq!(
        packet.validate_at(packet.created_at),
        Err(PacketError::UnknownWorkItem("absent".into()))
    );
}

#[test]
fn dependency_cycle_fails_closed() {
    let mut packet = fixture();
    let mut second = packet.work_items[0].clone();
    second.id = "second".into();
    second.campaign.codename = "SECOND-ORRERY".into();
    second.campaign.canonical_slug = "second-orientation-packet".into();
    second.dependencies = vec!["packet".into()];
    packet.work_items[0].dependencies = vec!["second".into()];
    packet.work_items.push(second);
    packet.seal().unwrap();
    assert_eq!(
        packet.validate_at(packet.created_at),
        Err(PacketError::DependencyCycle)
    );
}

#[test]
fn duplicate_dependency_fails_closed() {
    let mut packet = fixture();
    let mut second = packet.work_items[0].clone();
    second.id = "second".into();
    second.campaign.codename = "SECOND-ORRERY".into();
    second.campaign.canonical_slug = "second-orientation-packet".into();
    packet.work_items.push(second);
    packet.work_items[0].dependencies = vec!["second".into(), "second".into()];
    assert_eq!(
        packet.seal(),
        Err(PacketError::InvalidField("work_items.dependencies"))
    );
}

#[test]
fn stale_packet_fails_closed() {
    let packet = fixture();
    assert_eq!(
        packet.validate_at(packet.current_until + Duration::seconds(1)),
        Err(PacketError::NotCurrent)
    );
}

#[test]
fn mismatched_digest_fails_closed() {
    let mut packet = fixture();
    packet.global_constraints.invariants.push("changed".into());
    assert_eq!(
        packet.validate_at(packet.created_at),
        Err(PacketError::DigestMismatch)
    );
}

#[test]
fn changed_plan_ref_fails_closed() {
    let mut packet = fixture();
    packet.switchyard.plan_ref.push('0');
    assert_eq!(
        packet.validate_at(packet.created_at),
        Err(PacketError::InvalidField("switchyard"))
    );
}

#[test]
fn unknown_json_field_fails_closed() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("authority".into(), Value::Bool(true));
    assert!(matches!(
        NightshiftPacketV1::from_slice(&serde_json::to_vec(&value).unwrap()),
        Err(PacketError::Json(_))
    ));
}

#[test]
fn substituted_type_and_nested_unknown_field_fail_closed() {
    let mut substituted = serde_json::to_value(fixture()).unwrap();
    substituted["worker_budget"]["reserve_posture"] = Value::Bool(true);
    assert!(matches!(
        NightshiftPacketV1::from_slice(&serde_json::to_vec(&substituted).unwrap()),
        Err(PacketError::Json(_))
    ));

    let mut unknown = serde_json::to_value(fixture()).unwrap();
    unknown["work_items"][0]["model_routing"]["fallback"] = Value::String("none".into());
    assert!(matches!(
        NightshiftPacketV1::from_slice(&serde_json::to_vec(&unknown).unwrap()),
        Err(PacketError::Json(_))
    ));
}

#[test]
fn custody_nullable_fields_are_required_and_explicit_null_is_accepted() {
    let explicit_null =
        include_bytes!("../../../qualification/nightshift-packet-v1/fixtures/positive.v1.json");
    let decoded = NightshiftPacketV1::from_slice(explicit_null).unwrap();
    assert_eq!(decoded.repository_custody[0].remote, None);
    assert_eq!(decoded.repository_custody[0].remote_commit, None);
    assert_eq!(decoded.repository_custody[0].discrepancy, None);

    let directory = tempfile::tempdir().unwrap();
    for field in ["remote", "remote_commit", "discrepancy"] {
        let mut missing = serde_json::to_value(fixture()).unwrap();
        missing["packet_digest"] = Value::String(String::new());
        missing["switchyard"]["plan_ref"] = Value::String(String::new());
        missing["repository_custody"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let bytes = serde_json::to_vec(&missing).unwrap();
        assert!(matches!(
            NightshiftPacketV1::from_slice(&bytes),
            Err(PacketError::Json(_))
        ));

        let path = directory.path().join(format!("missing-{field}.json"));
        std::fs::write(&path, bytes).unwrap();
        let seal = Command::new(env!("CARGO_BIN_EXE_nightshift"))
            .args(["packet", "seal", "--packet"])
            .arg(path)
            .output()
            .unwrap();
        assert!(!seal.status.success(), "field {field}");
        assert!(seal.stdout.is_empty(), "field {field}");
    }
}

#[test]
fn committed_fixtures_have_expected_dispositions() {
    fn load(bytes: &[u8]) -> NightshiftPacketV1 {
        NightshiftPacketV1::from_slice(bytes).unwrap()
    }
    let evaluated_at = Utc.with_ymd_and_hms(2026, 8, 29, 17, 30, 0).unwrap();
    let positive = load(include_bytes!(
        "../../../qualification/nightshift-packet-v1/fixtures/positive.v1.json"
    ));
    assert_eq!(
        positive.validate_at(evaluated_at).unwrap().authority_effect,
        "NONE"
    );

    let unknown_work_item = load(include_bytes!(
        "../../../qualification/nightshift-packet-v1/fixtures/negative-unknown-work-item.v1.json"
    ));
    assert_eq!(
        unknown_work_item.validate_at(evaluated_at),
        Err(PacketError::UnknownWorkItem("missing-work-item".into()))
    );

    let stale = load(include_bytes!(
        "../../../qualification/nightshift-packet-v1/fixtures/negative-stale.v1.json"
    ));
    assert_eq!(
        stale.validate_at(evaluated_at),
        Err(PacketError::NotCurrent)
    );

    let mismatch = load(include_bytes!(
        "../../../qualification/nightshift-packet-v1/fixtures/negative-digest-mismatch.v1.json"
    ));
    assert_eq!(
        mismatch.validate_at(evaluated_at),
        Err(PacketError::DigestMismatch)
    );

    assert!(matches!(
        NightshiftPacketV1::from_slice(include_bytes!(
            "../../../qualification/nightshift-packet-v1/fixtures/negative-unknown-field.v1.json"
        )),
        Err(PacketError::Json(_))
    ));
}
