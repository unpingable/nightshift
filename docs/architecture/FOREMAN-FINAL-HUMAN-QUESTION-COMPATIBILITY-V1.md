# Foreman final human-question compatibility V1

Campaign: QUESTION-RIVET

Canonical slug: \`nightshift-foreman-final-human-question-casework-compatibility-v1\`

The foreman's exact final \`nightshift.run-receipts/v1\` projection uses the
sealed renderer/Casework human-question vocabulary:

- \`work_item\`
- \`exact_question\`
- \`evidence_exhausted\`
- \`safe_default\`
- \`consequences\`
- \`resume_point\`

A worker terminal or not-started receipt continues to retain its closed
\`nightshift.worker-*/v1\` \`HumanQuestionV1\`, including \`question_id\`, in the
exact raw receipt. The final snapshot projects its \`question\` text to the
existing \`exact_question\` field and its work-item binding to \`work_item\`.
It does not revise the sealed run-receipts contract, mint authority, or promote
question text into scheduler or campaign semantics.

The historical defect emitted \`question\` instead of \`exact_question\`.
Casework and the report renderer correctly refused that incompatible derived
snapshot. QUESTION-RIVET changes only this projection key for newly generated
snapshots. Already retained historical final snapshot bytes are immutable.

Qualification must cover terminal and not-started question sources, exact raw
receipt retention, all six projected fields, sealed Casework loading, report
rendering, and deterministic refusal of the historical substituted key. No
approval response, target effect, aggregate result, retry, provider session, or
write control is introduced.
