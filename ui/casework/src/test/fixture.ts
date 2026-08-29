import projectionJson from "../../../../qualification/nightshift-casework-mvp-20260829/velvet-orrery.casework-run.v1.json";
import type { CaseworkRun, RunIndex } from "../types";

export const run = projectionJson as CaseworkRun;

export const index: RunIndex = {
  schema: "nightshift.casework-run-index/v1",
  runs: [{
    run_id: run.run_id,
    projection_digest: run.projection_digest,
    packet_id: run.packet.packet_id,
    packet_digest: run.packet.packet_digest,
    receipt_updated_at: run.receipts.updated_at,
    summary: run.summary,
    packet_integrity: run.packet.integrity,
    packet_currentness_at_receipt_snapshot: run.packet.currentness_at_receipt_snapshot,
    packet_currentness_now: run.packet.currentness_now,
  }],
};

export const packetBytes = '{"exact":"packet bytes"}\n';
export const receiptBytes = '{"exact":"receipt bytes"}\n';

export function installApiMock(caseworkRun: CaseworkRun = run) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input);
    if (path === "/api/v1/runs") return new Response(JSON.stringify(index), { status: 200 });
    if (path.endsWith("/raw/packet")) return new Response(packetBytes, { status: 200 });
    if (path.endsWith("/raw/receipts")) return new Response(receiptBytes, { status: 200 });
    if (path.startsWith("/api/v1/runs/")) return new Response(JSON.stringify(caseworkRun), { status: 200 });
    return new Response("not found", { status: 404 });
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

export function at(path: string) {
  window.history.replaceState(null, "", path);
}
