import type { CaseworkLiveProviderExecution, CaseworkLiveRun, CaseworkOperationalCondition, CaseworkRun, LiveRunIndex, OperationalConditionIndex, RunIndex } from "./contract";

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

export async function getLiveProviderExecution(
  navigationId: string,
): Promise<CaseworkLiveProviderExecution> {
  return (
    await get(
      `/api/v1/active-runs/${encodeURIComponent(navigationId)}/provider-execution`,
    )
  ).json() as Promise<CaseworkLiveProviderExecution>;
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
export async function getOperationalConditionIndex(): Promise<OperationalConditionIndex> {
  return (await get("/api/v1/operational-conditions")).json() as Promise<OperationalConditionIndex>;
}

export async function getOperationalCondition(
  navigationId: string,
): Promise<CaseworkOperationalCondition> {
  return (
    await get(`/api/v1/operational-conditions/${encodeURIComponent(navigationId)}`)
  ).json() as Promise<CaseworkOperationalCondition>;
}

export async function getOperationalRaw(
  navigationId: string,
  kind: "monitor" | "nq" | "lineage" | "profile" | "evaluation",
): Promise<string> {
  return (
    await get(
      `/api/v1/operational-conditions/${encodeURIComponent(navigationId)}/raw/${kind}`,
    )
  ).text();
}
