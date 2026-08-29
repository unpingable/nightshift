use chrono::{Duration, TimeZone as _, Utc};
use nightshiftd::packet::{
    AuthoringIdentityV1, CampaignIdentityV1, CanonicalizationV1, ExactWorkRefV1,
    GlobalConstraintsV1, ModelRoutingV1, NightshiftPacketV1, PacketError, PredecessorRefV1,
    RepositoryCustodyV1, SourceEvidenceRefV1, SwitchyardRegistrationV1, WorkItemV1, WorkerBudgetV1,
    EXACT_WORK_PROPOSAL_SCHEMA_V1, NIGHTSHIFT_PACKET_SCHEMA_V1,
};
use serde_json::Value;

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
            digest_preimage: "packet object with packet_digest and switchyard.plan_ref omitted"
                .into(),
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
