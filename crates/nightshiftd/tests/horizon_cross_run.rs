//! Cross-run integration tests for the tolerability horizon
//! A5 four-way distinction (`GOV_GAP_TOLERABILITY_HORIZON_001`).
//!
//! This file is the **acceptance center** for Phase A of the
//! horizon-consumer wiring (per chatty's 2026-04-23 guardrail #3:
//! "if that test is clean, A is real").
//!
//! Each scenario simulates two runs against a shared persistent
//! SQLite store, a fixed `FixtureHorizonPolicySource` (Night Shift's
//! local horizon-declaration surface — see `horizon_policy.rs` for
//! the invariant that horizon is producer-local, not a Governor
//! lookup), and a controlled wall-clock. Run A writes (or does not
//! write) a tolerance record via `apply_horizon_outcomes`; run B
//! reads that state via `process_horizon` and must produce the
//! correct `HorizonAction`.
//!
//! Four cases:
//!
//! 1. `tolerated_active` — run A defers under `horizon=hours` with
//!    future expiry; run B, still before expiry, with matching
//!    basis, continues to defer. Tolerance record remains present.
//! 2. `expired_tolerance` — run A defers; run B is past expiry with
//!    matching basis; outcome is `EscalateExpired` carrying the
//!    prior record as lineage. Tolerance record is cleared.
//! 3. `basis_invalidated` — run A defers under basis hash X; run B
//!    sees the same finding with a declaration under hash Y (still
//!    future expiry); outcome is `EscalateBasisInvalidated` with
//!    both sides surfaced. Tolerance record is cleared.
//! 4. `fresh_arrival` — no run A deferral. Run B sees a finding
//!    with no prior tolerance record and no horizon declaration;
//!    outcome is `ActOnVerdict(Missing)`. No tolerance record ever
//!    written.
//!
//! Phase B adds real Governor RPC plumbing (`check_policy`,
//! `record_receipt`, `authorize_transition`) — orthogonal to this
//! file's decision logic. The horizon acquisition surface stays
//! NS-internal; what Phase B adds is the archival path: when NS
//! defers, it forwards that tolerance declaration to Governor via
//! `record_receipt` so the audit trail exists.

use chrono::{DateTime, TimeZone, Utc};

use nightshiftd::bundle::{
    CaptureInput, Freshness, InputStatus, InvalidationRule, ReconciliationPhase,
    ReconciliationResult, ReconciliationSummary, RelianceClass, RelianceScope, ValidFor,
};
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::horizon::{HorizonAction, HorizonBlock, HorizonClass};
use nightshiftd::horizon_policy::{FixtureHorizonPolicySource, HorizonDeclaration};
use nightshiftd::reconcile_horizon::{apply_horizon_outcomes, process_horizon};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::Store;

// ---- helpers ----

fn finding_key() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn input_id_for(key: &FindingKey) -> String {
    format!("nq:finding:{}:{}", key.detector, key.subject)
}

fn capture_input_for(key: &FindingKey, captured_at: DateTime<Utc>) -> CaptureInput {
    let snap = FindingSnapshot {
        finding_key: key.clone(),
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 6,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 14, 32, 15).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: 39000,
        captured_at,
        evidence_hash: "sha256:baseline".into(),
    };
    CaptureInput {
        input_id: input_id_for(key),
        source: "nq".into(),
        kind: "nq_finding_snapshot".into(),
        status: InputStatus::Observed,
        evidence_hash: "sha256:baseline".into(),
        freshness: Freshness {
            captured_at,
            expires_at: None,
            invalidates_if: vec![InvalidationRule::FindingAbsentForNGenerations { n: 1 }],
        },
        payload_ref: "ledger://test".into(),
        admissible_for: vec![],
        inadmissible_for: vec![],
        captured_finding_snapshot: Some(snap),
    }
}

fn reconciliation_phase_for(input_id: &str, at: DateTime<Utc>) -> ReconciliationPhase {
    ReconciliationPhase {
        reconciled_at: at,
        reconciled_by: "test".into(),
        results: vec![ReconciliationResult {
            input_id: input_id.into(),
            status: InputStatus::Committed,
            reliance_class: RelianceClass::Authoritative,
            scope: RelianceScope {
                run_id: "run_xxx".into(),
                valid_for: vec![
                    ValidFor::Authorization,
                    ValidFor::Proposal,
                    ValidFor::Diagnosis,
                ],
            },
            previous_evidence_hash: Some("sha256:baseline".into()),
            current_evidence_hash: Some("sha256:baseline".into()),
            notes: None,
            concurrent_activity: None,
            current_finding_snapshot: None,
        }],
        summary: ReconciliationSummary {
            ok_to_proceed: true,
            ..Default::default()
        },
    }
}

fn declaration_with_timed_horizon(
    key: &FindingKey,
    basis_id: &str,
    basis_hash: &str,
    class: HorizonClass,
    expiry: DateTime<Utc>,
) -> HorizonDeclaration {
    HorizonDeclaration {
        finding_key: key.clone(),
        horizon: HorizonBlock {
            class,
            basis_id: Some(basis_id.into()),
            basis_hash: Some(basis_hash.into()),
            expiry: Some(expiry),
        },
    }
}

// ---- the four-way distinction, one test each ----

/// **Case 1: tolerated_active.**
/// Run A defers under `horizon=hours` with expiry at t0+4h. Run B
/// runs at t0+2h, still before expiry, with matching basis. Outcome
/// must remain `Defer` — tolerance persists through the second run
/// because basis hasn't changed and expiry hasn't arrived.
#[test]
fn tolerated_active_continues_to_defer_before_expiry() {
    let store = SqliteStore::open_in_memory().unwrap();
    let key = finding_key();

    let t0 = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
    let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
    let t_mid = Utc.with_ymd_and_hms(2026, 4, 23, 14, 0, 0).unwrap();

    let declaration = declaration_with_timed_horizon(
        &key,
        "maintenance-window-042",
        "sha256:basis-abc",
        HorizonClass::Hours,
        expiry,
    );
    let policy = FixtureHorizonPolicySource::from_declarations(vec![declaration]);

    let input = capture_input_for(&key, t0);
    let phase_a = reconciliation_phase_for(&input.input_id, t0);
    let outcomes_a =
        process_horizon(&phase_a, std::slice::from_ref(&input), &policy, &store, t0).unwrap();
    assert_eq!(outcomes_a.len(), 1);
    assert!(matches!(outcomes_a[0].action, HorizonAction::Defer { .. }));
    apply_horizon_outcomes(&outcomes_a, &store, "run_a", t0).unwrap();
    assert!(
        store.load_tolerance(&key).unwrap().is_some(),
        "run A must write tolerance record"
    );

    let phase_b = reconciliation_phase_for(&input.input_id, t_mid);
    let outcomes_b = process_horizon(
        &phase_b,
        std::slice::from_ref(&input),
        &policy,
        &store,
        t_mid,
    )
    .unwrap();
    assert_eq!(outcomes_b.len(), 1);
    match &outcomes_b[0].action {
        HorizonAction::Defer {
            until,
            basis_id,
            basis_hash,
            class,
        } => {
            assert_eq!(*until, expiry);
            assert_eq!(basis_id, "maintenance-window-042");
            assert_eq!(basis_hash, "sha256:basis-abc");
            assert_eq!(*class, HorizonClass::Hours);
        }
        other => panic!("run B expected Defer (tolerated_active), got {other:?}"),
    }
    apply_horizon_outcomes(&outcomes_b, &store, "run_b", t_mid).unwrap();
    assert!(
        store.load_tolerance(&key).unwrap().is_some(),
        "tolerance record must persist through tolerated_active case"
    );
}

/// **Case 2: expired_tolerance.**
/// Run A defers at t0; run B runs at t0+5h (past the 4h expiry)
/// with still-matching basis. Outcome must be `EscalateExpired`
/// with the prior tolerance surfaced as lineage. This is the
/// canonical failure mode the A5 write obligation prevents —
/// without the prior record, run B would see a fresh finding.
#[test]
fn expired_tolerance_escalates_with_prior_lineage() {
    let store = SqliteStore::open_in_memory().unwrap();
    let key = finding_key();

    let t0 = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
    let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
    let t_past = Utc.with_ymd_and_hms(2026, 4, 23, 17, 0, 0).unwrap();

    let declaration = declaration_with_timed_horizon(
        &key,
        "maintenance-window-042",
        "sha256:basis-abc",
        HorizonClass::Hours,
        expiry,
    );
    let policy = FixtureHorizonPolicySource::from_declarations(vec![declaration]);
    let input = capture_input_for(&key, t0);

    let phase_a = reconciliation_phase_for(&input.input_id, t0);
    let outcomes_a =
        process_horizon(&phase_a, std::slice::from_ref(&input), &policy, &store, t0).unwrap();
    assert!(matches!(outcomes_a[0].action, HorizonAction::Defer { .. }));
    apply_horizon_outcomes(&outcomes_a, &store, "run_a", t0).unwrap();

    let phase_b = reconciliation_phase_for(&input.input_id, t_past);
    let outcomes_b = process_horizon(
        &phase_b,
        std::slice::from_ref(&input),
        &policy,
        &store,
        t_past,
    )
    .unwrap();
    match &outcomes_b[0].action {
        HorizonAction::EscalateExpired { prior } => {
            assert_eq!(prior.basis_id, "maintenance-window-042");
            assert_eq!(prior.basis_hash, "sha256:basis-abc");
            assert_eq!(prior.prior_class, HorizonClass::Hours);
            assert_eq!(prior.expired_at, expiry);
        }
        other => panic!(
            "run B expected EscalateExpired carrying prior lineage, got {other:?}"
        ),
    }
    apply_horizon_outcomes(&outcomes_b, &store, "run_b", t_past).unwrap();
    assert!(
        store.load_tolerance(&key).unwrap().is_none(),
        "EscalateExpired must clear the consumed tolerance record"
    );
}

/// **Case 3: basis_invalidated.**
/// Run A defers under basis hash X. Before expiry, run B sees a
/// receipt declaring basis hash Y (same class, same future expiry)
/// — the justification has changed. Outcome must be
/// `EscalateBasisInvalidated`, not `Defer`, with both sides
/// surfaced. Tolerance record is cleared.
#[test]
fn basis_invalidated_escalates_with_both_sides_surfaced() {
    let store = SqliteStore::open_in_memory().unwrap();
    let key = finding_key();

    let t0 = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
    let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
    let t_mid = Utc.with_ymd_and_hms(2026, 4, 23, 14, 0, 0).unwrap();

    // Run A: governor emits receipt under basis hash "old".
    let policy_a = FixtureHorizonPolicySource::from_declarations(vec![declaration_with_timed_horizon(
        &key,
        "basis-old",
        "sha256:hash-old",
        HorizonClass::Hours,
        expiry,
    )]);
    let input = capture_input_for(&key, t0);
    let phase_a = reconciliation_phase_for(&input.input_id, t0);
    let outcomes_a =
        process_horizon(&phase_a, std::slice::from_ref(&input), &policy_a, &store, t0).unwrap();
    apply_horizon_outcomes(&outcomes_a, &store, "run_a", t0).unwrap();
    assert_eq!(
        store.load_tolerance(&key).unwrap().unwrap().basis_hash,
        "sha256:hash-old"
    );

    // Run B: governor now emits the same finding with a new basis
    // hash. Expiry identical, still in the future.
    let policy_b = FixtureHorizonPolicySource::from_declarations(vec![declaration_with_timed_horizon(
        &key,
        "basis-new",
        "sha256:hash-new",
        HorizonClass::Hours,
        expiry,
    )]);
    let phase_b = reconciliation_phase_for(&input.input_id, t_mid);
    let outcomes_b = process_horizon(
        &phase_b,
        std::slice::from_ref(&input),
        &policy_b,
        &store,
        t_mid,
    )
    .unwrap();
    match &outcomes_b[0].action {
        HorizonAction::EscalateBasisInvalidated {
            prior,
            current_basis_hash,
        } => {
            assert_eq!(prior.basis_hash, "sha256:hash-old");
            assert_eq!(prior.basis_id, "basis-old");
            assert_eq!(current_basis_hash, "sha256:hash-new");
        }
        other => panic!(
            "run B expected EscalateBasisInvalidated, got {other:?}"
        ),
    }
    apply_horizon_outcomes(&outcomes_b, &store, "run_b", t_mid).unwrap();
    assert!(
        store.load_tolerance(&key).unwrap().is_none(),
        "EscalateBasisInvalidated must clear the invalidated record"
    );
}

/// **Case 4: fresh_arrival.**
/// No prior tolerance record. Run B observes a finding for which
/// Governor has not emitted a horizon-bearing receipt (or no
/// receipt at all). Outcome must be `ActOnVerdict { reason:
/// Missing }` — fail-closed to `now`. Missing ≠ tolerable.
#[test]
fn fresh_arrival_fail_closes_to_act_on_verdict() {
    let store = SqliteStore::open_in_memory().unwrap();
    let key = finding_key();
    let t = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();

    // No tolerance record pre-exists. Governor has no receipt for
    // this finding (simulates "no horizon declared").
    assert!(store.load_tolerance(&key).unwrap().is_none());
    let policy = FixtureHorizonPolicySource::from_declarations(vec![]);

    let input = capture_input_for(&key, t);
    let phase = reconciliation_phase_for(&input.input_id, t);
    let outcomes =
        process_horizon(&phase, std::slice::from_ref(&input), &policy, &store, t).unwrap();
    match &outcomes[0].action {
        HorizonAction::ActOnVerdict { reason } => {
            assert_eq!(
                *reason,
                nightshiftd::horizon::ActReason::Missing,
                "fresh_arrival with no receipt must fail-closed as Missing"
            );
        }
        other => panic!("fresh_arrival expected ActOnVerdict(Missing), got {other:?}"),
    }
    apply_horizon_outcomes(&outcomes, &store, "run_b", t).unwrap();
    assert!(
        store.load_tolerance(&key).unwrap().is_none(),
        "fresh_arrival must never write a tolerance record"
    );
}

/// Phase A acceptance summary — all four cases executed
/// back-to-back against a shared store, each with its own
/// governor fixture. This test exists primarily to prove there's
/// no cross-contamination between scenarios when they share a
/// persistent backend: each case keys off its own finding, and
/// the four outcomes are distinguishable end-to-end.
#[test]
fn four_way_distinction_is_observable_end_to_end() {
    let store = SqliteStore::open_in_memory().unwrap();

    let mk_key = |detector: &str, subject: &str| FindingKey {
        source: "nq".into(),
        detector: detector.into(),
        subject: subject.into(),
    };
    let mk_input = |k: &FindingKey, at: DateTime<Utc>| capture_input_for(k, at);
    let mk_phase = |input_id: &str, at: DateTime<Utc>| reconciliation_phase_for(input_id, at);

    let t0 = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
    let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
    let t_mid = Utc.with_ymd_and_hms(2026, 4, 23, 14, 0, 0).unwrap();
    let t_past = Utc.with_ymd_and_hms(2026, 4, 23, 17, 0, 0).unwrap();

    // Four distinct findings, one per case.
    let k_active = mk_key("wal_bloat", "host-a:/db");
    let k_expired = mk_key("wal_bloat", "host-b:/db");
    let k_invalid = mk_key("wal_bloat", "host-c:/db");
    let k_fresh = mk_key("wal_bloat", "host-d:/db");

    let input_active = mk_input(&k_active, t0);
    let input_expired = mk_input(&k_expired, t0);
    let input_invalid = mk_input(&k_invalid, t0);
    let input_fresh = mk_input(&k_fresh, t0);

    // Governor at run A: emits receipts for the three that will
    // start under tolerance. The "fresh" finding has no receipt.
    let policy_a = FixtureHorizonPolicySource::from_declarations(vec![
        declaration_with_timed_horizon(
            &k_active,
            "bw-active",
            "sha256:active-1",
            HorizonClass::Hours,
            expiry,
        ),
        declaration_with_timed_horizon(
            &k_expired,
            "bw-expired",
            "sha256:expired-1",
            HorizonClass::Hours,
            expiry,
        ),
        declaration_with_timed_horizon(
            &k_invalid,
            "bw-invalid",
            "sha256:invalid-old",
            HorizonClass::Hours,
            expiry,
        ),
    ]);

    // Run A: write tolerance for the first three.
    for (input, name) in [
        (&input_active, "active"),
        (&input_expired, "expired"),
        (&input_invalid, "invalid"),
    ] {
        let phase = mk_phase(&input.input_id, t0);
        let outcomes = process_horizon(
            &phase,
            std::slice::from_ref(input),
            &policy_a,
            &store,
            t0,
        )
        .unwrap();
        assert!(
            matches!(outcomes[0].action, HorizonAction::Defer { .. }),
            "run A for {name}: expected Defer, got {:?}",
            outcomes[0].action
        );
        apply_horizon_outcomes(&outcomes, &store, "run_a", t0).unwrap();
    }

    // Governor at run B: rotates the basis on `invalid` (same
    // expiry); leaves `active` and `expired` receipts unchanged;
    // still no receipt for `fresh`.
    let policy_b = FixtureHorizonPolicySource::from_declarations(vec![
        declaration_with_timed_horizon(
            &k_active,
            "bw-active",
            "sha256:active-1",
            HorizonClass::Hours,
            expiry,
        ),
        declaration_with_timed_horizon(
            &k_expired,
            "bw-expired",
            "sha256:expired-1",
            HorizonClass::Hours,
            expiry,
        ),
        declaration_with_timed_horizon(
            &k_invalid,
            "bw-invalid-new",
            "sha256:invalid-new",
            HorizonClass::Hours,
            expiry,
        ),
    ]);

    // Run B cases exercise all four outcomes at different clocks.
    let cases: [(&CaptureInput, DateTime<Utc>, &str); 4] = [
        (&input_active, t_mid, "tolerated_active"),
        (&input_expired, t_past, "expired_tolerance"),
        (&input_invalid, t_mid, "basis_invalidated"),
        (&input_fresh, t_mid, "fresh_arrival"),
    ];

    let mut seen: Vec<String> = Vec::new();
    for (input, now, name) in cases {
        let phase = mk_phase(&input.input_id, now);
        let outcomes = process_horizon(
            &phase,
            std::slice::from_ref(input),
            &policy_b,
            &store,
            now,
        )
        .unwrap();
        let kind = match &outcomes[0].action {
            HorizonAction::Defer { .. } => "defer",
            HorizonAction::EscalateExpired { .. } => "escalate_expired",
            HorizonAction::EscalateBasisInvalidated { .. } => "escalate_basis_invalidated",
            HorizonAction::ActOnVerdict { .. } => "act_on_verdict",
            HorizonAction::RenderNoIntervene { .. } => "render_no_intervene",
            HorizonAction::RenderHolding { .. } => "render_holding",
        };
        seen.push(format!("{name}={kind}"));
        apply_horizon_outcomes(&outcomes, &store, "run_b", now).unwrap();
    }

    // The ordering + identity of outcomes is the proof: four
    // different findings produce four different outcome kinds in
    // one store lifetime, and the basis/expiry state is what
    // drives each decision.
    assert_eq!(
        seen,
        vec![
            "tolerated_active=defer".to_string(),
            "expired_tolerance=escalate_expired".to_string(),
            "basis_invalidated=escalate_basis_invalidated".to_string(),
            "fresh_arrival=act_on_verdict".to_string(),
        ]
    );

    // Final store state: tolerance records remain only for the
    // still-active case; the other two cleared by escalation; the
    // fresh one never written.
    assert!(store.load_tolerance(&k_active).unwrap().is_some());
    assert!(store.load_tolerance(&k_expired).unwrap().is_none());
    assert!(store.load_tolerance(&k_invalid).unwrap().is_none());
    assert!(store.load_tolerance(&k_fresh).unwrap().is_none());
}
