# Human questions

## 1. switchyard-transport

- Exact question: What is the authoritative private remote/destination and canonical branch policy for /data/git/switchyard?
- Evidence exhausted: The configured origin is an absent local bundle, ls-remote fails, and Cartography has no Switchyard repository row.
- Safe default: Do not invent a remote; retain the clean local commits and leave the service disabled.
- Consequences: Interlock and QUIET-BRIDGE remain sole-local until a registered destination exists.
- Resume point: Verify repository identity and branch policy, configure the registered remote, push exact commits, and verify SHAs.

## 2. worker-vm-custody

- Exact question: What is the authoritative private remote for campaign-driver-ng?
- Evidence exhausted: No authoritative remote is configured; accepted head ff2b6562b3a19c9f5c1b669109ef3c836c614da4 is clean and sole-local.
- Safe default: Retain the exact local head and do not invent a destination.
- Consequences: Porter and AG are published but the composed repair retains one sole-local repository.
- Resume point: Register the repository destination, push the exact campaign branch without rewriting, and verify SHA.

## 3. copper-artifact-custody

- Exact question: Is git@github-unpingable:unpingable/civild.git the authorized publication destination for FORGE-VAULT and TWIN-HARBOR?
- Evidence exhausted: The remote is configured, but environment review did not establish ownership/trust and blocked publication before remote mutation.
- Safe default: Keep both clean exact heads local and do not retry or work around the block.
- Consequences: The qualified bundle and blocked entry record remain sole-local.
- Resume point: Confirm ownership and branch policy, then push 0f70fd18ca94996e8d56798c34f15acd69913999 and 21c33de9d9106e9f59663c4cbfdbedf22f1707c5 exactly.

## 4. copper-deployment

- Exact question: Which existing disposable FreeBSD 15.1 reference fixture is authorized for TWIN-HARBOR, or is distinct fixture creation separately authorized?
- Evidence exhausted: No exact-work reference, repository fixture record, libvirt domain, or authorized overlay identifies a target; KVM availability is not authority.
- Safe default: Do not create or repurpose infrastructure; keep TWIN-HARBOR not started.
- Consequences: Install, boot, runtime, reboot, removal, and reinstall qualification cannot begin.
- Resume point: Record exact fixture identity and authority, rerun entry, then mint a fresh deployment occurrence.

## 5. lanternwake-port

- Exact question: Which separate successor-NQ live route is authorized for AMBER-COMPASS?
- Evidence exhausted: The local identity boundary qualifies, but no separate live route is recorded and GLASSHOPPER is forbidden.
- Safe default: Keep live-route status unqualified and do not touch Classic or GLASSHOPPER.
- Consequences: The port remains qualified locally without a live qualification claim.
- Resume point: Provide exact route and fixture authority, then run a distinct live occurrence.

## 6. bedrock-docket-executor

- Exact question: May a separate successor campaign design a versioned NQ prepared-occurrence contract with an explicit Docket executor-plan binding?
- Evidence exhausted: Docket V1 binds AG/dispatch work to its content-bound executor plan, while NQ V1 binds AG work to PreparedOccurrenceV1.plan_id; equality would require circularity or misrepresentation.
- Safe default: Preserve both V1 contracts, keep the composed prerequisite not qualified, and do not start OPEN-QUARRY.
- Consequences: A versioned successor can represent both identities; weakening V1 would violate exactness law.
- Resume point: Authorize and scope a distinct NQ contract successor, then independently requalify the composed adapter before any live occurrence.
