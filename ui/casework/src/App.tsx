import { useEffect, useMemo, useState } from "react";
import { getLiveRunIndex, getRaw, getRun, getRunIndex } from "./api";
import { DefinitionGrid, Exact, Field, Section, StringList } from "./components";
import { CompatibleExact, CompatibleList, recognized, timestampText, UnrecognizedValue, UNRECOGNIZED_RECEIPT_VALUE } from "./compatible";
import { parseRoute, questionPath, runPath, workItemPath, type Route } from "./router";
import type { CaseworkRun, HumanQuestion, RunIndex, WorkItem } from "./contract";
import {
  ActiveRunIndex,
  LiveEventsView,
  LiveQuestionView,
  LiveRawView,
  LiveRunView,
  LiveWorkItemView,
} from "./LiveViews";

function Link({ href, children, className }: { href: string; children: React.ReactNode; className?: string }) {
  return <a href={href} className={className}>{children}</a>;
}

const ALL_STATES_OPTION = "all-states";
const STATE_OPTION_PREFIX = "state:";

function stateOptionValue(index: number): string {
  return STATE_OPTION_PREFIX + index;
}

function stateFromOptionValue(value: string, states: string[]): string | null {
  if (value === ALL_STATES_OPTION) return null;
  if (!value.startsWith(STATE_OPTION_PREFIX)) return null;
  const indexText = value.slice(STATE_OPTION_PREFIX.length);
  if (!/^(0|[1-9]\d*)$/.test(indexText)) return null;
  return states[Number(indexText)] ?? null;
}

function selectedStateOptionValue(state: string | null, states: string[]): string {
  if (state === null) return ALL_STATES_OPTION;
  const index = states.indexOf(state);
  return index === -1 ? ALL_STATES_OPTION : stateOptionValue(index);
}

function stateOptionLabel(state: string): string {
  if (state === "") return '"" (empty string)';
  if (/\s/.test(state)) return JSON.stringify(state) + " (whitespace preserved)";
  return state;
}

function useRoute(): Route {
  const [route, setRoute] = useState(() => parseRoute(window.location.pathname));
  useEffect(() => {
    const onPop = () => setRoute(parseRoute(window.location.pathname));
    const onClick = (event: MouseEvent) => {
      if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      const anchor = (event.target as Element).closest("a");
      if (!anchor || anchor.origin !== window.location.origin || anchor.target) return;
      if (anchor.hash && anchor.pathname === window.location.pathname && anchor.search === window.location.search) return;
      event.preventDefault();
      window.history.pushState(null, "", anchor.href);
      onPop();
      window.scrollTo({ top: 0 });
    };
    window.addEventListener("popstate", onPop);
    document.addEventListener("click", onClick);
    return () => {
      window.removeEventListener("popstate", onPop);
      document.removeEventListener("click", onClick);
    };
  }, []);
  return route;
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
    // The caller supplies a stable dependency list for the requested resource.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dependencies);
  return state;
}

function ScreenState({ loading, error }: { loading: boolean; error?: string }) {
  if (loading) return <p role="status" className="screen-state">Loading exact casework projection…</p>;
  return <p role="alert" className="screen-state error">{error ?? "The requested case record was not found."}</p>;
}

function StateCounts({ counts }: { counts: Record<string, number> }) {
  return (
    <dl className="state-counts" aria-label="Counts by exact state">
      {Object.entries(counts).map(([state, count]) => (
        <div key={state}><dt><Exact wrap>{state}</Exact></dt><dd>{count}</dd></div>
      ))}
    </dl>
  );
}

function RunIndexView() {
  const state = useRemote(getRunIndex, []);
  if (!state.data) return <ScreenState loading={state.loading} error={state.error} />;
  return (
    <main id="main" tabIndex={-1} className="page">
      <header className="page-heading">
        <p className="eyebrow">Read-only evidence projection</p>
        <h1>Run case index</h1>
        <p>Sealed packet intent paired with one exact receipt snapshot. Classifications remain independent and verbatim.</p>
      </header>
      <ActiveRunIndex />
      <Section title="Sealed receipt cases">
        <p>Immutable packet intent paired with exact closeout receipt snapshots.</p>
      <div className="run-ledger">
        {state.data.runs.map((run) => (
          <article className="run-card" key={run.run_id}>
            <header>
              <div><p className="eyebrow">Packet</p><h2><Link href={runPath(run.run_id)}>{run.packet_id}</Link></h2></div>
              <time dateTime={run.receipt_updated_at.recognized_rfc3339 ?? undefined}>{timestampText(run.receipt_updated_at)}</time>
            </header>
            <DefinitionGrid>
              <Field label="Packet digest"><Exact wrap>{run.packet_digest}</Exact></Field>
              <Field label="Projection digest"><Exact wrap>{run.projection_digest}</Exact></Field>
              <Field label="Packet integrity"><Exact wrap>{run.packet_integrity}</Exact></Field>
              <Field label="Current at snapshot"><Exact>{run.packet_currentness_at_receipt_snapshot}</Exact></Field>
              <Field label="Current now"><Exact>{run.packet_currentness_now}</Exact></Field>
              <Field label="Work items">{run.summary.work_item_count}</Field>
              <Field label="Human questions">{run.summary.human_question_count}</Field>
              <Field label="Packet custody discrepancies">{run.summary.packet_custody_discrepancy_count}</Field><Field label="Unrecognized state values">{run.summary.unrecognized_state_count}</Field>
            </DefinitionGrid>
            <StateCounts counts={run.summary.state_counts} />
            <p><Link className="text-link" href={runPath(run.run_id)}>Open run case <span aria-hidden="true">→</span></Link></p>
          </article>
        ))}
        {state.data.runs.length === 0 && <p className="empty">No explicit run directories are loaded.</p>}
      </div>
      </Section>
    </main>
  );
}

function RunNav({ run }: { run: CaseworkRun }) {
  const base = runPath(run.run_id);
  return (
    <nav aria-label="Run case sections" className="run-nav">
      <Link href={base}>Work items</Link>
      <a href="#human-questions">Human questions</a>
      <Link href={`${base}/custody`}>Custody</Link>
      <Link href={`${base}/raw`}>Raw artifacts</Link>
    </nav>
  );
}

function RunHeader({ run }: { run: CaseworkRun }) {
  const liveState = useRemote(getLiveRunIndex, []);
  const live = liveState.data?.runs.find((entry) => entry.sealed_case_run_id === run.run_id);
  return (
    <>
      <div className="breadcrumbs"><Link href="/">Run index</Link><span aria-hidden="true">/</span><Exact>{run.packet.packet_id}</Exact></div>
      <header className="page-heading compact">
        <p className="eyebrow">Run case · receipt snapshot <time dateTime={run.receipts.updated_at.recognized_rfc3339 ?? undefined}>{timestampText(run.receipts.updated_at)}</time></p>
        <h1>{run.packet.packet_id}</h1>
        <p className="digest"><Exact wrap>{run.packet.packet_digest}</Exact></p>
      </header>
      {live && <p><Link href={`/active-runs/${encodeURIComponent(live.navigation_id)}`}>Open byte-matched live foreman history</Link></p>}
      <RunNav run={run} />
    </>
  );
}

function RunCaseView({ digest }: { digest: string }) {
  const state = useRemote(() => getRun(digest), [digest]);
  const [stateFilter, setStateFilter] = useState<string | null>(null);
  const [trackFilter, setTrackFilter] = useState("");
  const [questionFilter, setQuestionFilter] = useState("all");
  const run = state.data;
  const questionItems = useMemo(() => new Set(run?.human_questions.flatMap((question) => question.linked_work_item ? [question.linked_work_item] : []) ?? []), [run]);
  const states = useMemo(() => [...new Set(run?.work_items.map((item) => recognized(item.outcome.state)).filter((value): value is string => value !== null) ?? [])].sort(), [run]);
  const tracks = useMemo(() => [...new Set(run?.work_items.map((item) => item.track) ?? [])].sort(), [run]);
  const items = useMemo(() => {
    if (!run) return undefined;
    return run.work_items.filter((item) => {
      const questionMatches = questionFilter === "all"
        || (questionFilter === "with" ? questionItems.has(item.id) : !questionItems.has(item.id));
      return (stateFilter === null || recognized(item.outcome.state) === stateFilter)
        && (!trackFilter || item.track === trackFilter)
        && questionMatches;
    });
  }, [run, stateFilter, trackFilter, questionFilter, questionItems]);
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  return (
    <main id="main" tabIndex={-1} className="page wide">
      <RunHeader run={run} />
      <section className="summary-strip" aria-label="Run facts">
        <div><span>Work items</span><strong>{run.summary.work_item_count}</strong></div>
        <div><span>Human questions</span><strong>{run.summary.human_question_count}</strong></div>
        <div><span>Packet custody discrepancies</span><strong>{run.summary.packet_custody_discrepancy_count}</strong></div><div><span>Unrecognized state values</span><strong>{run.summary.unrecognized_state_count}</strong></div>
        <div><span>Current at snapshot</span><Exact>{run.packet.currentness_at_receipt_snapshot}</Exact></div>
        <div><span>Current now</span><Exact>{run.packet.currentness_now}</Exact></div>
      </section>
      <Section title="Work-item ledger">
        <form className="filters" aria-label="Work-item filters" onSubmit={(event) => event.preventDefault()}>
          <label>Exact state<select value={selectedStateOptionValue(stateFilter, states)} onChange={(event) => setStateFilter(stateFromOptionValue(event.target.value, states))}><option value={ALL_STATES_OPTION}>All exact states</option>{states.map((value, index) => <option key={stateOptionValue(index)} value={stateOptionValue(index)}>{stateOptionLabel(value)}</option>)}</select></label>
          <label>Track<select value={trackFilter} onChange={(event) => setTrackFilter(event.target.value)}><option value="">All tracks</option>{tracks.map((value) => <option key={value}>{value}</option>)}</select></label>
          <label>Human question<select value={questionFilter} onChange={(event) => setQuestionFilter(event.target.value)}><option value="all">All work items</option><option value="with">Has human question</option><option value="without">No human question</option></select></label>
        </form>
        <p className="result-count" aria-live="polite">Showing {items?.length ?? 0} of {run.work_items.length} exact work items</p>
        <div className="table-scroll"><table className="ledger-table"><thead><tr><th scope="col">Campaign</th><th scope="col">Track</th><th scope="col">Exact state</th><th scope="col">Exact classification</th><th scope="col">Dependencies</th><th scope="col">Question</th></tr></thead>
          <tbody>{items?.map((item) => <tr key={item.derived_id}><th scope="row"><Link href={workItemPath(run.run_id, item.id)}>{item.campaign.codename}</Link><small>{item.campaign.canonical_slug}</small></th><td><Exact wrap>{item.track}</Exact></td><td><CompatibleExact value={item.outcome.state} runId={run.run_id} /></td><td><CompatibleExact value={item.outcome.result_classification} runId={run.run_id} /></td><td>{item.dependencies.length ? item.dependencies.map((value) => <Exact key={value} wrap>{value}</Exact>) : <span className="empty">None</span>}</td><td>{questionItems.has(item.id) ? <span className="label">Present</span> : <span className="empty">None</span>}</td></tr>)}</tbody>
        </table></div>
      </Section>
      <QuestionList run={run} />
    </main>
  );
}

function QuestionList({ run }: { run: CaseworkRun }) {
  return <Section title={"Human questions · " + run.human_questions.length} className="questions" id="human-questions" tabIndex={-1}>
    <p>These are exact receipt fields. This surface records no disposition.</p>
    <ol>{run.human_questions.map((question) => <li key={question.navigation_id}><article><h3><Link href={questionPath(run.run_id, question.navigation_id)}>{recognized(question.exact_question) ?? UNRECOGNIZED_RECEIPT_VALUE}</Link></h3>{recognized(question.exact_question) === null && <p><UnrecognizedValue runId={run.run_id} /></p>}<p><span className="field-label">Linked work item</span> {question.linked_work_item ? <Link href={workItemPath(run.run_id, question.linked_work_item)}><Exact>{question.linked_work_item}</Exact></Link> : <CompatibleExact value={question.work_item} runId={run.run_id} />}</p></article></li>)}</ol>
  </Section>;
}

const INTENT_LISTS: Array<[keyof WorkItem, string]> = [
  ["entry_predicates", "Entry predicates"], ["allowed_mutation_surfaces", "Allowed mutation surfaces"],
  ["forbidden_actions", "Forbidden actions"], ["acceptance_tests", "Acceptance tests"],
  ["stop_conditions", "Stop conditions"], ["expected_receipts", "Expected receipts"],
  ["closeout_requirements", "Closeout requirements"],
];

function WorkItemView({ digest, id }: { digest: string; id: string }) {
  const state = useRemote(() => getRun(digest), [digest]);
  const run = state.data;
  const item = run?.work_items.find((entry) => entry.id === id);
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  if (!item) return <ScreenState loading={false} error={`Work item ${id} is not present in this exact run.`} />;
  return <main id="main" tabIndex={-1} className="page wide"><RunHeader run={run} />
    <header className="record-heading"><div><p className="eyebrow">Work item · <Exact>{item.id}</Exact></p><h1>{item.campaign.codename}</h1><p>{item.campaign.canonical_slug}</p></div><div className="classification-block"><span>Exact state</span><CompatibleExact value={item.outcome.state} runId={run.run_id} /><span>Exact classification</span><CompatibleExact value={item.outcome.result_classification} runId={run.run_id} /></div></header>
    <div className="paired-columns">
      <section className="case-column intent"><header><p className="eyebrow">Sealed packet</p><h2>Bounded intent</h2></header>
        <DefinitionGrid><Field label="Track"><Exact>{item.track}</Exact></Field><Field label="Dependencies"><StringList values={item.dependencies} /></Field></DefinitionGrid>
        <h3>Predecessor lineage</h3>{item.predecessor_lineage.length ? <div className="record-stack">{item.predecessor_lineage.map((row, index) => <DefinitionGrid key={`${row.commit}-${index}`}><Field label="Campaign"><Exact>{row.campaign}</Exact></Field><Field label="Classification"><Exact wrap>{row.classification}</Exact></Field><Field label="Commit"><Exact wrap>{row.commit}</Exact></Field></DefinitionGrid>)}</div> : <p className="empty">None recorded</p>}
        <h3>Exact-work references</h3>{item.exact_work_refs.length ? <div className="record-stack">{item.exact_work_refs.map((row, index) => <DefinitionGrid key={`${row.proposal_ref}-${index}`}><Field label="Contract"><Exact wrap>{row.contract_kind} · {row.contract_schema}</Exact></Field><Field label="Repository"><Exact wrap>{row.repository} · {row.branch} · {row.commit}</Exact></Field><Field label="Path"><Exact wrap>{row.path}</Exact></Field><Field label="Proposal reference"><Exact wrap>{row.proposal_ref}</Exact></Field></DefinitionGrid>)}</div> : <p className="empty">None recorded</p>}
        {INTENT_LISTS.map(([key, title]) => <div className="list-block" key={key}><h3>{title}</h3><StringList values={item[key] as string[]} /></div>)}
        <h3>Model routing</h3><DefinitionGrid><Field label="Class"><Exact>{item.model_routing.class}</Exact></Field><Field label="Maximum mutating workers">{item.model_routing.maximum_mutating_workers}</Field><Field label="Reason"><Exact wrap>{item.model_routing.reason}</Exact></Field></DefinitionGrid>
      </section>
      <section className="case-column outcome"><header><p className="eyebrow">Receipt snapshot</p><h2>Recorded outcome</h2></header>
        <DefinitionGrid><Field label="Exact state"><CompatibleExact value={item.outcome.state} runId={run.run_id} /></Field><Field label="Exact classification"><CompatibleExact value={item.outcome.result_classification} runId={run.run_id} /></Field></DefinitionGrid>
        <h3>Resulting repositories and custody</h3>{item.outcome.repositories.recognized_rows ? <div className="record-stack">{item.outcome.repositories.recognized_rows.map((row, index) => <DefinitionGrid key={index}><Field label="Repository"><Exact>{row.repository}</Exact></Field><Field label="Branch"><Exact wrap>{row.branch}</Exact></Field><Field label="Head"><Exact wrap>{row.head}</Exact></Field><Field label="Push status"><Exact wrap>{row.push_status}</Exact></Field></DefinitionGrid>)}</div> : <p className="unrecognized">Receipt value does not have the recognized repository-row shape. <a href={runPath(run.run_id) + "/raw"}>inspect raw receipts</a></p>}
        <div className="list-block"><h3>Tests</h3><CompatibleList value={item.outcome.tests} runId={run.run_id} /></div><div className="list-block"><h3>Evidence</h3><CompatibleList value={item.outcome.evidence} runId={run.run_id} /></div><div className="list-block"><h3>Live or production mutations</h3><CompatibleList value={item.outcome.live_or_production_mutations} runId={run.run_id} /></div>
        <h3>Remaining trigger</h3><p><CompatibleExact value={item.outcome.remaining_trigger} runId={run.run_id} /></p><h3>Next lawful action</h3><p><CompatibleExact value={item.outcome.next_lawful_action} runId={run.run_id} /></p>
      </section>
    </div>
  </main>;
}

function QuestionRecord({ run, question }: { run: CaseworkRun; question: HumanQuestion }) {
  return <article className="question-record"><header><p className="eyebrow">Exact receipt question</p><h1>{recognized(question.exact_question) ?? <UnrecognizedValue runId={run.run_id} />}</h1></header><DefinitionGrid>
    <Field label="Question identifier">{question.derived_id ? <Exact wrap>{question.derived_id}</Exact> : <span className="unrecognized">Not derived from an unrecognized question value</span>}</Field><Field label="Navigation identifier"><Exact wrap>{question.navigation_id}</Exact></Field><Field label="Source ordinal">{question.source_ordinal}</Field>
    <Field label="Linked work item">{question.linked_work_item ? <Link href={workItemPath(run.run_id, question.linked_work_item)}><Exact>{question.linked_work_item}</Exact></Link> : <CompatibleExact value={question.work_item} runId={run.run_id} />}</Field>
    <Field label="Evidence exhausted"><CompatibleExact value={question.evidence_exhausted} runId={run.run_id} /></Field>
    <Field label="Safe default"><CompatibleExact value={question.safe_default} runId={run.run_id} /></Field>
    <Field label="Consequences"><CompatibleExact value={question.consequences} runId={run.run_id} /></Field>
    <Field label="Resume point"><CompatibleExact value={question.resume_point} runId={run.run_id} /></Field>
  </DefinitionGrid></article>;
}

function QuestionView({ digest, id }: { digest: string; id: string }) {
  const state = useRemote(() => getRun(digest), [digest]); const run = state.data; const question = run?.human_questions.find((entry) => entry.navigation_id === id);
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  if (!question) return <ScreenState loading={false} error="Question identifier is not present in this exact run." />;
  return <main id="main" tabIndex={-1} className="page"><RunHeader run={run} /><QuestionRecord run={run} question={question} /></main>;
}

function CustodyView({ digest }: { digest: string }) {
  const state = useRemote(() => getRun(digest), [digest]); const run = state.data;
  if (!run) return <ScreenState loading={state.loading} error={state.error} />;
  return <main id="main" tabIndex={-1} className="page wide"><RunHeader run={run} /><header className="record-heading"><div><p className="eyebrow">Exact source sections</p><h1>Repository custody</h1><p>Starting packet custody and final receipt custody remain separate. Text is displayed without inferred disposition.</p></div></header>
    <div className="paired-columns custody-columns"><Section title="Starting packet custody"><div className="record-stack">{run.packet.repository_custody.map((row) => <article className="custody-row" key={row.derived_id}><h3>{row.repository}</h3><DefinitionGrid><Field label="Path"><Exact wrap>{row.path}</Exact></Field><Field label="Branch"><Exact wrap>{row.branch}</Exact></Field><Field label="Commit"><Exact wrap>{row.commit}</Exact></Field><Field label="Remote"><Exact wrap>{row.remote ?? "null"}</Exact></Field><Field label="Remote commit"><Exact wrap>{row.remote_commit ?? "null"}</Exact></Field><Field label="Worktree clean"><Exact>{String(row.worktree_clean)}</Exact></Field><Field label="Discrepancy"><Exact wrap>{row.discrepancy ?? "null"}</Exact></Field></DefinitionGrid></article>)}</div></Section>
      <Section title="Final receipt custody"><div className="record-stack">{run.final_repository_custody.map((row) => <article className="custody-row" key={row.navigation_id}><h3><CompatibleExact value={row.repository} runId={run.run_id} /></h3><DefinitionGrid><Field label="Branch head"><CompatibleExact value={row.branch_head} runId={run.run_id} /></Field><Field label="Push custody"><CompatibleExact value={row.push_custody} runId={run.run_id} /></Field><Field label="Dirty"><CompatibleExact value={row.dirty} runId={run.run_id} /></Field><Field label="Live runtime"><CompatibleExact value={row.live_runtime} runId={run.run_id} /></Field><Field label="Secrets"><CompatibleExact value={row.secrets} runId={run.run_id} /></Field><Field label="Teardown"><CompatibleExact value={row.teardown} runId={run.run_id} /></Field></DefinitionGrid></article>)}</div></Section>
    </div></main>;
}

function RawView({ digest }: { digest: string }) {
  const runState = useRemote(() => getRun(digest), [digest]); const packetState = useRemote(() => getRaw(digest, "packet"), [digest]); const receiptsState = useRemote(() => getRaw(digest, "receipts"), [digest]); const run = runState.data;
  if (!run) return <ScreenState loading={runState.loading} error={runState.error} />;
  return <main id="main" tabIndex={-1} className="page wide"><RunHeader run={run} /><header className="record-heading"><div><p className="eyebrow">Exact source-byte inspection</p><h1>Raw artifacts</h1><p>Read-only source bytes from the explicitly loaded run directory.</p></div></header>
    <div className="raw-grid"><Section title="Packet bytes"><DefinitionGrid><Field label="SHA-256"><Exact wrap>{run.packet.source_bytes_digest}</Exact></Field><Field label="Validation disposition"><Exact wrap>{run.packet.integrity}</Exact></Field></DefinitionGrid>{packetState.data !== undefined ? <pre tabIndex={0} aria-label="Exact packet bytes">{packetState.data}</pre> : <ScreenState loading={packetState.loading} error={packetState.error} />}</Section>
      <Section title="Receipt bytes"><DefinitionGrid><Field label="SHA-256"><Exact wrap>{run.receipts.source_bytes_digest}</Exact></Field><Field label="Validation disposition"><Exact wrap>{run.receipts.validation}</Exact></Field></DefinitionGrid>{receiptsState.data !== undefined ? <pre tabIndex={0} aria-label="Exact receipt bytes">{receiptsState.data}</pre> : <ScreenState loading={receiptsState.loading} error={receiptsState.error} />}</Section></div>
  </main>;
}

function RoutedView({ route }: { route: Route }) {
  switch (route.kind) {
    case "index": return <RunIndexView />;
    case "run": return <RunCaseView digest={route.digest} />;
    case "work-item": return <WorkItemView digest={route.digest} id={route.id} />;
    case "question": return <QuestionView digest={route.digest} id={route.id} />;
    case "custody": return <CustodyView digest={route.digest} />;
    case "raw": return <RawView digest={route.digest} />;
    case "live-run": return <LiveRunView navigationId={route.navigationId} />;
    case "live-work-item": return <LiveWorkItemView navigationId={route.navigationId} id={route.id} />;
    case "live-question": return <LiveQuestionView navigationId={route.navigationId} id={route.id} />;
    case "live-events": return <LiveEventsView navigationId={route.navigationId} />;
    case "live-raw": return <LiveRawView navigationId={route.navigationId} />;
    default: return <main id="main" tabIndex={-1} className="page"><h1>Case route not found</h1><p><Link href="/">Return to run index</Link></p></main>;
  }
}

export default function App() {
  const route = useRoute();
  return <div className="app-shell"><a className="skip-link" href="#main">Skip to casework</a><header className="site-header"><Link href="/" className="brand"><span className="brand-mark" aria-hidden="true">N</span><span>Nightshift <b>Casework</b></span></Link><span className="readonly-marker">Read-only operator surface</span></header><RoutedView route={route} /><footer><span>Projection families: nightshift.casework-run/v1 · nightshift.casework-live-run/v1</span><span>Exact sources remain separate: sealed snapshots and query-only foreman journals</span></footer></div>;
}
