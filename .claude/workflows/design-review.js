export const meta = {
  name: 'design-review',
  description: 'Draft an OpenSpec change from a brief, review it on eight criteria, and revise it at most twice',
  whenToUse: 'Run by the /design skill with args {change, brief}. Not for direct use.',
  phases: [
    { title: 'Draft', detail: 'one writer follows the OpenSpec propose skill' },
    { title: 'Review', detail: 'eight reviewers, one criterion each' },
    { title: 'Revise', detail: 'one writer fixes or rejects each finding' },
  ],
}

const REVIEWERS = [
  'cold-read',
  'limits',
  'executable-spec',
  'innovation-tokens',
  'abstractions',
  'standalone',
  'root-cause',
  'performance',
]
const MAX_REVISIONS = 2
const { change, brief } = args
const dir = `openspec/changes/${change}`

const WRITE_RESULT = {
  type: 'object',
  required: ['validate_ok', 'summary'],
  properties: {
    validate_ok: { type: 'boolean', description: 'openspec validate --strict passed' },
    summary: { type: 'string', description: 'two sentences on what was written or changed' },
  },
}

const VERDICT = {
  type: 'object',
  required: ['pass', 'findings'],
  properties: {
    pass: { type: 'boolean', description: 'true when there is no blocking finding' },
    explanation: {
      type: 'string',
      description: 'cold-read only: the reviewer’s own explanation of the design from pass 1',
    },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['file', 'section', 'problem', 'fix', 'severity'],
        properties: {
          file: { type: 'string' },
          section: { type: 'string' },
          problem: { type: 'string' },
          fix: { type: 'string' },
          severity: { type: 'string', enum: ['blocking', 'minor'] },
        },
      },
    },
  },
}

const validate = `Before you finish, run \`openspec validate ${change} --strict\` and fix every error.`

phase('Draft')
const draft = await agent(
  `Write the OpenSpec change "${change}" for this brief.

${brief}

Follow .claude/skills/openspec-propose/SKILL.md, with these differences:
- Nobody can answer questions during this run. For each ambiguity, make a reasonable assumption,
  record it in ${dir}/review.md under "## Open questions", and continue.
- Write design.md in the shape that its instruction and template give. When
  openspec/changes/archive/ holds a design.md, read the newest one first: it is a finished design
  in that shape.
- Edit nothing outside ${dir}. Do not commit.
- Create ${dir}/review.md with two headings, "## Open questions" and "## Review log".

${validate}`,
  { label: 'writer', phase: 'Draft', schema: WRITE_RESULT },
)
if (!draft) throw new Error(`The writer did not finish, so ${dir} may be incomplete.`)

const review = (reviewer, round) =>
  agent(
    `Review the OpenSpec change in ${dir}. Read .claude/skills/design/reviewers/${reviewer}.md and
judge only that criterion, in the reading order it gives. Read the "Review log" in
${dir}/review.md when the rubric allows it. Do not repeat a finding the writer rejected, unless
the stated reason is wrong. In that case, say why it is wrong. Edit no files. This is review
round ${round}.`,
    { label: `${reviewer} · round ${round}`, phase: 'Review', schema: VERDICT },
  ).then((verdict) => verdict && { ...verdict, reviewer })

const revise = (findings, explanation, round) =>
  agent(
    `Revise the OpenSpec change in ${dir} using these review findings:

${JSON.stringify(findings, null, 2)}
${explanation ? `\nA cold reader explained the design back like this. Where it is wrong or thin, the reader sections misled them:\n\n${explanation}\n` : ''}
For each finding, either fix the artifact or reject the finding with a reason. Reviewers can be
wrong. Reject a finding that conflicts with the brief, with the code, or with another finding.
Fix content where it is. Keep design.md in the shape its template gives, and keep file paths and
code identifiers in "Where in the code". Keep proposal.md under one page.

In ${dir}/review.md, under "## Review log", add "### Round ${round}" with one line per finding:
the reviewer, the problem in one sentence, and "fixed" or "rejected: <reason>".

${validate}`,
    { label: `writer · revision ${round}`, phase: 'Revise', schema: WRITE_RESULT },
  )

const rounds = []
let pending = REVIEWERS
let latest = draft
let revisions = 0
let unchecked = { reviewers: [], fixes: 0 }
let explanation = null

while (pending.length) {
  const round = revisions + 1
  const verdicts = await parallel(pending.map((reviewer) => () => review(reviewer, round)))
  for (const reviewer of pending) {
    const v = verdicts.find((verdict) => verdict && verdict.reviewer === reviewer)
    rounds.push(
      v
        ? {
            round,
            reviewer,
            pass: v.pass,
            blocking: v.findings.filter((f) => f.severity === 'blocking').length,
            minor: v.findings.filter((f) => f.severity === 'minor').length,
          }
        : { round, reviewer, pass: null, blocking: 0, minor: 0 },
    )
  }
  const missing = rounds.filter((r) => r.round === round && r.pass === null).map((r) => r.reviewer)
  if (missing.length) log(`No verdict from: ${missing.join(', ')}. Their criteria are unchecked this round.`)

  const coldRead = verdicts.find((v) => v && v.reviewer === 'cold-read')
  if (coldRead && coldRead.explanation) explanation = coldRead.explanation

  const findings = verdicts
    .filter(Boolean)
    .flatMap((v) => v.findings.map((finding) => ({ reviewer: v.reviewer, ...finding })))
  if (!findings.length) break

  latest = (await revise(findings, coldRead && coldRead.explanation, round)) || latest
  revisions = round
  // Every reviewer whose findings were just answered checks the fixes in the next round.
  pending = [...new Set(findings.map((f) => f.reviewer))]
  if (revisions === MAX_REVISIONS) {
    unchecked = {
      reviewers: pending,
      fixes: findings.length,
      blocking: [...new Set(findings.filter((f) => f.severity === 'blocking').map((f) => f.reviewer))],
    }
    break
  }
}

// After the last revision, nobody has read the final text. A final pass, with no revision after
// it, re-checks each reviewer whose blocking finding that revision answered, and re-runs the cold
// read so its explanation describes the design the person will read.
let finalCheck = null
if (unchecked.fixes) {
  const finalReviewers = [...new Set([...unchecked.blocking, 'cold-read'])]
  const verdicts = await parallel(
    finalReviewers.map((reviewer) => () => review(reviewer, MAX_REVISIONS + 1)),
  )
  const coldRead = verdicts.find((v) => v && v.reviewer === 'cold-read')
  if (coldRead && coldRead.explanation) explanation = coldRead.explanation
  finalCheck = finalReviewers.map((reviewer) => {
    const v = verdicts.find((verdict) => verdict && verdict.reviewer === reviewer)
    return v
      ? { reviewer, pass: v.pass, findings: v.findings }
      : { reviewer, pass: null, findings: [] }
  })
  const failing = finalCheck.filter((c) => c.pass === false).map((c) => c.reviewer)
  if (failing.length) log(`Still blocking after the last revision: ${failing.join(', ')}. See final_check.`)
  log(
    `Revision ${MAX_REVISIONS} answered ${unchecked.fixes} findings. The final pass re-checked ${finalReviewers.join(', ')}. Minor fixes from the other reviewers were not re-checked.`,
  )
}

return {
  change,
  dir,
  revisions,
  validate_ok: latest.validate_ok,
  rounds,
  unchecked,
  final_check: finalCheck,
  cold_read_explanation: explanation,
  summary: latest.summary,
}
