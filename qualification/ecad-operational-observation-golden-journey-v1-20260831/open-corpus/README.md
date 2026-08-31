# SILICON-ORCHARD open corpus

This deliberately minimal corpus has no proprietary PDK, tool, scheduler, or
license dependency. The Monitor fixture generator hashes the exact bytes of the
RTL and expected/alternate tool, PDK, repository, input-artifact, and output
manifests under their family-owned identity domains. Alternate bytes make each
observed mismatch independently reproducible.

The fake tool does not execute. Its process-exit field is retained only as
mechanical testimony; NQ qualifies it as one independent value and neither
Nightshift nor Casework promotes it into a stage or job result.

The scheduler job, worker, license entitlement, stage occurrence, and generated
output identities use their explicit registry/occurrence/content contracts.
Every required scenario is a separate signed Monitor acquisition.
