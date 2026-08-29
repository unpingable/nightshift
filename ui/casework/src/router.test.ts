import { parseRoute, questionPath, runPath, workItemPath } from "./router";

describe("stable casework routes", () => {
  const digest = "01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7";

  it("parses every public route and round-trips encoded record ids", () => {
    expect(parseRoute(runPath(digest))).toEqual({ kind: "run", digest });
    expect(parseRoute(workItemPath(digest, "bedrock/docket"))).toEqual({ kind: "work-item", digest, id: "bedrock/docket" });
    expect(parseRoute(questionPath(digest, "sha256:question"))).toEqual({ kind: "question", digest, id: "sha256:question" });
    expect(parseRoute(`${runPath(digest)}/custody`)).toEqual({ kind: "custody", digest });
    expect(parseRoute(`${runPath(digest)}/raw`)).toEqual({ kind: "raw", digest });
  });

  it("fails closed for incomplete and additional path segments", () => {
    expect(parseRoute("/runs")).toEqual({ kind: "not-found" });
    expect(parseRoute(`${runPath(digest)}/raw/packet`)).toEqual({ kind: "not-found" });
    expect(parseRoute("/something-else")).toEqual({ kind: "not-found" });
  });
});
