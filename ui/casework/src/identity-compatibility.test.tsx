import { render, screen } from "@testing-library/react";
import App from "./App";
import { questionPath, runPath } from "./router";
import { at, installApiMock, run } from "./test/fixture";

describe("nullable receipt identity navigation", () => {
  it("does not invent a work-item link for an unlinked question", async () => {
    const changed = structuredClone(run);
    const question = changed.human_questions[0];
    question.linked_work_item = null;
    question.work_item.recognized_string = "receipt-only-work-item";
    installApiMock(changed);
    at(questionPath(changed.run_id, question.navigation_id));
    render(<App />);

    expect(await screen.findByText("receipt-only-work-item")).toBeVisible();
    expect(screen.queryByRole("link", { name: "receipt-only-work-item" })).not.toBeInTheDocument();
  });

  it("routes unrecognized question and custody identities to exact raw receipts", async () => {
    const changed = structuredClone(run);
    const question = changed.human_questions[0];
    question.derived_id = null;
    question.linked_work_item = null;
    question.work_item.recognized_string = null;
    installApiMock(changed);
    at(questionPath(changed.run_id, question.navigation_id));
    const questionView = render(<App />);

    expect(await screen.findAllByRole("link", { name: "inspect raw receipts" })).not.toHaveLength(0);
    for (const link of screen.getAllByRole("link", { name: "inspect raw receipts" })) {
      expect(link).toHaveAttribute("href", `${runPath(changed.run_id)}/raw`);
    }
    questionView.unmount();

    const custody = changed.final_repository_custody[0];
    custody.derived_id = null;
    custody.repository.recognized_string = null;
    at(`${runPath(changed.run_id)}/custody`);
    render(<App />);
    expect(await screen.findByRole("heading", { name: /Unrecognized receipt value/ })).toBeVisible();
    expect(screen.getByRole("link", { name: "inspect raw receipts" })).toHaveAttribute(
      "href",
      `${runPath(changed.run_id)}/raw`,
    );
  });
});
