# Review Process Topic

Status: current workflow contract.

Edict treats pull request review as part of claim integrity. A merge decision
must not silently convert a review-tool outage into approval. The structured
policy lives in [`policy.toml`](./policy.toml). [REVIEW-REQ-001]

## Bot Review Policy

CodeRabbit is the primary review bot. When CodeRabbit is actively reviewing, its
approval is required before merge. Actionable CodeRabbit feedback is handled the
same way as other review feedback: verify the finding, fix valid issues with
RED/GREEN evidence when behavior changes, and resolve the review thread only
after the fix lands. [REVIEW-REQ-001]

When CodeRabbit cannot review because it is rate limited, out of credits, or
reports insufficient usage credits, the reviewer must request an alternate bot
review by posting:

```text
@codex review please
```

The reviewer then waits for the alternate review response before treating the
pull request as merge-ready. [REVIEW-REQ-002] [REVIEW-REQ-003]

The goal is at least one automated or human reviewer beyond the author. A credit
or rate-limit failure is not approval. If no alternate bot responds, the pull
request remains blocked until an explicit maintainer decision overrides the
review-bot gate. [REVIEW-REQ-004]

## Merge Readiness

Before merge:

- required CI is green;
- there are zero unresolved blocking review threads;
- CodeRabbit has approved, or CodeRabbit is unavailable and the alternate bot
  review request has received a response;
- no required review is converted from unavailable to approved without a
  recorded maintainer decision;
- local release or behavior gates have passed for the change type.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
