# GAP: MCP Call Authority Classes

> Status: identified, partially specified

## The rule

MCP is capability discovery and tool transport. Tool availability is not
permission. All promoted actions pass through Governor.

But not every MCP call needs Governor. Making Governor a bottleneck for
`list_tools` or `read_resource` would be operationally miserable and
substantively pointless. The discrimination is by **call class**.

## Call classes

```text
discover   list tools/resources                    local policy may allow
read       fetch state, no mutation                local policy may allow
propose    produce candidate action (no-op)        local policy may allow
stage      prepare reversible mutation             requires Governor
mutate     change state                            requires Governor
publish    expose artifact to external audience    requires Governor
page       wake human                              requires Governor (receipt required)
```

## Class semantics

### discover

Listing tools, resources, prompts. Pure capability introspection. No
state change, no side effects observable outside the process.

Examples:
- `tools/list`
- `resources/list`
- `prompts/list`

Local policy may allow without Governor consultation.

### read

Fetching state from an external system without mutation. The fetch
itself may have side effects on the target (access logs, rate limiting),
but no observable semantic change.

Examples:
- HTTP GET to NQ `/api/findings`
- `resources/read` for a file
- SQLite SELECT

Local policy may allow. Governor may still require receipt emission for
audit (`record_receipt`), but not gating.

### propose

Generating candidate actions or text. LLM calls that produce output
without executing. No external state change.

Examples:
- Model inference
- Rendering a diff candidate
- Composing a repair proposal

Local policy may allow. These are the bulk of night-shift work and
gating them per-call would be absurd.

### stage

Preparing a mutation that is reversible and not yet applied. Writing a
patch file, creating a draft PR, staging a SQL statement.

Examples:
- Writing `.patch` to disk
- Creating a git branch (non-destructive)
- Staging a SQL statement for review

**Requires Governor.** Even reversible preparation is a step up the
authority ladder.

### mutate

Changing state on an external system. Applying a patch, running
`systemctl restart`, issuing a DB write, overwriting a file.

Examples:
- `git commit --apply`
- `systemctl restart`
- Filesystem writes outside scratch areas
- HTTP POST/PUT/DELETE to production APIs

**Requires Governor.** This is the authority boundary in its literal
form.

### publish

Making an artifact visible to an external audience. Pushing to a
remote, deploying a static site update, posting to a public channel.

Examples:
- `git push`
- Deploying Grid Dependency Atlas update
- Posting to Slack/email/webhook

**Requires Governor.** Publication has a social blast radius different
from internal mutation.

### page

Waking a human. Calls that result in a notification intended to demand
attention (not just an advisory packet).

Examples:
- PagerDuty / Opsgenie
- SMS / phone
- Priority email tagged for on-call

**Requires Governor.** A page is a coarse, high-cost interrupt. It
should always leave a receipt.

## Classification responsibility

- The caller (Night Shift / Python workflow) classifies the intended
  call.
- The adapter layer validates the classification against a known list of
  tool IDs (via capability metadata or curated list).
- Tool IDs that cannot be classified default to `mutate` (fail-closed).

## Invariants

- A Python workflow may never declare its own calls as `discover | read |
  propose` for tools that have a registered class of `stage+`.
- Governor receipt is required for every `stage+` call, regardless of
  verdict (allow/deny both recorded).
- A single agenda may not mix `propose` output with `apply` effects
  without a Governor-authorized `authorize_transition` in between.

## Open questions

- Where does the tool-ID → call-class mapping live? (Probably a curated
  YAML in Night Shift, extensible via agenda overrides with Governor
  sign-off.)
- How do we handle tools that span classes (e.g., a "smart" tool that
  does both read and mutate depending on args)? (Classify at the
  argument level, or split into separate tools. Prefer splitting.)
- Does `page` need its own authority level in the escalation ladder, or
  is it a side effect of `escalate`? (Probably both — escalate is the
  run's authority level; page is the MCP call that implements it.)
