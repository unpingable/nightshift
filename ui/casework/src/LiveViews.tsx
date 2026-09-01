import { useEffect, useState } from "react";

import { getLiveProviderExecution, getLiveRaw, getLiveRun, getLiveRunIndex } from "./api";
import { DefinitionGrid, Exact, Field, Section, StringList } from "./components";
import type { CaseworkLiveProviderExecution, CaseworkLiveRun, LiveProviderExecutionIdentity, LiveQuestion } from "./contract";
import { liveProviderExecutionPath, liveQuestionPath, liveRunPath, liveWorkItemPath } from "./router";

function Link({ href, children }: { href: string; children: React.ReactNode }) {
  return <a href={href}>{children}</a>;
}

function useRemote<T>(load: () => Promise<T>, dependencies: unknown[]) {
  const [state, setState] = useState<{ data?: T; error?: string; loading: boolean }>({ loading: true });
  useEffect(() => {
    let active = true;
    setState({ loading: true });
    load().then(
      (data) => active && setState({ data, loading: false }),
      (error: unknown) => active && setState({ error: error instanceof Error ? error.message : String(error), loading: false }),
    );
    return () => { active = false; };
    // The caller supplies a stable dependency list for the exact source.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dependencies);
  return state;
}

function ScreenState({ loading, error }: { loading: boolean; error?: string }) {
  if (loading) return <p role="status" className="screen-state">Loading exact live projection…</p>;
  return <p role="alert" className="screen-state error">{error ?? "Live record not found."}</p>;
}

function StateCounts({ counts }: { counts: Record<string, number> }) {
  return <dl className="state-counts" aria-label="Live scheduler-state counts">
    {Object.entries(counts).map(([name, count]) => <div key={name}><dt><Exact wrap>{name}</Exact></dt><dd>{count}</dd></div>)}
  </dl>;
}

export function ActiveRunIndex() {
  const state = useRemote(getLiveRunIndex, []);
  if (!state.data) return <ScreenState loading={state.loading} error={state.error} />;
  return <Section title="Active foreman runs">
    <p>Transaction-consistent mechanism state from explicitly registered query-only foreman stores. No scheduler state is a campaign result.</p>
    <div className="run-ledger">
      {state.data.runs.map((run) => <article className="run-card" key={run.navigation_id}>
        <header><div><p className="eyebrow">Live foreman run</p><h2><Link href={liveRunPath(run.navigation_id)}>{run.packet_id}</Link></h2></div><Exact>{run.lifecycle}</Exact></header>
        <DefinitionGrid>
          <Field label="Exact run identity"><Exact wrap>{run.run_id}</Exact></Field>
          <Field label="Packet digest"><Exact wrap>{run.packet_digest}</Exact></Field>
          <Field label="Projection digest"><Exact wrap>{run.projection_digest}</Exact></Field>
        </DefinitionGrid>
        <StateCounts counts={run.scheduler_state_counts} />
      </article>)}
      {state.data.runs.length === 0 && <p className="empty">No explicit foreman stores are registered.</p>}
    </div>
  </Section>;
}

function LiveHeader({ run }: { run: CaseworkLiveRun }) {
  const base = liveRunPath(run.navigation_id);
  return <>
    <div className="breadcrumbs"><Link href="/">Casework index</Link><span aria-hidden="true">/</span><Exact>{run.packet.packet_id}</Exact></div>
    <header className="page-heading compact">
      <p className="eyebrow">Active foreman mechanism · evaluated <time dateTime={run.evaluated_at}>{run.evaluated_at}</time></p>
      <h1>{run.packet.packet_id}</h1>
      <p><Exact wrap>{run.run_id}</Exact></p>
    </header>
    {run.sealed_case_run_id && <p><Link href={`/runs/${encodeURIComponent(run.sealed_case_run_id)}`}>Open byte-matched sealed receipt case</Link></p>}
    <nav aria-label="Active run sections" className="run-nav">
      <Link href={base}>Work items</Link>
      <Link href={base + "/events"}>Event timeline</Link>
      <Link href={liveProviderExecutionPath(run.navigation_id)}>Provider execution</Link>
      <Link href={base + "/raw"}>Raw sources</Link>
    </nav>
  </>;
}

export function LiveRunView({ navigationId }: { navigationId: string }) {
  const state = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const run = state.data;
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  return <main id="main" tabIndex={-1} className="page wide"><LiveHeader run={run} />
    <section className="summary-strip" aria-label="Live run facts">
      <div><span>Lifecycle</span><Exact>{run.foreman.lifecycle}</Exact></div>
      <div><span>Packet currentness</span><Exact>{run.packet.currentness}</Exact></div>
      <div><span>Admission currentness</span><Exact>{run.admission.currentness}</Exact></div>
      <div><span>Resource claims</span><strong>{run.resource_claims.length}</strong></div>
      <div><span>Events</span><strong>{run.events.length}</strong></div>
    </section>
    <Section title="Work-item mechanism ledger">
      <div className="table-scroll"><table className="ledger-table"><thead><tr><th>Campaign</th><th>Scheduler state</th><th>Attempt</th><th>Resources</th><th>Accepted receipt</th><th>Questions</th></tr></thead><tbody>
        {run.work_items.map((item) => <tr key={item.work_item_id}><th scope="row"><Link href={liveWorkItemPath(run.navigation_id, item.work_item_id)}>{item.campaign_codename}</Link><small>{item.campaign_slug}</small></th><td><Exact>{item.scheduler_state}</Exact></td><td><Exact wrap>{item.active_attempt_id ?? "none"}</Exact></td><td><StringList values={item.resource_lock_keys} /></td><td><Exact>{item.accepted_receipt_kind ?? "absent"}</Exact></td><td>{item.human_questions.length}</td></tr>)}
      </tbody></table></div>
    </Section>
    <Section title="Resource ownership"><div className="record-stack">{run.resource_claims.map((claim) => <DefinitionGrid key={claim.resource_lock_key}><Field label="Resource"><Exact wrap>{claim.resource_lock_key}</Exact></Field><Field label="Work item"><Exact>{claim.work_item_id}</Exact></Field><Field label="Attempt"><Exact wrap>{claim.attempt_id}</Exact></Field></DefinitionGrid>)}{run.resource_claims.length === 0 && <p className="empty">No resource claims retained.</p>}</div></Section>
    <Section title="Recorded provider-capacity mechanism">
      <DefinitionGrid>
        <Field label="Binding status"><Exact>{run.provider_capacity.status}</Exact></Field>
        <Field label="Policy reference"><Exact>{run.execution_profile.budget_policy_ref}</Exact></Field>
        <Field label="Requirement digest"><Exact wrap>{run.provider_capacity.requirement?.capacity_requirement_digest ?? "not recorded"}</Exact></Field>
        <Field label="Provider"><Exact>{run.provider_capacity.requirement?.provider_id ?? "not recorded"}</Exact></Field>
      </DefinitionGrid>
      <p>{run.provider_capacity.explanation}</p>
      {run.provider_capacity.attempts.length > 0 && <div className="table-scroll"><table className="ledger-table"><thead><tr><th>Attempt</th><th>Provider / model bindings</th><th>State / disposition</th><th>Source / confidence</th><th>Times / currentness</th><th>Owner-domain digests</th><th>Exact raw digests</th></tr></thead><tbody>
        {run.provider_capacity.attempts.map((attempt) => <tr key={attempt.attempt_id}><th scope="row"><Exact wrap>{attempt.work_item_id}</Exact><small>{attempt.attempt_id} · journal {attempt.journal_sequence}</small></th><td><Exact wrap>{attempt.provider_id} · packet {attempt.packet_model_class} · profile {attempt.profile_model_class} · {attempt.cost_class}</Exact></td><td><Exact wrap>{attempt.capacity_state} · {attempt.admission_disposition}</Exact></td><td><Exact wrap>{attempt.source_class} · {attempt.confidence} · {attempt.observation_disposition}</Exact></td><td><Exact wrap>recorded {attempt.recorded_at} · evaluated {attempt.evaluated_at} · observed {attempt.observed_at} → {attempt.expires_at} · decision {attempt.decision_at} · {attempt.currentness}</Exact></td><td><Exact wrap>{attempt.capacity_admission_digest} · {attempt.observation_digest} · {attempt.policy_digest} · {attempt.decision_digest}</Exact></td><td><Exact wrap>{attempt.admission_exact_bytes_sha256} · {attempt.observation_exact_bytes_sha256} · {attempt.policy_exact_bytes_sha256} · {attempt.decision_exact_bytes_sha256}</Exact></td></tr>)}
      </tbody></table></div>}
    </Section>
  </main>;
}

function EventRawLink({ navigationId, sequence }: { navigationId: string; sequence: number }) {
  return <a target="_blank" rel="noreferrer" href={`/api/v1/active-runs/${encodeURIComponent(navigationId)}/events/${sequence}/raw`}>Exact event {sequence} bytes</a>;
}

function ExecutionIdentity({ identity }: { identity: LiveProviderExecutionIdentity }) {
  return <DefinitionGrid>
    <Field label="Provider / model"><Exact wrap>{identity.provider_id} · {identity.model_id}</Exact></Field>
    <Field label="App Server session"><Exact wrap>{identity.app_server_session_identity}</Exact></Field>
    <Field label="Thread / turn"><Exact wrap>{identity.thread_id} · {identity.turn_id}</Exact></Field>
    <Field label="First response"><Exact wrap>{identity.first_response_id}</Exact></Field>
  </DefinitionGrid>;
}

function ProviderExecutionAbsence({ projection }: { projection: CaseworkLiveProviderExecution }) {
  return <Section title="Provider-execution history absence">
    <DefinitionGrid>
      <Field label="Exact status"><Exact>{projection.status}</Exact></Field>
      <Field label="Independent FUEL status"><Exact>{projection.independent_provider_capacity_status}</Exact></Field>
      <Field label="Projection digest"><Exact wrap>{projection.projection_digest}</Exact></Field>
      <Field label="Packet digest"><Exact wrap>{projection.packet_digest}</Exact></Field>
      <Field label="Evaluated at"><time dateTime={projection.evaluated_at}>{projection.evaluated_at}</time></Field>
      <Field label="Projection boundary"><Exact>{projection.authority_effect}</Exact></Field>
    </DefinitionGrid>
    <p className="empty">{projection.explanation}</p>
  </Section>;
}

export function LiveProviderExecutionView({ navigationId }: { navigationId: string }) {
  const runState = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const projectionState = useRemote(() => getLiveProviderExecution(navigationId), [navigationId]);
  const run = runState.data;
  const projection = projectionState.data;
  if (!run || !projection) return <ScreenState loading={runState.loading || projectionState.loading} error={runState.error ?? projectionState.error} />;
  const requirement = projection.requirement;
  const rawBase = liveRunPath(navigationId) + "/raw";
  return <main id="main" tabIndex={-1} className="page wide"><LiveHeader run={run} />
    <header className="record-heading"><div><p className="eyebrow">Read-only scheduling mechanism</p><h1>Provider execution availability</h1><p>{projection.explanation}</p></div></header>
    <section className="summary-strip" aria-label="Provider-execution projection facts">
      <div><span>Exact history status</span><Exact>{projection.status}</Exact></div>
      <div><span>Independent FUEL status</span><Exact>{projection.independent_provider_capacity_status}</Exact></div>
      <div><span>Dispatch occurrences</span><strong>{projection.dispatches.length}</strong></div>
      <div><span>Disposition records</span><strong>{projection.dispositions.length}</strong></div>
      <div><span>Evaluated at</span><time dateTime={projection.evaluated_at}>{projection.evaluated_at}</time></div>
    </section>
    {projection.status === "NOT_RECORDED_BY_FOREMAN" || !requirement ? <ProviderExecutionAbsence projection={projection} /> : <>
      <Section title="Exact requirement and ordered model selections">
        <DefinitionGrid>
          <Field label="Run / packet"><Exact wrap>{projection.run_id} · {projection.packet_digest}</Exact></Field>
          <Field label="Projection schema / digest"><Exact wrap>{projection.schema} · {projection.projection_digest}</Exact></Field>
          <Field label="Requirement sequence / digest"><Exact wrap>{requirement.journal_sequence} · {requirement.requirement_digest}</Exact></Field>
          <Field label="Policy identity"><Exact wrap>{requirement.policy_id} · {requirement.policy_digest}</Exact></Field>
          <Field label="Provider"><Exact>{requirement.provider_id}</Exact></Field>
          <Field label="Adapter identity"><Exact wrap>{requirement.adapter_id} · {requirement.adapter_protocol} · {requirement.adapter_version}</Exact></Field>
          <Field label="Adapter executable"><Exact wrap>{requirement.adapter_executable_identity}</Exact></Field>
          <Field label="Owner heads"><Exact wrap>{requirement.codex_owner_head} · {requirement.provider_admission_owner_head}</Exact></Field>
          <Field label="Owner schema / fixture"><Exact wrap>{requirement.provider_admission_schema_sha256} · {requirement.deterministic_fixture_sha256}</Exact></Field>
          <Field label="Admitted at"><time dateTime={requirement.admitted_at}>{requirement.admitted_at}</time></Field>
          <Field label="Exact requirement / policy bytes"><Exact wrap>{requirement.requirement_exact_bytes_sha256} · {requirement.policy_exact_bytes_sha256}</Exact></Field>
          <Field label="Parked lock policy"><Exact>{requirement.parked_resource_lock_policy}</Exact></Field>
          <Field label="Ordered fallback permitted"><Exact>{String(requirement.allow_ordered_model_fallback)}</Exact></Field>
          <Field label="Automatic semantic retry"><Exact>{String(requirement.automatic_semantic_retry)}</Exact></Field>
          <Field label="Approval response authorized"><Exact>{String(requirement.approval_response_authorized)}</Exact></Field>
          <Field label="Projection boundary"><Exact>{requirement.authority_effect}</Exact></Field>
        </DefinitionGrid>
        <div className="record-stack">{Object.entries(requirement.work_item_model_selections).map(([workItem, selections]) => <article className="custody-row" key={workItem}><h3><Link href={liveWorkItemPath(navigationId, workItem)}>{workItem}</Link></h3><ol start={0}>{selections.map((selection, ordinal) => <li key={`${selection.provider_id}:${selection.model_id}:${ordinal}`}><Exact wrap>ordinal {ordinal} · {selection.provider_id} · {selection.model_id} · {selection.model_class}</Exact></li>)}</ol></article>)}</div>
      </Section>
      <Section title="Work attempts and fresh dispatch occurrences"><div className="record-stack">{projection.dispatches.map((dispatch) => <article className="custody-row" key={dispatch.dispatch_occurrence_id}><h3>{dispatch.dispatch_occurrence_id}</h3><DefinitionGrid>
        <Field label="Work item / stable attempt"><Exact wrap>{dispatch.work_item_id} · {dispatch.work_attempt_id}</Exact></Field>
        <Field label="Dispatch occurrence / ordinal"><Exact wrap>{dispatch.dispatch_occurrence_id} · {dispatch.dispatch_ordinal}</Exact></Field>
        <Field label="Selected model ordinal"><Exact>{dispatch.selected_model_ordinal}</Exact></Field>
        <Field label="Provider / model / class"><Exact wrap>{dispatch.provider_id} · {dispatch.model_id} · {dispatch.model_class}</Exact></Field>
        <Field label="Adapter"><Exact wrap>{dispatch.adapter_id} · {dispatch.adapter_version} · {dispatch.adapter_protocol}</Exact></Field>
        <Field label="Process occurrence / session"><Exact wrap>{dispatch.adapter_process_occurrence_id} · {dispatch.app_server_session_identity}</Exact></Field>
        <Field label="Worker start / brief"><Exact wrap>{dispatch.worker_start_request_digest} · {dispatch.worker_brief_digest}</Exact></Field>
        <Field label="Dispatch digest"><Exact wrap>{dispatch.dispatch_digest}</Exact></Field>
        <Field label="Opened at"><time dateTime={dispatch.opened_at}>{dispatch.opened_at}</time></Field>
        <Field label="Exact start / dispatch bytes"><Exact wrap>{dispatch.start_request_exact_bytes_sha256} · {dispatch.dispatch_exact_bytes_sha256}</Exact></Field>
        <Field label="Execution identity absent at start"><Exact>{String(dispatch.provider_execution_identity_absent_at_start)}</Exact></Field>
        <Field label="Journal custody"><Exact wrap>{dispatch.journal_event_id} · {dispatch.journal_retained_raw_digest} · {dispatch.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={dispatch.journal_sequence} /></Field>
      </DefinitionGrid></article>)}</div></Section>
      <Section title="Exact provider-admission dispositions"><div className="record-stack">{projection.dispositions.map((disposition) => <article className="custody-row" key={disposition.disposition_digest}><h3><Exact>{disposition.mechanism_state}</Exact></h3><DefinitionGrid>
        <Field label="Work item / attempt"><Exact wrap>{disposition.work_item_id} · {disposition.work_attempt_id}</Exact></Field>
        <Field label="Dispatch occurrence / digest"><Exact wrap>{disposition.dispatch_occurrence_id} · {disposition.dispatch_digest}</Exact></Field>
        <Field label="Disposition / reconciliation"><Exact wrap>{disposition.disposition_digest} · {disposition.reconciles_disposition_digest ?? "none"}</Exact></Field>
        <Field label="Provider / model"><Exact wrap>{disposition.provider_id} · {disposition.model_id}</Exact></Field>
        <Field label="Availability / admission"><Exact wrap>{disposition.availability_state} · {disposition.admission_disposition}</Exact></Field>
        <Field label="Observed"><time dateTime={disposition.observed_at}>{disposition.observed_at}</time></Field>
        <Field label="Evidence received"><time dateTime={disposition.evidence_received_at}>{disposition.evidence_received_at}</time></Field>
        <Field label="Disposition recorded"><time dateTime={disposition.disposition_received_at}>{disposition.disposition_received_at}</time></Field>
        <Field label="Expires / currentness"><Exact wrap>{disposition.expires_at} · {disposition.currentness}</Exact></Field>
        <Field label="Projection evaluated"><time dateTime={projection.evaluated_at}>{projection.evaluated_at}</time></Field>
        <Field label="Source"><Exact wrap>{disposition.source_identity} · {disposition.source_version}</Exact></Field>
        <Field label="Response created / acquisition complete"><Exact wrap>{String(disposition.response_created)} · {String(disposition.acquisition_complete)}</Exact></Field>
        <Field label="Provider retry-after"><Exact>{disposition.provider_retry_after ?? "none"}</Exact></Field>
        <Field label="Provider request occurrence"><Exact wrap>{disposition.provider_request_occurrence_id}</Exact></Field>
        <Field label="Approval response / protected effect"><Exact wrap>{String(disposition.approval_response_sent)} · {String(disposition.protected_effect_absent)}</Exact></Field>
        <Field label="Mapper snapshot"><Exact wrap>{disposition.mapper_snapshot_schema} · {disposition.mapper_snapshot_digest}</Exact></Field>
        <Field label="Observation custody"><Exact wrap>{disposition.observation_digest} · {disposition.observation_exact_bytes_sha256}</Exact></Field>
        <Field label="Disposition exact bytes"><Exact wrap>{disposition.disposition_exact_bytes_sha256}</Exact></Field>
        <Field label="Journal custody"><Exact wrap>{disposition.journal_event_id} · {disposition.journal_retained_raw_digest} · {disposition.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={disposition.journal_sequence} /></Field>
      </DefinitionGrid>{disposition.provider_execution ? <><h4>Exact provider execution identity</h4><ExecutionIdentity identity={disposition.provider_execution} /></> : <p className="empty">No provider execution identity was established by this disposition.</p>}</article>)}</div></Section>
      <Section title="Deferred wake, backoff, and fallback"><div className="record-stack">{projection.deferrals.map((deferral) => <article className="custody-row" key={deferral.deferred_dispatch_digest}><h3>{deferral.deferred_dispatch_digest}</h3><DefinitionGrid>
        <Field label="Work item / attempt"><Exact wrap>{deferral.work_item_id} · {deferral.work_attempt_id}</Exact></Field><Field label="Last dispatch"><Exact wrap>{deferral.last_dispatch_occurrence_id}</Exact></Field><Field label="Disposition edge"><Exact wrap>{deferral.disposition_digest}</Exact></Field><Field label="Provider / model"><Exact wrap>{deferral.provider_id} · {deferral.model_id}</Exact></Field><Field label="Selected / remaining ordinals"><Exact wrap>{deferral.selected_model_ordinal} · {deferral.remaining_model_ordinals.join(", ") || "none"}</Exact></Field><Field label="Refusal received"><time dateTime={deferral.refusal_received_at}>{deferral.refusal_received_at}</time></Field><Field label="Wake basis / ordinal / seconds"><Exact wrap>{deferral.wake_basis} · {deferral.backoff_ordinal} · {deferral.backoff_seconds}</Exact></Field><Field label="Provider retry-after / wake-at"><Exact wrap>{deferral.provider_retry_after ?? "none"} · {deferral.wake_at}</Exact></Field><Field label="Lock / capacity policy"><Exact wrap>{deferral.parked_resource_lock_policy} · {String(deferral.provider_capacity_released)}</Exact></Field><Field label="Exact deferral bytes"><Exact wrap>{deferral.deferred_exact_bytes_sha256}</Exact></Field><Field label="Journal custody"><Exact wrap>{deferral.journal_event_id} · {deferral.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={deferral.journal_sequence} /></Field>
      </DefinitionGrid></article>)}</div>
      <div className="record-stack">{projection.wakes.map((wake) => <article className="custody-row" key={wake.wake_occurrence_id}><h3>{wake.wake_occurrence_id}</h3><DefinitionGrid><Field label="Work item / attempt"><Exact wrap>{wake.work_item_id} · {wake.work_attempt_id}</Exact></Field><Field label="Deferred / next dispatch edges"><Exact wrap>{wake.deferred_dispatch_digest} · {wake.next_dispatch_digest}</Exact></Field><Field label="Recorded at"><time dateTime={wake.recorded_at}>{wake.recorded_at}</time></Field><Field label="Journal custody"><Exact wrap>{wake.journal_event_id} · {wake.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={wake.journal_sequence} /></Field></DefinitionGrid></article>)}</div></Section>
      <Section title="Resource release and reacquisition"><div className="record-stack">{projection.resource_transitions.map((resource) => <article className="custody-row" key={`${resource.journal_sequence}:${resource.transition}`}><h3><Exact>{resource.transition}</Exact></h3><DefinitionGrid><Field label="Work item / attempt"><Exact wrap>{resource.work_item_id} · {resource.work_attempt_id}</Exact></Field><Field label="Dispatch edge"><Exact wrap>{resource.dispatch_digest}</Exact></Field><Field label="Disposition predecessor"><Exact wrap>{resource.disposition_digest ?? "none"}</Exact></Field><Field label="Deferred predecessor"><Exact wrap>{resource.deferred_dispatch_digest ?? "none"}</Exact></Field><Field label="Policy / wake"><Exact wrap>{resource.policy_digest} · {resource.wake_occurrence_id ?? "none"}</Exact></Field><Field label="Resource locks"><StringList values={resource.resource_lock_keys} /></Field><Field label="Recorded at"><time dateTime={resource.recorded_at}>{resource.recorded_at}</time></Field><Field label="Journal custody"><Exact wrap>{resource.journal_event_id} · {resource.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={resource.journal_sequence} /></Field></DefinitionGrid></article>)}</div></Section>
      <Section title="Same-execution resumes"><div className="record-stack">{projection.resumes.map((resume) => <article className="custody-row" key={resume.resume_occurrence_id}><h3>{resume.resume_occurrence_id}</h3><DefinitionGrid><Field label="Work item / attempt"><Exact wrap>{resume.work_item_id} · {resume.work_attempt_id}</Exact></Field><Field label="Disposition edge"><Exact wrap>{resume.disposition_digest}</Exact></Field><Field label="Fresh adapter process"><Exact wrap>{resume.adapter_process_occurrence_id}</Exact></Field><Field label="Recorded at"><time dateTime={resume.recorded_at}>{resume.recorded_at}</time></Field><Field label="Journal custody"><Exact wrap>{resume.journal_event_id} · {resume.journal_exact_bytes_sha256}</Exact> · <EventRawLink navigationId={navigationId} sequence={resume.journal_sequence} /></Field></DefinitionGrid><ExecutionIdentity identity={resume.execution_identity} /></article>)}</div></Section>
    </>}
    <Section title="Independent FUEL capacity evidence">
      <DefinitionGrid><Field label="Provider-execution reference"><Exact>{projection.independent_provider_capacity_status}</Exact></Field><Field label="FUEL projection status"><Exact>{run.provider_capacity.status}</Exact></Field><Field label="Status comparison"><Exact>{projection.independent_provider_capacity_status === run.provider_capacity.status ? "same exact string" : "different exact strings"}</Exact></Field><Field label="Capacity requirement digest"><Exact wrap>{run.provider_capacity.requirement?.capacity_requirement_digest ?? "not recorded"}</Exact></Field><Field label="Capacity requirement exact bytes"><Exact wrap>{run.provider_capacity.requirement?.exact_bytes_sha256 ?? "not recorded"}</Exact></Field></DefinitionGrid>
      <div className="record-stack">{run.provider_capacity.attempts.map((attempt) => <DefinitionGrid key={attempt.attempt_id}><Field label="Attempt"><Exact wrap>{attempt.work_item_id} · {attempt.attempt_id}</Exact></Field><Field label="Capacity state / disposition"><Exact wrap>{attempt.capacity_state} · {attempt.admission_disposition}</Exact></Field><Field label="Owner digests"><Exact wrap>{attempt.capacity_admission_digest} · {attempt.observation_digest} · {attempt.policy_digest} · {attempt.decision_digest}</Exact></Field><Field label="Exact source bytes"><Exact wrap>{attempt.admission_exact_bytes_sha256} · {attempt.observation_exact_bytes_sha256} · {attempt.policy_exact_bytes_sha256} · {attempt.decision_exact_bytes_sha256}</Exact></Field></DefinitionGrid>)}</div>
      <p>{run.provider_capacity.explanation}</p>
    </Section>
    <Section title="Exact raw evidence routes"><p><Link href={rawBase}>Open registered live raw sources and complete event-byte index</Link></p><DefinitionGrid><Field label="Projection boundary"><Exact>{projection.authority_effect}</Exact></Field><Field label="Projection digest"><Exact wrap>{projection.projection_digest}</Exact></Field></DefinitionGrid></Section>
  </main>;
}

export function LiveWorkItemView({ navigationId, id }: { navigationId: string; id: string }) {
  const state = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const run = state.data;
  const item = run?.work_items.find((candidate) => candidate.work_item_id === id);
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  if (!item) return <ScreenState loading={false} error="Work item is absent from this exact live run." />;
  return <main id="main" tabIndex={-1} className="page wide"><LiveHeader run={run} />
    <header className="record-heading"><div><p className="eyebrow">Live work item</p><h1>{item.campaign_codename}</h1><Exact>{item.work_item_id}</Exact></div></header>
    <div className="record-stack">
      <Section title="1 · Bounded packet intent"><DefinitionGrid><Field label="Track"><Exact>{item.track}</Exact></Field><Field label="Dependencies"><StringList values={item.dependencies} /></Field><Field label="Entry predicates"><StringList values={item.entry_predicates} /></Field><Field label="Stop conditions"><StringList values={item.stop_conditions} /></Field></DefinitionGrid></Section>
      <Section title="2 · Live mechanism and attempt"><DefinitionGrid><Field label="Scheduler state"><Exact>{item.scheduler_state}</Exact></Field><Field label="Active attempt"><Exact wrap>{item.active_attempt_id ?? "none"}</Exact></Field><Field label="Adapter"><Exact wrap>{item.adapter_id} · {item.adapter_version}</Exact></Field><Field label="Provider/model class"><Exact>{item.provider_model_class}</Exact></Field><Field label="Provider identity"><Exact wrap>{item.provider_identity ?? "not observed"}</Exact></Field><Field label="Session / thread / turn / queue"><Exact wrap>{[item.session_identity, item.thread_identity, item.turn_identity, item.queue_identity].map((value) => value ?? "null").join(" · ")}</Exact></Field><Field label="Last event"><Exact wrap>{item.last_event_sequence === null ? "none" : `${item.last_event_sequence} · ${item.last_event_digest}`}</Exact></Field></DefinitionGrid></Section>
      <Section title="3 · Accepted terminal / not-started receipt or explicit absence">{item.accepted_outcome ? <DefinitionGrid><Field label="Receipt kind"><Exact>{item.accepted_receipt_kind ?? "unknown"}</Exact></Field><Field label="Exact state"><Exact wrap>{item.accepted_outcome.state}</Exact></Field><Field label="Exact classification"><Exact wrap>{item.accepted_outcome.result_classification}</Exact></Field><Field label="Receipt digest"><Exact wrap>{item.accepted_outcome.receipt_digest}</Exact></Field></DefinitionGrid> : <p className="empty"><Exact>{item.accepted_outcome_absent_reason ?? "ABSENT"}</Exact></p>}</Section>
      <Section title={"Lane-local questions · " + item.human_questions.length}>{item.human_questions.map((question) => <p key={question.navigation_id}><Link href={liveQuestionPath(run.navigation_id, question.navigation_id)}>{question.question}</Link></p>)}{item.human_questions.length === 0 && <p className="empty">None</p>}</Section>
    </div>
  </main>;
}

function QuestionRecord({ question }: { question: LiveQuestion }) {
  return <DefinitionGrid><Field label="Question"><Exact wrap>{question.question}</Exact></Field><Field label="Evidence exhausted"><Exact wrap>{question.exhausted_evidence}</Exact></Field><Field label="Safe default"><Exact wrap>{question.safe_default}</Exact></Field><Field label="Consequences"><Exact wrap>{question.consequences}</Exact></Field><Field label="Resume point"><Exact wrap>{question.resume_point}</Exact></Field></DefinitionGrid>;
}

export function LiveQuestionView({ navigationId, id }: { navigationId: string; id: string }) {
  const state = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const run = state.data;
  const question = run?.work_items.flatMap((item) => item.human_questions).find((candidate) => candidate.navigation_id === id);
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  if (!question) return <ScreenState loading={false} error="Question is absent from this exact lane." />;
  return <main id="main" tabIndex={-1} className="page"><LiveHeader run={run} /><Section title={"Lane-local question · " + question.question_id}><QuestionRecord question={question} /><p>This read-only surface records no answer or disposition.</p></Section></main>;
}

export function LiveEventsView({ navigationId }: { navigationId: string }) {
  const state = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const run = state.data;
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  return <main id="main" tabIndex={-1} className="page wide"><LiveHeader run={run} /><Section title="Exact append-only event timeline"><div className="record-stack">{run.events.map((event) => <DefinitionGrid key={event.sequence}><Field label="Sequence">{event.sequence}</Field><Field label="Event identity"><Exact wrap>{event.event_id}</Exact></Field><Field label="Kind"><Exact>{event.kind}</Exact></Field><Field label="Work item / attempt"><Exact wrap>{event.work_item_id ?? "run"} · {event.attempt_id ?? "none"}</Exact></Field><Field label="Recorded at"><Exact>{event.recorded_at}</Exact></Field><Field label="Retained raw digest"><Exact wrap>{event.retained_raw_digest}</Exact></Field><Field label="Exact byte SHA-256"><Exact wrap>{event.exact_bytes_sha256}</Exact></Field></DefinitionGrid>)}</div></Section></main>;
}

export function LiveRawView({ navigationId }: { navigationId: string }) {
  const state = useRemote(() => getLiveRun(navigationId), [navigationId]);
  const packet = useRemote(() => getLiveRaw(navigationId, "packet"), [navigationId]);
  const admission = useRemote(() => getLiveRaw(navigationId, "admission"), [navigationId]);
  const profile = useRemote(() => getLiveRaw(navigationId, "profile"), [navigationId]);
  const journal = useRemote(() => getLiveRaw(navigationId, "foreman-journal"), [navigationId]);
  const receipts = useRemote(() => getLiveRaw(navigationId, "accepted-receipts"), [navigationId]);
  const finalSnapshot = useRemote(
    () => state.data?.raw_sources.final_snapshot_sha256
      ? getLiveRaw(navigationId, "final")
      : Promise.resolve("ABSENT — no exact final snapshot is retained."),
    [navigationId, state.data?.raw_sources.final_snapshot_sha256],
  );
  const run = state.data;
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  const rows: Array<[string, string | undefined, boolean, string | undefined]> = [
    ["Packet", packet.data, packet.loading, packet.error],
    ["Admission", admission.data, admission.loading, admission.error],
    ["Execution profile", profile.data, profile.loading, profile.error],
    ["Foreman journal framing (hex)", journal.data, journal.loading, journal.error],
    ["Accepted receipt framing (hex)", receipts.data, receipts.loading, receipts.error],
    ["Exact final snapshot", finalSnapshot.data, finalSnapshot.loading, finalSnapshot.error],
  ];
  return <main id="main" tabIndex={-1} className="page wide"><LiveHeader run={run} /><header className="record-heading"><div><p className="eyebrow">Exact source custody</p><h1>Live raw sources</h1><p>The journal and accepted-receipt framings use fixed magic, big-endian lengths, and exact retained bytes.</p></div></header><Section title="Exact event bytes">{run.events.length ? <ul>{run.events.map((event) => <li key={event.sequence}><a target="_blank" rel="noreferrer" href={`/api/v1/active-runs/${encodeURIComponent(run.navigation_id)}/events/${event.sequence}/raw`}>Event {event.sequence} · {event.event_id}</a></li>)}</ul> : <p className="empty">None</p>}</Section><div className="raw-grid">{rows.map(([title, data, loading, error]) => <Section title={title} key={title}>{data === undefined ? <ScreenState loading={loading} error={error} /> : <pre tabIndex={0}>{data}</pre>}</Section>)}</div></main>;
}
