export type Route =
  | { kind: "index" }
  | { kind: "run"; digest: string }
  | { kind: "work-item"; digest: string; id: string }
  | { kind: "question"; digest: string; id: string }
  | { kind: "custody"; digest: string }
  | { kind: "raw"; digest: string }
  | { kind: "live-run"; navigationId: string }
  | { kind: "live-work-item"; navigationId: string; id: string }
  | { kind: "live-question"; navigationId: string; id: string }
  | { kind: "live-events"; navigationId: string }
  | { kind: "live-raw"; navigationId: string }
  | { kind: "not-found" };

function decode(value: string): string | null {
  try {
    return decodeURIComponent(value);
  } catch {
    return null;
  }
}

export function parseRoute(pathname: string): Route {
  const parts = pathname.split("/").filter(Boolean);
  if (parts.length === 0) return { kind: "index" };
  if (parts[0] === "active-runs" && parts[1]) {
    const navigationId = decode(parts[1]);
    if (!navigationId) return { kind: "not-found" };
    if (parts.length === 2) return { kind: "live-run", navigationId };
    if (parts.length === 3 && parts[2] === "events") return { kind: "live-events", navigationId };
    if (parts.length === 3 && parts[2] === "raw") return { kind: "live-raw", navigationId };
    if (parts.length === 4 && parts[2] === "work-items") {
      const id = decode(parts[3]);
      return id ? { kind: "live-work-item", navigationId, id } : { kind: "not-found" };
    }
    if (parts.length === 4 && parts[2] === "questions") {
      const id = decode(parts[3]);
      return id ? { kind: "live-question", navigationId, id } : { kind: "not-found" };
    }
    return { kind: "not-found" };
  }
  if (parts[0] !== "runs" || !parts[1]) return { kind: "not-found" };
  const digest = decode(parts[1]);
  if (!digest) return { kind: "not-found" };
  if (parts.length === 2) return { kind: "run", digest };
  if (parts.length === 3 && parts[2] === "custody") return { kind: "custody", digest };
  if (parts.length === 3 && parts[2] === "raw") return { kind: "raw", digest };
  if (parts.length === 4 && parts[2] === "work-items") {
    const id = decode(parts[3]);
    return id ? { kind: "work-item", digest, id } : { kind: "not-found" };
  }
  if (parts.length === 4 && parts[2] === "questions") {
    const id = decode(parts[3]);
    return id ? { kind: "question", digest, id } : { kind: "not-found" };
  }
  return { kind: "not-found" };
}

export function runPath(digest: string): string {
  return `/runs/${encodeURIComponent(digest)}`;
}

export function workItemPath(digest: string, id: string): string {
  return `${runPath(digest)}/work-items/${encodeURIComponent(id)}`;
}

export function questionPath(digest: string, id: string): string {
  return `${runPath(digest)}/questions/${encodeURIComponent(id)}`;
}

export function liveRunPath(navigationId: string): string {
  return `/active-runs/${encodeURIComponent(navigationId)}`;
}

export function liveWorkItemPath(navigationId: string, id: string): string {
  return `${liveRunPath(navigationId)}/work-items/${encodeURIComponent(id)}`;
}

export function liveQuestionPath(navigationId: string, id: string): string {
  return `${liveRunPath(navigationId)}/questions/${encodeURIComponent(id)}`;
}
