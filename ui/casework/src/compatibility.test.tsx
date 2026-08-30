import { render, screen } from "@testing-library/react";
import App from "./App";
import { runPath, workItemPath } from "./router";
import { at, installApiMock, run } from "./test/fixture";

describe("renderer-compatible raw-only values", () => {
  it("does not assign semantics to a non-string state or classification", async () => {
    const changed = structuredClone(run);
    changed.work_items[0].outcome.state.recognized_string = null;
    changed.work_items[0].outcome.result_classification.recognized_string = null;
    changed.summary.state_counts = {};
    changed.summary.unrecognized_state_count = 1;
    installApiMock(changed);
    at(runPath(run.run_id));
    const { container } = render(<App />);

    await screen.findByText("Showing 14 of 14 exact work items");
    expect(container.querySelectorAll(".unrecognized")).toHaveLength(2);
    expect(screen.getByText("Unrecognized state values").nextElementSibling).toHaveTextContent("1");
    for (const link of screen.getAllByRole("link", { name: "inspect raw receipts" })) {
      expect(link).toHaveAttribute("href", `${runPath(run.run_id)}/raw`);
    }
  });

  it("keeps unrecognized repository and joined values raw-only", async () => {
    const changed = structuredClone(run);
    const item = changed.work_items[0];
    item.outcome.repositories.recognized_rows = null;
    item.outcome.tests.recognized_strings = null;
    item.outcome.evidence.recognized_strings = null;
    item.outcome.live_or_production_mutations.recognized_strings = null;
    installApiMock(changed);
    at(workItemPath(run.run_id, item.id));
    const { container } = render(<App />);

    expect(await screen.findByText("Receipt value does not have the recognized repository-row shape.")).toBeVisible();
    expect(container.querySelectorAll(".unrecognized")).toHaveLength(4);
    expect(screen.queryByText(/canonical renderer json/i)).not.toBeInTheDocument();
  });
});
