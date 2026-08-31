import { useEffect, useState } from "react";

import {
  getOperationalCondition,
  getOperationalConditionIndex,
  getOperationalRaw,
} from "./api";
import { DefinitionGrid, Exact, Field, Section, StringList } from "./components";
import type {
  CaseworkOperationalCondition,
  OperationalConditionIndex,
  OperationalRawSource,
} from "./contract";
import {
  operationalConditionPath,
  operationalQuestionPath,
  operationalRawPath,
} from "./router";

function Link({ href, children }: { href: string; children: React.ReactNode }) {
  return <a href={href}>{children}</a>;
}

function useRemote<T>(load: () => Promise<T>, dependencies: unknown[]) {
  const [state, setState] = useState<{ data?: T; error?: string; loading: boolean }>({
    loading: true,
  });
  useEffect(() => {
    let active = true;
    setState({ loading: true });
    load().then(
      (data) => active && setState({ data, loading: false }),
      (error: unknown) =>
        active &&
        setState({
          error: error instanceof Error ? error.message : String(error),
          loading: false,
        }),
    );
    return () => {
      active = false;
    };
    // The caller supplies the exact requested resource identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dependencies);
  return state;
}

function ScreenState({ loading, error }: { loading: boolean; error?: string }) {
  if (loading) return <p role="status" className="screen-state">Loading exact operational projection…</p>;
  return <p role="alert" className="screen-state error">{error ?? "Operational record not found."}</p>;
}

export function OperationalConditionIndexSection() {
  const state = useRemote<OperationalConditionIndex>(getOperationalConditionIndex, []);
  return (
    <Section title="Operational conditions">
      <p>
        Monitor testimony, NQ qualification, and Nightshift temporal evaluation remain
        distinct. This view creates no combined disposition.
      </p>
      {!state.data ? (
        <ScreenState loading={state.loading} error={state.error} />
      ) : (
        <div className="table-scroll">
          <table className="ledger-table">
            <thead>
              <tr>
                <th scope="col">Operational subject</th>
                <th scope="col">Subject kind</th>
                <th scope="col">Exact disposition</th>
                <th scope="col">Re-observation trigger</th>
                <th scope="col">Questions</th>
                <th scope="col">Evaluated</th>
              </tr>
            </thead>
            <tbody>
              {state.data.conditions.map((condition) => (
                <tr key={condition.navigation_id}>
                  <th scope="row">
                    <Link href={operationalConditionPath(condition.navigation_id)}>
                      <Exact wrap>{condition.subject_namespace}</Exact>
                    </Link>
                    <small><Exact wrap>{condition.subject_identity_digest}</Exact></small>
                  </th>
                  <td><Exact>{condition.subject_kind}</Exact></td>
                  <td><Exact>{condition.disposition}</Exact></td>
                  <td><Exact wrap>{condition.reobservation_trigger}</Exact></td>
                  <td>{condition.question_count}</td>
                  <td><time dateTime={condition.evaluated_at}>{condition.evaluated_at}</time></td>
                </tr>
              ))}
            </tbody>
          </table>
          {state.data.conditions.length === 0 && (
            <p className="empty">No explicit operational condition directories are loaded.</p>
          )}
        </div>
      )}
      <p><Link href="/operational-conditions">Open operational condition index</Link></p>
    </Section>
  );
}

export function OperationalConditionIndexView() {
  return (
    <main id="main" tabIndex={-1} className="page wide">
      <div className="breadcrumbs"><Link href="/">Run index</Link><span aria-hidden="true">/</span>Operational conditions</div>
      <header className="page-heading">
        <p className="eyebrow">Read-only operational evidence</p>
        <h1>Operational condition index</h1>
        <p>Each row is an independent typed subject and temporal evaluation.</p>
      </header>
      <OperationalConditionIndexSection />
    </main>
  );
}

function ConditionHeader({ condition }: { condition: CaseworkOperationalCondition }) {
  const base = operationalConditionPath(condition.navigation_id);
  return (
    <>
      <div className="breadcrumbs">
        <Link href="/operational-conditions">Operational conditions</Link>
        <span aria-hidden="true">/</span>
        <Exact>{condition.subject.namespace}</Exact>
      </div>
      <header className="page-heading compact">
        <p className="eyebrow">Operational subject · <Exact>{condition.subject.kind}</Exact></p>
        <h1>{condition.subject.namespace}</h1>
        <p className="digest"><Exact wrap>{condition.subject_identity_digest}</Exact></p>
      </header>
      <nav aria-label="Operational condition sections" className="run-nav">
        <Link href={base}>Condition</Link>
        <a href="#qualification-findings">NQ findings</a>
        <a href="#temporal-lineage">Temporal lineage</a>
        <Link href={operationalRawPath(condition.navigation_id)}>Raw artifacts</Link>
      </nav>
    </>
  );
}

function RawCustody({
  label,
  source,
  semantic,
}: {
  label: string;
  source: OperationalRawSource;
  semantic: string;
}) {
  return (
    <article className="custody-row">
      <h3>{label}</h3>
      <DefinitionGrid>
        <Field label="Exact bytes SHA-256"><Exact wrap>{source.exact_bytes_sha256}</Exact></Field>
        <Field label="Exact byte length">{source.exact_bytes_length}</Field>
        <Field label="Semantic digest"><Exact wrap>{semantic}</Exact></Field>
        <Field label="Validation"><Exact wrap>{source.validation}</Exact></Field>
      </DefinitionGrid>
    </article>
  );
}

function QuestionList({
  condition,
}: {
  condition: CaseworkOperationalCondition;
}) {
  return (
    <Section title={`Presentation questions · ${condition.questions.length}`}>
      <p>
        These links present exact upstream nonclaims or contradictions. They create no
        finding, answer, or disposition.
      </p>
      <ol>
        {condition.questions.map((question) => (
          <li key={question.navigation_id}>
            <Link
              href={operationalQuestionPath(
                condition.navigation_id,
                question.navigation_id,
              )}
            >
              {question.question}
            </Link>
          </li>
        ))}
      </ol>
      {condition.questions.length === 0 && <p className="empty">None recorded.</p>}
    </Section>
  );
}

export function OperationalConditionView({ navigationId }: { navigationId: string }) {
  const state = useRemote(() => getOperationalCondition(navigationId), [navigationId]);
  const condition = state.data;
  if (!condition) return <ScreenState loading={state.loading} error={state.error} />;
  const lineage = condition.lineage;
  const evaluation = condition.evaluation;
  return (
    <main id="main" tabIndex={-1} className="page wide">
      <ConditionHeader condition={condition} />
      <section className="summary-strip" aria-label="Exact operational evaluation">
        <div><span>Acquisition outcome</span><Exact>{condition.acquisition_outcome}</Exact></div>
        <div><span>Disposition</span><Exact>{evaluation.disposition}</Exact></div>
        <div><span>Re-observation trigger</span><Exact wrap>{evaluation.reobservation_trigger}</Exact></div>
        <div><span>Next lawful action</span><Exact wrap>{evaluation.next_lawful_action}</Exact></div>
        <div><span>Current until</span><Exact wrap>{evaluation.current_until ?? "null"}</Exact></div>
        <div><span>Authority effect</span><Exact wrap>{condition.authority_effect}</Exact></div>
      </section>

      <div className="paired-columns">
        <Section title="Canonical operational subject">
          <DefinitionGrid>
            <Field label="Kind"><Exact>{condition.subject.kind}</Exact></Field>
            <Field label="Namespace"><Exact wrap>{condition.subject.namespace}</Exact></Field>
            <Field label="Basis contract"><Exact wrap>{condition.subject.basis_contract}</Exact></Field>
            <Field label="Identity digest"><Exact wrap>{condition.subject_identity_digest}</Exact></Field>
          </DefinitionGrid>
          <h3>Stable basis</h3>
          <DefinitionGrid>
            {Object.entries(condition.subject.stable_basis).map(([key, value]) => (
              <Field key={key} label={key}><Exact wrap>{value}</Exact></Field>
            ))}
          </DefinitionGrid>
        </Section>
        <Section title="Monitor producer">
          <DefinitionGrid>
            <Field label="Principal"><Exact wrap>{condition.producer.principal_id}</Exact></Field>
            <Field label="Collector"><Exact wrap>{condition.producer.collector_id}</Exact></Field>
            <Field label="Producer class"><Exact wrap>{condition.producer.producer_class}</Exact></Field>
            <Field label="Key algorithm"><Exact>{condition.producer.key_algorithm}</Exact></Field>
            <Field label="Public-key digest"><Exact wrap>{condition.producer.public_key_digest}</Exact></Field>
            <Field label="Producer identity digest"><Exact wrap>{condition.producer_identity_digest}</Exact></Field>
          </DefinitionGrid>
        </Section>
      </div>

      <Section title="Exact source custody">
        <div className="record-stack">
          <RawCustody
            label="Monitor signed acquisition"
            source={condition.raw_sources.monitor}
            semantic={lineage.monitor_custody.semantic_digest}
          />
          <RawCustody
            label="NQ qualification"
            source={condition.raw_sources.nq}
            semantic={lineage.nq_custody.semantic_digest}
          />
        </div>
        <DefinitionGrid>
          <Field label="Monitor result head"><Exact wrap>{lineage.monitor_result_head}</Exact></Field>
          <Field label="NQ result head"><Exact wrap>{lineage.nq_result_head}</Exact></Field>
          <Field label="NQ profile"><Exact wrap>{lineage.nq_profile_id}</Exact></Field>
          <Field label="NQ input"><Exact wrap>{lineage.nq_input_id}</Exact></Field>
        </DefinitionGrid>
      </Section>

      <Section title="NQ qualification findings" id="qualification-findings" tabIndex={-1}>
        <h3>Exact supported claims</h3>
        <div className="record-stack">
          {lineage.claim_support.map((claim) => (
            <DefinitionGrid key={claim.claim_id}>
              <Field label="Claim"><Exact wrap>{claim.claim_id}</Exact></Field>
              <Field label="Proposition"><Exact wrap>{claim.proposition}</Exact></Field>
              <Field label="Value digest"><Exact wrap>{claim.value_digest}</Exact></Field>
              <Field label="Monitor record"><Exact wrap>{claim.monitor_record_digest}</Exact></Field>
            </DefinitionGrid>
          ))}
        </div>
        {lineage.claim_support.length === 0 && <p className="empty">No supported claims.</p>}
        <h3>Cannot testify</h3>
        <div className="record-stack">
          {lineage.cannot_testify.map((finding) => (
            <DefinitionGrid key={finding.claim_id}>
              <Field label="Claim"><Exact wrap>{finding.claim_id}</Exact></Field>
              <Field label="Reason"><Exact wrap>{finding.reason}</Exact></Field>
            </DefinitionGrid>
          ))}
        </div>
        {lineage.cannot_testify.length === 0 && <p className="empty">None recorded.</p>}
        <h3>Refusals</h3>
        <div className="record-stack">
          {lineage.refusals.map((finding, index) => (
            <DefinitionGrid key={`${finding.code}-${index}`}>
              <Field label="Code"><Exact wrap>{finding.code}</Exact></Field>
              <Field label="Exact basis digest"><Exact wrap>{finding.exact_basis_digest}</Exact></Field>
              <Field label="Detail"><Exact wrap>{finding.detail}</Exact></Field>
            </DefinitionGrid>
          ))}
        </div>
        {lineage.refusals.length === 0 && <p className="empty">None recorded.</p>}
        <h3>Contradictions</h3>
        <div className="record-stack">
          {lineage.contradictions.map((finding, index) => (
            <DefinitionGrid key={`${finding.claim_id}-${index}`}>
              <Field label="Claim"><Exact wrap>{finding.claim_id}</Exact></Field>
              <Field label="First input/value"><Exact wrap>{finding.first_input_id} · {finding.first_value_digest}</Exact></Field>
              <Field label="Second input/value"><Exact wrap>{finding.second_input_id} · {finding.second_value_digest}</Exact></Field>
            </DefinitionGrid>
          ))}
        </div>
        {lineage.contradictions.length === 0 && <p className="empty">None recorded.</p>}
        <h3>Exact nonclaims</h3>
        <StringList values={lineage.nonclaims} />
      </Section>

      <Section title="Nightshift temporal lineage" id="temporal-lineage" tabIndex={-1}>
        <DefinitionGrid>
          <Field label="Lineage ID"><Exact wrap>{lineage.lineage_id}</Exact></Field>
          <Field label="Epoch"><Exact wrap>{lineage.epoch}</Exact></Field>
          <Field label="Sequence">{lineage.sequence}</Field>
          <Field label="Predecessor observation"><Exact wrap>{lineage.predecessor_observation_digest ?? "null"}</Exact></Field>
          <Field label="Acquisition started"><time dateTime={lineage.acquisition_started_at}>{lineage.acquisition_started_at}</time></Field>
          <Field label="Acquisition ended"><time dateTime={lineage.acquisition_ended_at}>{lineage.acquisition_ended_at}</time></Field>
          <Field label="Producer observed"><Exact wrap>{lineage.producer_observed_at ?? "null"}</Exact></Field>
          <Field label="Receiver custody"><time dateTime={lineage.receiver_custody_at}>{lineage.receiver_custody_at}</time></Field>
          <Field label="NQ qualified"><time dateTime={lineage.nq_qualified_at}>{lineage.nq_qualified_at}</time></Field>
          <Field label="Nightshift admitted"><time dateTime={lineage.nightshift_admitted_at}>{lineage.nightshift_admitted_at}</time></Field>
          <Field label="Evaluated"><time dateTime={evaluation.evaluated_at}>{evaluation.evaluated_at}</time></Field>
          <Field label="Profile max age">{condition.profile.max_age_seconds} seconds</Field>
          <Field label="Current until"><Exact wrap>{evaluation.current_until ?? "null"}</Exact></Field>
        </DefinitionGrid>
      </Section>
      <QuestionList condition={condition} />
    </main>
  );
}

export function OperationalQuestionView({
  navigationId,
  id,
}: {
  navigationId: string;
  id: string;
}) {
  const state = useRemote(() => getOperationalCondition(navigationId), [navigationId]);
  const condition = state.data;
  if (!condition) return <ScreenState loading={state.loading} error={state.error} />;
  const question = condition.questions.find((item) => item.navigation_id === id);
  if (!question) return <ScreenState loading={false} error="Question is not present in this exact operational condition." />;
  return (
    <main id="main" tabIndex={-1} className="page">
      <ConditionHeader condition={condition} />
      <article className="question-record">
        <header>
          <p className="eyebrow">Presentation-only upstream finding</p>
          <h1>{question.question}</h1>
        </header>
        <DefinitionGrid>
          <Field label="Question ID"><Exact wrap>{question.question_id}</Exact></Field>
          <Field label="Source kind"><Exact>{question.source.source_kind}</Exact></Field>
          <Field label="Source ordinal">{question.source_index}</Field>
          <Field label="Next lawful action"><Exact wrap>{question.next_lawful_action}</Exact></Field>
          <Field label="Presentation only"><Exact>{String(question.presentation_only)}</Exact></Field>
        </DefinitionGrid>
        <h2>Exact upstream finding</h2>
        <pre tabIndex={0} aria-label="Exact upstream finding">{JSON.stringify(question.source.finding, null, 2)}</pre>
        <p>This surface records no answer or disposition.</p>
      </article>
    </main>
  );
}

function RawArtifact({
  navigationId,
  kind,
  source,
}: {
  navigationId: string;
  kind: "monitor" | "nq" | "lineage" | "profile" | "evaluation";
  source: OperationalRawSource;
}) {
  const state = useRemote(() => getOperationalRaw(navigationId, kind), [navigationId, kind]);
  return (
    <Section title={`${kind} exact bytes`}>
      <DefinitionGrid>
        <Field label="SHA-256"><Exact wrap>{source.exact_bytes_sha256}</Exact></Field>
        <Field label="Byte length">{source.exact_bytes_length}</Field>
        <Field label="Validation"><Exact wrap>{source.validation}</Exact></Field>
      </DefinitionGrid>
      {state.data !== undefined ? (
        <pre tabIndex={0} aria-label={`Exact ${kind} bytes`}>{state.data}</pre>
      ) : (
        <ScreenState loading={state.loading} error={state.error} />
      )}
    </Section>
  );
}

export function OperationalRawView({ navigationId }: { navigationId: string }) {
  const state = useRemote(() => getOperationalCondition(navigationId), [navigationId]);
  const condition = state.data;
  if (!condition) return <ScreenState loading={state.loading} error={state.error} />;
  const kinds = ["monitor", "nq", "lineage", "profile", "evaluation"] as const;
  return (
    <main id="main" tabIndex={-1} className="page wide">
      <ConditionHeader condition={condition} />
      <header className="record-heading">
        <div>
          <p className="eyebrow">Exact operational source-byte inspection</p>
          <h1>Raw operational artifacts</h1>
          <p>Five fixed files from the explicitly supplied condition directory.</p>
        </div>
      </header>
      <div className="raw-grid">
        {kinds.map((kind) => (
          <RawArtifact
            key={kind}
            navigationId={navigationId}
            kind={kind}
            source={condition.raw_sources[kind]}
          />
        ))}
      </div>
    </main>
  );
}
