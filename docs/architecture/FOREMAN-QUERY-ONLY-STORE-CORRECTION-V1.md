# Foreman query-only store correction V1

STILL-CIPHER established the descriptor-bound query-only constructor inherited by TUNNEL-FINCH. Every read/export command requires an existing regular database opened with `O_NOFOLLOW` and retained by file descriptor. SQLite opens `/proc/self/fd/{fd}` rather than reopening the operator-supplied pathname.

When WAL and SHM are both absent, `mode=ro&immutable=1` prevents observation from creating sidecars. When both exist, their regular-file identities are retained and revalidated across `mode=ro`; partial or changing sidecar custody is refused. No read path initializes schema or assigns a write pragma. Multi-query projections, worker briefs, and event exports use one deferred read transaction.

Qualification preserves exact directory entries, main/schema/WAL/SHM bytes, and final receipt bytes; refuses absent, symlink, non-regular, incomplete, partial-sidecar, and pathname-substitution fixtures; and creates no target-effect, approval, retry, subprocess, service, listener, provider session, secret, or aggregate result surface.
