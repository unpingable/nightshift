import { useEffect, useState } from "react";

import { getLiveRaw, getLiveRun, getLiveRunIndex } from "./api";
import { DefinitionGrid, Exact, Field, Section, StringList } from "./components";
import type { CaseworkLiveRun, LiveQuestion } from "./contract";
import { liveQuestionPath, liveRunPath, liveWorkItemPath } from "./router";

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
      {run.provider_capacity.attempts.length > 0 && <div className="table-scroll"><table className="ledger-table"><thead><tr><th>Attempt</th><th>Provider / model</th><th>State / disposition</th><th>Source / confidence</th><th>Times / currentness</th><th>Exact raw digests</th></tr></thead><tbody>
        {run.provider_capacity.attempts.map((attempt) => <tr key={attempt.attempt_id}><th scope="row"><Exact wrap>{attempt.work_item_id}</Exact><small>{attempt.attempt_id} · journal {attempt.journal_sequence}</small></th><td><Exact wrap>{attempt.provider_id} · {attempt.packet_model_class} · {attempt.cost_class}</Exact></td><td><Exact wrap>{attempt.capacity_state} · {attempt.admission_disposition}</Exact></td><td><Exact wrap>{attempt.source_class} · {attempt.confidence} · {attempt.observation_disposition}</Exact></td><td><Exact wrap>{attempt.observed_at} → {attempt.expires_at} · decision {attempt.decision_at} · {attempt.currentness}</Exact></td><td><Exact wrap>{attempt.admission_exact_bytes_sha256} · {attempt.observation_exact_bytes_sha256} · {attempt.policy_exact_bytes_sha256} · {attempt.decision_exact_bytes_sha256}</Exact></td></tr>)}
      </tbody></table></div>}
    </Section>
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
