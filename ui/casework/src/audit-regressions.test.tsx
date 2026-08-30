import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { runPath } from "./router";
import { at, installApiMock, run } from "./test/fixture";

describe("casework UI audit regressions", () => {
  it("activates same-path fragment links without the SPA router consuming them", async () => {
    const user = userEvent.setup();
    installApiMock();
    at(runPath(run.run_id));
    render(<App />);
    await screen.findByText("Showing 14 of 14 exact work items");

    const pushState = vi.spyOn(window.history, "pushState");
    const main = screen.getByRole("main");
    expect(main).toHaveAttribute("tabindex", "-1");
    await user.click(screen.getByRole("link", { name: "Skip to casework" }));
    await waitFor(() => expect(window.location.hash).toBe("#main"));
    expect(pushState).not.toHaveBeenCalled();
    main.focus();
    expect(main).toHaveFocus();

    const questionsLink = screen.getByRole("link", { name: "Human questions" });
    const questions = document.getElementById("human-questions");
    expect(questions).not.toBeNull();
    expect(questions).toHaveAttribute("tabindex", "-1");
    await user.click(questionsLink);
    await waitFor(() => expect(window.location.hash).toBe("#human-questions"));
    expect(pushState).not.toHaveBeenCalled();
    questions!.focus();
    expect(questions).toHaveFocus();
    expect(window.location.pathname).toBe(runPath(run.run_id));
  });

  it("filters empty and whitespace-bearing exact states without sentinel collisions", async () => {
    const user = userEvent.setup();
    const changed = structuredClone(run);
    changed.work_items[0].outcome.state.recognized_string = "";
    changed.work_items[1].outcome.state.recognized_string = "  QUALIFIED  ";
    installApiMock(changed);
    at(runPath(changed.run_id));
    render(<App />);
    await screen.findByText("Showing 14 of 14 exact work items");

    const select = screen.getByLabelText("Exact state");
    const options = within(select).getAllByRole("option") as HTMLOptionElement[];
    const all = options.find((option) => option.textContent === "All exact states")!;
    const empty = options.find((option) => option.textContent === '\"\" (empty string)')!;
    const whitespace = options.find((option) => option.textContent?.includes("whitespace preserved"))!;

    expect(all.value).toBe("all-states");
    expect(empty.value).toBe("state:0");
    expect(whitespace.value).toBe("state:1");
    expect(options.slice(1).every((option) => option.value.startsWith("state:"))).toBe(true);

    await user.selectOptions(select, empty);
    expect(screen.getByText("Showing 1 of 14 exact work items")).toBeVisible();
    expect(screen.getByRole("link", { name: "VELVET-ORRERY" })).toBeVisible();
    expect(screen.queryByRole("link", { name: "QUIET-BRIDGE" })).not.toBeInTheDocument();

    await user.selectOptions(select, whitespace);
    expect(screen.getByText("Showing 1 of 14 exact work items")).toBeVisible();
    expect(screen.getByRole("link", { name: "QUIET-BRIDGE" })).toBeVisible();
    expect(screen.queryByRole("link", { name: "VELVET-ORRERY" })).not.toBeInTheDocument();

    await user.selectOptions(select, all);
    expect(screen.getByText("Showing 14 of 14 exact work items")).toBeVisible();
  });
});
