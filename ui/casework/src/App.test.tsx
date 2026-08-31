import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { questionPath, runPath, workItemPath } from "./router";
import { at, index, installApiMock, packetBytes, receiptBytes, run } from "./test/fixture";

const digest = run.run_id;

describe("Nightshift Casework golden journeys", () => {
  it("indexes the exact VELVET run facts and exact-state counts", async () => {
    installApiMock(); at("/"); render(<App />);
    expect(await screen.findByRole("heading", { name: run.packet.packet_id })).toBeVisible();
    expect(screen.getByText("14")).toBeVisible();
    expect(screen.getByText("6")).toBeVisible();
    expect(screen.getByText("CLOSEOUT-COMPLETE-NOT-QUALIFIED")).toBeVisible();
    expect(screen.getByText(run.packet.packet_digest)).toBeVisible();
    expect(screen.getByText("EXPIRED")).toBeVisible();
    expect(screen.queryByText(/aggregate verdict/i)).not.toBeInTheDocument();
  });

  it("shows all 14 items, preserves exact classifications, and filters by exact fields", async () => {
    const user = userEvent.setup(); installApiMock(); at(runPath(digest)); render(<App />);
    expect(await screen.findByText("Showing 14 of 14 exact work items")).toBeVisible();
    expect(screen.getByRole("link", { name: "RIVER-CLERK" })).toBeVisible();
    expect(screen.getByRole("link", { name: "GLASSHOPPER" })).toBeVisible();
    expect(screen.getByText("CLOSEOUT-COMPLETE-CAMPAIGN-NOT-QUALIFIED")).toBeVisible();

    await user.selectOptions(screen.getByLabelText("Exact state"), "CLOSEOUT-COMPLETE-NOT-QUALIFIED");
    expect(screen.getByText("Showing 1 of 14 exact work items")).toBeVisible();
    expect(screen.getByRole("link", { name: "GLASSHOPPER" })).toBeVisible();
    expect(screen.queryByRole("link", { name: "RIVER-CLERK" })).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Exact state"), "all-states");
    await user.selectOptions(screen.getByLabelText("Track"), "bedrock-prerequisite");
    expect(screen.getByText("Showing 3 of 14 exact work items")).toBeVisible();
    await user.selectOptions(screen.getByLabelText("Track"), "");
    await user.selectOptions(screen.getByLabelText("Human question"), "with");
    expect(screen.getByText("Showing 6 of 14 exact work items")).toBeVisible();
  });

  it("exposes the RIVER identity-contract successor requirement without smoothing", async () => {
    installApiMock(); at(workItemPath(digest, "bedrock-docket-executor")); render(<App />);
    expect(await screen.findByRole("heading", { name: "RIVER-CLERK" })).toBeVisible();
    expect(screen.getAllByText("TERMINAL-NOT-QUALIFIED").length).toBeGreaterThan(0);
    expect(screen.getAllByText("NOT-QUALIFIED-IDENTITY-CONTRACT-SUCCESSOR-REQUIRED").length).toBeGreaterThan(0);
    expect(screen.getByText(/prepared-occurrence contract binding NQ plan and Docket executor-plan identities without circularity/)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Bounded intent" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Recorded outcome" })).toBeVisible();
  });

  it("shows GLASSHOPPER as closeout-complete and not qualified exactly", async () => {
    installApiMock(); at(workItemPath(digest, "glasshopper-closeout")); render(<App />);
    expect(await screen.findByRole("heading", { name: "GLASSHOPPER" })).toBeVisible();
    expect(screen.getAllByText("CLOSEOUT-COMPLETE-NOT-QUALIFIED").length).toBeGreaterThan(0);
    expect(screen.getAllByText("CLOSEOUT-COMPLETE-CAMPAIGN-NOT-QUALIFIED").length).toBeGreaterThan(0);
  });

  it("renders all six exact human questions and all question record fields", async () => {
    installApiMock(); at(runPath(digest)); const { unmount } = render(<App />);
    expect(await screen.findByRole("heading", { name: "Human questions · 6" })).toBeVisible();
    const list = screen.getByRole("heading", { name: "Human questions · 6" }).closest("section")!;
    expect(within(list).getAllByRole("listitem")).toHaveLength(6);
    unmount();
    const question = run.human_questions[0];
    at(questionPath(digest, question.navigation_id)); render(<App />);
    expect(await screen.findByRole("heading", { name: question.exact_question.recognized_string! })).toBeVisible();
    for (const label of ["Evidence exhausted", "Safe default", "Consequences", "Resume point"]) {
      expect(screen.getByText(label)).toBeVisible();
    }
    expect(screen.getByRole("link", { name: question.linked_work_item! })).toBeVisible();
  });

  it("keeps starting and final custody separate and leaves exact custody prose uninterpreted", async () => {
    installApiMock(); at(`${runPath(digest)}/custody`); render(<App />);
    expect(await screen.findByRole("heading", { name: "Starting packet custody" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Final receipt custody" })).toBeVisible();
    expect(screen.getByText("sole local; authoritative remote absent")).toBeVisible();
    expect(screen.getAllByText(/remote verified exact/).length).toBeGreaterThan(0);
    expect(screen.getByText(/Text is displayed without inferred disposition/)).toBeVisible();
  });

  it("shows exact raw bytes, digests, and validation dispositions read-only", async () => {
    installApiMock(); at(`${runPath(digest)}/raw`); render(<App />);
    expect((await screen.findByLabelText("Exact packet bytes")).textContent).toBe(packetBytes);
    expect(screen.getByLabelText("Exact receipt bytes").textContent).toBe(receiptBytes);
    expect(screen.getByText(run.packet.source_bytes_digest)).toBeVisible();
    expect(screen.getByText(run.receipts.source_bytes_digest)).toBeVisible();
    expect(screen.getByText("VALID_PACKET_INTEGRITY")).toBeVisible();
    expect(screen.getByText("VALID_RENDERER_COMPATIBLE_RECEIPT_SNAPSHOT")).toBeVisible();
    expect(screen.getByLabelText("Exact packet bytes")).toHaveAttribute("tabindex", "0");
  });

  it("preserves an unknown state and classification verbatim", async () => {
    const changed = structuredClone(run);
    changed.work_items[0].outcome.state.recognized_string = "STATE-NOT-IN-ANY-TAXONOMY";
    changed.work_items[0].outcome.result_classification.recognized_string = "UNCLASSIFIED-LITERAL";
    installApiMock(changed); at(runPath(digest)); render(<App />);
    expect((await screen.findAllByText("STATE-NOT-IN-ANY-TAXONOMY")).length).toBeGreaterThan(0);
    expect(screen.getByText("UNCLASSIFIED-LITERAL")).toBeVisible();
  });

  it("contains navigation and filters but no mutation controls", async () => {
    installApiMock(); at(runPath(digest)); const { container } = render(<App />);
    await screen.findByText("Showing 14 of 14 exact work items");
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(container.querySelectorAll("textarea, input, [contenteditable='true']")).toHaveLength(0);
    expect(container.querySelectorAll("form")).toHaveLength(1);
    expect(within(container.querySelector("form")!).getAllByRole("combobox")).toHaveLength(3);
  });

  it("uses only the accepted same-origin GET endpoints", async () => {
    const fetchMock = installApiMock(); at("/"); render(<App />);
    await screen.findByRole("heading", { name: index.runs[0].packet_id });
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(fetchMock).toHaveBeenCalledWith("/api/v1/runs", expect.objectContaining({ method: "GET" }));
    expect(fetchMock).toHaveBeenCalledWith("/api/v1/active-runs", expect.objectContaining({ method: "GET" }));
    expect(fetchMock).toHaveBeenCalledWith("/api/v1/operational-conditions", expect.objectContaining({ method: "GET" }));
  });
});
