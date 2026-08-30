import { Exact, StringList } from "./components";
import { runPath } from "./router";
import type { CompatibleTimestamp, CompatibleValue, RendererJoinedValue } from "./contract";

export const UNRECOGNIZED_RECEIPT_VALUE = "Unrecognized receipt value";

export function recognized(value: CompatibleValue): string | null {
  return value.recognized_string;
}

export function timestampText(value: CompatibleTimestamp): string {
  return value.recognized_string ?? UNRECOGNIZED_RECEIPT_VALUE;
}

export function UnrecognizedValue({ runId }: { runId: string }) {
  return <span className="unrecognized">{UNRECOGNIZED_RECEIPT_VALUE} · <a href={runPath(runId) + "/raw"}>inspect raw receipts</a></span>;
}

export function CompatibleExact({ value, runId }: { value: CompatibleValue; runId: string }) {
  if (value.recognized_string !== null) return <Exact wrap>{value.recognized_string}</Exact>;
  return <UnrecognizedValue runId={runId} />;
}

export function CompatibleList({ value, runId }: { value: RendererJoinedValue; runId: string }) {
  if (value.recognized_strings !== null) return <StringList values={value.recognized_strings} />;
  return <UnrecognizedValue runId={runId} />;
}
