import { render, screen } from "@testing-library/react";

import App from "./App";
import { at, installApiMock, liveIndex, liveRun, run } from "./test/fixture";

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
    at(`/active-runs/${liveRun.navigation_id}/questions/question%3Aone`);
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
});
