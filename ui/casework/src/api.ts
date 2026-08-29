import type { CaseworkRun, RunIndex } from "./types";

async function get(path: string): Promise<Response> {
  const response = await fetch(path, { method: "GET", headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`GET ${path} returned ${response.status}`);
  return response;
}

export async function getRunIndex(): Promise<RunIndex> {
  return (await get("/api/v1/runs")).json() as Promise<RunIndex>;
}

export async function getRun(digest: string): Promise<CaseworkRun> {
  return (await get(`/api/v1/runs/${encodeURIComponent(digest)}`)).json() as Promise<CaseworkRun>;
}

export async function getRaw(digest: string, kind: "packet" | "receipts"): Promise<string> {
  return (await get(`/api/v1/runs/${encodeURIComponent(digest)}/raw/${kind}`)).text();
}
