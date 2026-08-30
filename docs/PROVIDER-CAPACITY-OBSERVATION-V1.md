# Provider capacity observation V1

FUEL-NEEDLE defines a provider-neutral, read-only boundary between capacity
testimony and Nightshift foreman scheduling. It does not define provider account
authority, billing, credential custody, or target-effect authority.

    provider-owned supported read surface
                    |
                    v
    closed normalized observation + exact raw-byte digest
                    |
                    v
    closed policy + deterministic decision
                    |
                    v
    provider-neutral foreman admission boundary

The three records are independent:

- nightshift.provider-capacity-observation/v1 says what capacity testimony was
  obtained, from which source class, with what confidence and expiry.
- nightshift.provider-capacity-policy/v1 owns thresholds and the canonical set
  of window types required for admission. Neither enters the observation
  protocol.
- nightshift.provider-capacity-decision/v1 binds the exact observation and
  policy digests used for one decision.

All record digests use RFC 8785/JCS with the record digest field omitted and a
NUL-terminated domain prefix named by the implementation. The raw source digest
uses its own domain. Raw provider response bytes are never printed, stored in
the record, or written to the repository.

## Source, confidence, and absence

Source class and confidence are orthogonal. AUTHORITATIVE, OBSERVED, INFERRED,
and UNKNOWN describe provenance; HIGH, MEDIUM, LOW, and UNKNOWN describe
confidence. Missing output, timeout, provider refusal, unrecognized layout,
impossible values, contradictions, expiry, reset rollover, an absent
policy-required window, or unverified executable/protocol identity produce
explicit UNKNOWN decisions. They never mean unlimited capacity.

Context-window consumption is not quota capacity and is not parsed into a
capacity window. Unknown source extensions remain only in the raw response
digest and acquire no semantic meaning.

## Codex bootstrap probe

The only live adapter invokes an operator-supplied canonical path to the native
Codex executable through its already-open descriptor:

    codex app-server --listen stdio://
    initialize
    initialized
    account/rateLimits/read

The operator supplies the canonical native-executable path, its exact raw
SHA-256 digest, and the expected protocol version. Before spawn, the probe opens
the regular executable, verifies executable mode, native format, canonical
path, and digest, and then invokes `/proc/self/fd/N`; a wrapper, symlink path,
pathname replacement, or content mutation cannot substitute the opened file.
The initialize response must independently report the exact expected
`codex_cli_rs/VERSION` before the rate-limit response becomes usable.

It creates no thread or model turn. The executable and method are fixed. The
collector reads fixed-size chunks, checks the total bound before extending a
message buffer, and has a deadline. It drains but does not retain diagnostics,
kills and reaps its own foreground App Server process, joins its reader threads,
and emits only the normalized record. It does not read provider configuration,
session, browser, or credential files directly; it does not expose login,
logout, reset, consume, or account-mutation methods. Other providers must
supply their own adapter into the same normalized contract. Platforms without
descriptor-pinned execution return UNKNOWN.

The profile locator is operator-supplied display metadata such as
local-codex-profile, not an account identity and not a secret.

## Policy behavior

The default policy requires both FIVE_HOUR and WEEKLY windows and projects the
minimum remaining fraction across all usable windows. A missing required window
produces UNKNOWN; the policy never treats a partial high-capacity report as a
complete observation.

| State | Default minimum remaining | New expensive work | New speculative work |
|---|---:|---:|---:|
| ABUNDANT | 0.50 | yes, bounded | yes, bounded |
| NORMAL | 0.25 | yes, bounded | no |
| CONSERVE | 0.10 | no | no |
| CRITICAL | below 0.10 | no | no |
| UNKNOWN | no trustworthy current value | no | no |

Every state permits already active work to reach safe receipt custody. This is
not automatic retry and does not authorize target effects. A reset time that
has arrived requires a fresh observation; the policy does not invent restored
capacity.

Timers and services are outside this campaign and remain disabled. A future
foreman consumes only the normalized record and exact decision, never the
provider-specific App Server response shape.
