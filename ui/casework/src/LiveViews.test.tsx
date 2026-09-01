import { render, screen, within } from "@testing-library/react";

import App from "./App";
import {
  at,
  installApiMock,
  liveIndex,
  liveRun,
  operationalIndex,
  providerExecution,
  providerExecutionAbsent,
  run,
} from "./test/fixture";

function recordedCapacityRun(): typeof liveRun {
  return {
    ...liveRun,
    execution_profile: {
      ...liveRun.execution_profile,
      capacity_binding_status: "EXACT_RECORDED_CAPACITY_REQUIREMENT",
    },
    provider_capacity: {
      status: "EXACT_RECORDED_BY_FOREMAN",
      requirement: {
        capacity_requirement_digest: "sha256:" + "1".repeat(64),
        exact_bytes_sha256: "sha256:" + "2".repeat(64),
        recorded_at: "2026-08-31T00:00:00Z",
        policy_id: "policy:fixture",
        provider_id: "provider:fixture",
        model_cost_classes: { bounded: "CHEAP" },
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
      },
      attempts: [{
        journal_sequence: 3,
        work_item_id: "lane-a",
        attempt_id: "attempt:one",
        recorded_at: "2026-08-31T00:00:00Z",
        provider_id: "provider:fixture",
        packet_model_class: "bounded",
        profile_model_class: "bounded",
        cost_class: "CHEAP",
        capacity_state: "CONSERVE",
        admission_disposition: "CHEAP_BOUNDED_ONLY",
        source_class: "OBSERVED",
        confidence: "HIGH",
        observation_disposition: "USABLE",
        observed_at: "2026-08-30T23:59:59Z",
        expires_at: "2026-08-31T00:10:00Z",
        decision_at: "2026-08-31T00:00:00Z",
        evaluated_at: "2026-08-31T00:00:00Z",
        currentness: "CURRENT",
        capacity_admission_digest: "sha256:" + "3".repeat(64),
        observation_digest: "sha256:" + "4".repeat(64),
        policy_digest: "sha256:" + "5".repeat(64),
        decision_digest: "sha256:" + "6".repeat(64),
        admission_exact_bytes_sha256: "sha256:" + "7".repeat(64),
        observation_exact_bytes_sha256: "sha256:" + "8".repeat(64),
        policy_exact_bytes_sha256: "sha256:" + "9".repeat(64),
        decision_exact_bytes_sha256: "sha256:" + "a".repeat(64),
      }],
      explanation: "Exact journal-recorded mechanism evidence; no campaign result.",
    },
  };
}

function primitiveValues(value: unknown): string[] {
  if (value === null || value === undefined) return [];
  if (typeof value === "object") return Object.values(value).flatMap(primitiveValues);
  return [String(value)];
}

describe("live Casework projection", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("keeps active foreman runs separate from sealed receipt cases", async () => {
    installApiMock();
    at("/");
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Active foreman runs" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Sealed receipt cases" })).toBeVisible();
    expect(await screen.findByText("live-packet")).toBeVisible();
    expect(screen.queryByText(/overall health/i)).not.toBeInTheDocument();
  });

  it("renders intent mechanism and explicit accepted-outcome absence on a direct route", async () => {
    installApiMock();
    at(`/active-runs/${liveRun.navigation_id}/work-items/lane-a`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "1 · Bounded packet intent" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "2 · Live mechanism and attempt" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "3 · Accepted terminal / not-started receipt or explicit absence" })).toBeVisible();
    expect(screen.getByText("NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT")).toBeVisible();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders lane-local questions and the exact event timeline without response controls", async () => {
    installApiMock();
    at(`/active-runs/${liveRun.navigation_id}/questions/${liveRun.work_items[0].human_questions[0].navigation_id}`);
    const { unmount } = render(<App />);
    expect(await screen.findByText("Which explicit input is required?")).toBeVisible();
    expect(screen.getByText("This read-only surface records no answer or disposition.")).toBeVisible();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    unmount();

    at(`/active-runs/${liveRun.navigation_id}/events`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Exact append-only event timeline" })).toBeVisible();
    expect(screen.getByText("event:one")).toBeVisible();
  });

  it("keeps repeated lane-local question IDs bound to distinct lane navigation", async () => {
    const shared = liveRun.work_items[0].human_questions[0];
    const repeated = {
      ...liveRun,
      work_items: [
        {
          ...liveRun.work_items[0],
          human_questions: [{ ...shared, question_id: "question:shared", question: "Lane A exact question" }],
        },
        {
          ...liveRun.work_items[0],
          work_item_id: "lane-b",
          campaign_codename: "LANE-B",
          campaign_slug: "lane-b",
          human_questions: [{
            ...shared,
            navigation_id: "f".repeat(64),
            question_id: "question:shared",
            question: "Lane B exact question",
          }],
        },
      ],
    };
    installApiMock(run, liveIndex, repeated);
    at(`/active-runs/${repeated.navigation_id}/questions/${repeated.work_items[1].human_questions[0].navigation_id}`);
    render(<App />);
    expect(await screen.findByText("Lane B exact question")).toBeVisible();
    expect(screen.queryByText("Lane A exact question")).not.toBeInTheDocument();
  });

  it("renders exact recorded provider-capacity attempts as mechanism facts without controls", async () => {
    const recorded = recordedCapacityRun();
    installApiMock(run, liveIndex, recorded);
    at(`/active-runs/${recorded.navigation_id}`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Recorded provider-capacity mechanism" })).toBeVisible();
    expect(screen.getByText("EXACT_RECORDED_BY_FOREMAN")).toBeVisible();
    expect(screen.getByText(/CONSERVE · CHEAP_BOUNDED_ONLY/)).toBeVisible();
    expect(screen.getByText(/OBSERVED · HIGH · USABLE/)).toBeVisible();
    const capacityRow = screen.getByText(/attempt:one · journal 3/).closest("tr");
    expect(capacityRow).not.toBeNull();
    expect(within(capacityRow!).getByText(/packet bounded · profile bounded · CHEAP/)).toBeVisible();
    expect(capacityRow).toHaveTextContent("recorded 2026-08-31T00:00:00Z · evaluated 2026-08-31T00:00:00Z");
    for (const digit of ["3", "4", "5", "6"]) {
      expect(capacityRow).toHaveTextContent("sha256:" + digit.repeat(64));
    }
    for (const digit of ["7", "8", "9", "a"]) {
      expect(capacityRow).toHaveTextContent("sha256:" + digit.repeat(64));
    }
    expect(screen.queryByText("NOT_RECORDED_BY_FOREMAN")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders the legacy no-record status and explanation without inference", async () => {
    installApiMock();
    at(`/active-runs/${liveRun.navigation_id}`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Recorded provider-capacity mechanism" })).toBeVisible();
    expect(screen.getByText("NOT_RECORDED_BY_FOREMAN")).toBeVisible();
    expect(screen.getByText("No exact capacity decision was recorded.")).toBeVisible();
    expect(screen.queryByText("EXACT_RECORDED_BY_FOREMAN")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders the complete recorded provider-execution mechanism and exact raw custody without controls", async () => {
    const recorded = recordedCapacityRun();
    const fetchMock = installApiMock(run, liveIndex, recorded, operationalIndex, providerExecution);
    at(`/active-runs/${recorded.navigation_id}/provider-execution`);
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Provider execution availability" })).toBeVisible();
    for (const heading of [
      "Exact requirement and ordered model selections",
      "Work attempts and fresh dispatch occurrences",
      "Exact provider-admission dispositions",
      "Deferred wake, backoff, and fallback",
      "Resource release and reacquisition",
      "Same-execution resumes",
      "Independent FUEL capacity evidence",
      "Exact raw evidence routes",
    ]) {
      expect(screen.getByRole("heading", { name: heading })).toBeVisible();
    }
    for (const state of [
      "PARKED_NOT_ADMITTED",
      "ADMISSION_INDETERMINATE",
      "EXECUTION_ADMITTED",
      "WAITING_APPROVAL",
      "POST_ADMISSION_INTERRUPTED",
      "PROVIDER_COMPLETED",
    ]) {
      expect(document.body).toHaveTextContent(state);
    }
    for (const value of new Set(primitiveValues(providerExecution))) {
      expect(document.body).toHaveTextContent(value);
    }
    for (const digit of ["3", "4", "5", "6", "7", "8", "9", "a"]) {
      expect(document.body).toHaveTextContent("sha256:" + digit.repeat(64));
    }
    expect(screen.getByRole("link", { name: "Exact event 3 bytes" })).toHaveAttribute(
      "href",
      `/api/v1/active-runs/${recorded.navigation_id}/events/3/raw`,
    );
    expect(fetchMock).toHaveBeenCalledWith(
      `/api/v1/active-runs/${recorded.navigation_id}/provider-execution`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(document.querySelector("form")).not.toBeInTheDocument();
  });

  it("renders explicit provider-execution history absence without inferred mechanism facts", async () => {
    installApiMock(run, liveIndex, liveRun, operationalIndex, providerExecutionAbsent);
    at(`/active-runs/${liveRun.navigation_id}/provider-execution`);
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Provider-execution history absence" })).toBeVisible();
    expect(screen.getAllByText("NOT_RECORDED_BY_FOREMAN").length).toBeGreaterThan(0);
    expect(screen.getAllByText(providerExecutionAbsent.explanation).length).toBe(2);
    expect(screen.queryByRole("heading", { name: "Exact provider-admission dispositions" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(document.querySelector("form")).not.toBeInTheDocument();
  });

  it("shows reciprocal navigation only for a server-qualified exact final-byte match", async () => {
    installApiMock(run, {
      ...liveIndex,
      runs: liveIndex.runs.map((entry) => ({ ...entry, sealed_case_run_id: run.run_id })),
    });
    at(`/runs/${run.run_id}`);
    render(<App />);
    expect(await screen.findByRole("link", { name: "Open byte-matched live foreman history" }))
      .toHaveAttribute("href", `/active-runs/${liveRun.navigation_id}`);
  });

  it("exposes final and per-event exact raw sources without adding controls", async () => {
    const closed = {
      ...liveRun,
      raw_sources: {
        ...liveRun.raw_sources,
        final_snapshot_sha256: "sha256:" + "f".repeat(64),
      },
    };
    const fetchMock = installApiMock(run, liveIndex, closed);
    at(`/active-runs/${closed.navigation_id}/raw`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Exact final snapshot" })).toBeVisible();
    expect(await screen.findByText('{"exact":"final snapshot bytes"}')).toBeVisible();
    expect(screen.getByRole("link", { name: /Event 1/ })).toHaveAttribute(
      "href",
      `/api/v1/active-runs/${closed.navigation_id}/events/1/raw`,
    );
    expect(fetchMock).toHaveBeenCalledWith(
      `/api/v1/active-runs/${closed.navigation_id}/raw/final`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});
