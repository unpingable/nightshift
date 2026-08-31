import type { CaseworkLiveRun, CaseworkRun, LiveRunIndex, RunIndex } from "./contract";

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

export async function getLiveRunIndex(): Promise<LiveRunIndex> {
  return (await get("/api/v1/active-runs")).json() as Promise<LiveRunIndex>;
}

export async function getLiveRun(navigationId: string): Promise<CaseworkLiveRun> {
  return (await get(`/api/v1/active-runs/${encodeURIComponent(navigationId)}`)).json() as Promise<CaseworkLiveRun>;
}

export async function getLiveRaw(
  navigationId: string,
  kind: "packet" | "admission" | "profile" | "foreman-journal" | "accepted-receipts" | "final",
): Promise<string> {
  const response = await get(`/api/v1/active-runs/${encodeURIComponent(navigationId)}/raw/${kind}`);
  if (kind !== "foreman-journal" && kind !== "accepted-receipts") return response.text();
  const bytes = new Uint8Array(await response.arrayBuffer());
  return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
}
