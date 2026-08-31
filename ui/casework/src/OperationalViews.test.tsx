import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import App from "./App";
import type {
  CaseworkOperationalCondition,
  OperationalConditionIndex,
} from "./contract";
import {
  operationalConditionPath,
  operationalQuestionPath,
  operationalRawPath,
} from "./router";
import { at } from "./test/fixture";

const navigationId = "a".repeat(64);
const digest = (character: string) => `sha256:${character.repeat(64)}`;

const condition: CaseworkOperationalCondition = {
  schema: "nightshift.casework-operational-condition/v1",
  projection_digest: digest("1"),
  navigation_id: navigationId,
  subject: {
    kind: "physical_host",
    namespace: "subject:ecad-worker-01",
    basis_contract: "monitor.physical-host-dmi/v1",
    stable_basis: {
      basis_type: "physical_host_dmi",
      system_uuid_digest: digest("2"),
    },
  },
  subject_identity_digest: digest("3"),
  producer: {
    principal_id: "producer:monitor-fixture",
    collector_id: "collector:local-fixture",
    key_algorithm: "Ed25519",
    public_key_hex: "11".repeat(32),
    public_key_digest: digest("4"),
    producer_class: "monitor.operational-acquisition-producer/v1",
  },
  producer_identity_digest: digest("5"),
  acquisition_outcome: "observation_produced",
  lineage: {
    schema: "nightshift.operational-observation-lineage/v1",
    lineage_id: digest("6"),
    monitor_result_head: "b2d52fe34f146774cbf5601819982c267c7fb082",
    nq_result_head: "39b9f84f2f70955dd12e5cbfe798c740f9e52854",
    monitor_custody: {
      raw_bytes_sha256: digest("7"),
      raw_bytes_length: 117,
      semantic_digest: digest("8"),
    },
    nq_custody: {
      raw_bytes_sha256: digest("9"),
      raw_bytes_length: 221,
      semantic_digest: digest("a"),
    },
    nq_profile_id: "profile:ecad-worker-availability",
    nq_input_id: "input:monitor-observation-01",
    subject: {
      kind: "physical_host",
      namespace: "subject:ecad-worker-01",
      basis_contract: "monitor.physical-host-dmi/v1",
      stable_basis: {
        basis_type: "physical_host_dmi",
        system_uuid_digest: digest("2"),
      },
    },
    subject_identity_digest: digest("3"),
    producer: {
      principal_id: "producer:monitor-fixture",
      collector_id: "collector:local-fixture",
      key_algorithm: "Ed25519",
      public_key_hex: "11".repeat(32),
      public_key_digest: digest("4"),
      producer_class: "monitor.operational-acquisition-producer/v1",
    },
    producer_identity_digest: digest("5"),
    acquisition_outcome: "observation_produced",
    acquisition_started_at: "2026-08-30T03:00:00.100Z",
    acquisition_ended_at: "2026-08-30T03:00:00.200Z",
    producer_observed_at: "2026-08-30T03:00:00.150Z",
    receiver_custody_at: "2026-08-30T03:00:00.300Z",
    nq_qualified_at: "2026-08-30T03:00:00.400Z",
    nightshift_admitted_at: "2026-08-30T03:00:00.500Z",
    epoch: "epoch:ecad-lab-01",
    sequence: 7,
    predecessor_observation_digest: digest("b"),
    payload_schema: "monitor.ecad-worker-observation/v1",
    claim_support: [{
      claim_id: "claim:worker-available",
      proposition: "The exact acquisition supports worker availability.",
      value_digest: digest("c"),
      monitor_record_digest: digest("8"),
    }],
    cannot_testify: [{
      claim_id: "claim:scheduler-slot",
      reason: "profile claim absent from exact observation payload",
    }],
    refusals: [{
      code: "claim_basis_unavailable",
      exact_basis_digest: digest("d"),
      detail: "required evidence was not in the exact admitted input",
    }],
    contradictions: [{
      subject_identity_digest: digest("3"),
      claim_id: "claim:license-seat",
      first_input_id: "input:one",
      first_value_digest: digest("e"),
      second_input_id: "input:two",
      second_value_digest: digest("f"),
    }],
    nonclaims: [
      "Monitor testimony grants no target-effect authority.",
      "NQ qualification is not remediation.",
    ],
  },
  evaluation: {
    schema: "nightshift.operational-reobservation-evaluation/v1",
    evaluation_id: digest("0"),
    lineage_id: digest("6"),
    profile_id: "profile:ecad-worker-availability",
    profile_digest: digest("1"),
    max_age_seconds: 60,
    evaluated_at: "2026-08-30T03:00:00.600Z",
    current_until: "2026-08-30T03:01:00.150Z",
    exact_supported_claim_ids: ["claim:worker-available"],
    disposition: "CURRENT_SUPPORTED_CLAIMS_WITH_UNCERTAINTY",
    reobservation_trigger: "AT_OR_AFTER_CURRENT_UNTIL",
    next_lawful_action: "ACQUIRE_A_NEW_MONITOR_OBSERVATION",
    grants_authority: false,
  },
  profile: {
    profile_id: "profile:ecad-worker-availability",
    max_age_seconds: 60,
  },
  questions: [
    {
      navigation_id: "question-cannot-testify",
      question_id: digest("2"),
      question: "Review cannot-testify finding for claim:scheduler-slot",
      source_index: 0,
      source: {
        source_kind: "cannot_testify",
        finding: {
          claim_id: "claim:scheduler-slot",
          reason: "profile claim absent from exact observation payload",
        },
      },
      next_lawful_action: "ACQUIRE_A_NEW_MONITOR_OBSERVATION",
      presentation_only: true,
    },
    {
      navigation_id: "question-refusal",
      question_id: digest("3"),
      question: "Review refusal claim_basis_unavailable",
      source_index: 0,
      source: {
        source_kind: "refusal",
        finding: {
          code: "claim_basis_unavailable",
          exact_basis_digest: digest("d"),
          detail: "required evidence was not in the exact admitted input",
        },
      },
      next_lawful_action: "ACQUIRE_A_NEW_MONITOR_OBSERVATION",
      presentation_only: true,
    },
    {
      navigation_id: "question-contradiction",
      question_id: digest("4"),
      question: "Review contradiction for claim:license-seat",
      source_index: 0,
      source: {
        source_kind: "contradiction",
        finding: {
          subject_identity_digest: digest("3"),
          claim_id: "claim:license-seat",
          first_input_id: "input:one",
          first_value_digest: digest("e"),
          second_input_id: "input:two",
          second_value_digest: digest("f"),
        },
      },
      next_lawful_action: "ACQUIRE_A_NEW_MONITOR_OBSERVATION",
      presentation_only: true,
    },
  ],
  raw_sources: Object.fromEntries(
    ["monitor", "nq", "lineage", "profile", "evaluation"].map((kind, index) => [
      kind,
      {
        exact_bytes_sha256: digest(String(index + 1)),
        exact_bytes_length: 100 + index,
        validation: "exact_owner_artifact_validated",
      },
    ]),
  ) as CaseworkOperationalCondition["raw_sources"],
  authority_effect: "read_only_projection_no_authority",
};

const index: OperationalConditionIndex = {
  schema: "nightshift.casework-operational-condition-index/v1",
  conditions: [{
    navigation_id: navigationId,
    projection_digest: condition.projection_digest,
    lineage_id: condition.lineage.lineage_id,
    evaluation_id: condition.evaluation.evaluation_id,
    subject_kind: condition.subject.kind,
    subject_namespace: condition.subject.namespace,
    subject_identity_digest: condition.subject_identity_digest,
    disposition: condition.evaluation.disposition,
    reobservation_trigger: condition.evaluation.reobservation_trigger,
    evaluated_at: condition.evaluation.evaluated_at,
    question_count: condition.questions.length,
  }],
};

const raw = {
  monitor: '{"exact":"monitor","unknown_extension":"raw-only"}\n',
  nq: '{"exact":"nq"}\n',
  lineage: '{"exact":"lineage"}\n',
  profile: '{"exact":"profile"}\n',
  evaluation: '{"exact":"evaluation"}\n',
};

function installOperationalMock() {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    if (init?.method !== "GET") return new Response("method not allowed", { status: 405 });
    if (path === "/api/v1/operational-conditions") {
      return new Response(JSON.stringify(index), { status: 200 });
    }
    if (path === `/api/v1/operational-conditions/${navigationId}`) {
      return new Response(JSON.stringify(condition), { status: 200 });
    }
    const match = path.match(/\/raw\/(monitor|nq|lineage|profile|evaluation)$/);
    if (match) return new Response(raw[match[1] as keyof typeof raw], { status: 200 });
    return new Response("not found", { status: 404 });
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("operational condition casework journeys", () => {
  it("refreshes a direct condition link with exact independent owner facts", async () => {
    installOperationalMock();
    at(operationalConditionPath(navigationId));
    const { container } = render(<App />);
    expect(await screen.findByRole("heading", { name: "subject:ecad-worker-01" })).toBeVisible();
    for (const exact of [
      "claim:worker-available",
      "profile claim absent from exact observation payload",
      "claim_basis_unavailable",
      "claim:license-seat",
      "CURRENT_SUPPORTED_CLAIMS_WITH_UNCERTAINTY",
      "AT_OR_AFTER_CURRENT_UNTIL",
      "ACQUIRE_A_NEW_MONITOR_OBSERVATION",
      "2026-08-30T03:00:00.150Z",
      "2026-08-30T03:00:00.300Z",
      "2026-08-30T03:00:00.400Z",
      "2026-08-30T03:00:00.500Z",
    ]) expect(screen.getAllByText(exact).length).toBeGreaterThan(0);
    expect(container.querySelectorAll("button, textarea, input, form, [contenteditable='true']")).toHaveLength(0);
    expect(container.textContent).not.toMatch(/unknown_extension|overall health|aggregate health/i);
  });

  it("keeps a deep-linked upstream question presentation-only", async () => {
    installOperationalMock();
    at(operationalQuestionPath(navigationId, "question-contradiction"));
    const { container } = render(<App />);
    expect(await screen.findByRole("heading", { name: "Review contradiction for claim:license-seat" })).toBeVisible();
    expect(screen.getByText("contradiction")).toBeVisible();
    expect(screen.getByText("true")).toBeVisible();
    expect(screen.getByLabelText("Exact upstream finding")).toHaveAttribute("tabindex", "0");
    expect(container.textContent).toContain("This surface records no answer or disposition.");
    expect(container.querySelectorAll("button, textarea, input, form, [contenteditable='true']")).toHaveLength(0);
  });

  it("renders all five exact source byte streams through fixed GET routes", async () => {
    const fetchMock = installOperationalMock();
    at(operationalRawPath(navigationId));
    render(<App />);
    for (const [kind, bytes] of Object.entries(raw)) {
      expect((await screen.findByLabelText(`Exact ${kind} bytes`)).textContent).toBe(bytes);
      expect(screen.getByLabelText(`Exact ${kind} bytes`)).toHaveAttribute("tabindex", "0");
      expect(fetchMock).toHaveBeenCalledWith(
        `/api/v1/operational-conditions/${navigationId}/raw/${kind}`,
        expect.objectContaining({ method: "GET" }),
      );
    }
  });

  it("supports keyboard navigation from the operational index", async () => {
    const user = userEvent.setup();
    installOperationalMock();
    at("/operational-conditions");
    render(<App />);
    const link = await screen.findByRole("link", { name: "subject:ecad-worker-01" });
    link.focus();
    expect(link).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(await screen.findByRole("heading", { name: "subject:ecad-worker-01" })).toBeVisible();
    expect(window.location.pathname).toBe(operationalConditionPath(navigationId));
  });
});
