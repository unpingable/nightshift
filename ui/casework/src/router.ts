export type Route =
  | { kind: "index" }
  | { kind: "run"; digest: string }
  | { kind: "work-item"; digest: string; id: string }
  | { kind: "question"; digest: string; id: string }
  | { kind: "custody"; digest: string }
  | { kind: "raw"; digest: string }
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
