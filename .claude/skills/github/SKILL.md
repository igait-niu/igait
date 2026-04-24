---
name: github
description: >
  Use this skill for ANY interaction with GitHub on `igait-niu/igait` — issues,
  PRs, or branches. This includes: creating, reading, updating, closing, or
  commenting on issues; creating, reviewing, or merging PRs; managing branches;
  searching issues; and starting work on an issue. Triggers on: "#N" (any
  issue number reference), "start #N", "work on #N", "pick up #N", "begin #N",
  "what's #N", "new issue", "create issue", "plan feature", "open a ticket",
  "close issue", "check PR", "what's the status of #N", "add a comment",
  "assign to", "create a branch", "link PR", "open PR", "merge PR", "review PR",
  or any reference to GitHub issues or PRs. Also triggers when the user
  describes a problem, idea, or change that should become a tracked work item —
  even casually phrased. When in doubt, trigger this skill.
---

# iGait GitHub

The single skill for all GitHub interactions on `igait-niu/igait`. Covers issue
creation, issue management, PR workflows, and branch management. iGait is a
web-based autism screening tool with a multi-stage gait analysis pipeline —
issues often involve pipeline stages, backend routes, frontend UI,
infrastructure/deployment, or ML model work.

## Core Philosophy

For **issue creation**: have a conversation, not an intake form. Cover everything,
feel like nothing.

For **everything else** (updates, comments, PR ops): be fast and direct. The user
wants something done — do it, confirm it, move on.

## Team Reference

| Handle | Role / Notes |
|--------|-------------|
| `@hiibolt` | Lead developer, primary implementer |
| `@Prashanna-Raj-Pandit` | Team member (ML/prediction work) |
| `@zworie` | Team member |
| `@shaivilp` | Team member |
| `@harshithamaartha02-png` | Team member |
| `@claude` | You — review, scaffolding, implementation, suggestions |

Repo: `igait-niu/igait`
Org: `igait-niu`

## Labels (pre-fetched)

Use these label names directly with `--label` flags.

| Name | ID | Color | Description |
|------|----|-------|-------------|
| `bug` | `LA_kwDOLBPL888AAAABfUNrlQ` | `d73a4a` | Something isn't working |
| `documentation` | `LA_kwDOLBPL888AAAABfUNrnA` | `0075ca` | Improvements or additions to documentation |
| `duplicate` | `LA_kwDOLBPL888AAAABfUNrnw` | `cfd3d7` | This issue or pull request already exists |
| `enhancement` | `LA_kwDOLBPL888AAAABfUNrpA` | `a2eeef` | New feature or request |
| `help wanted` | `LA_kwDOLBPL888AAAABfUNrqA` | `008672` | Extra attention is needed |
| `good first issue` | `LA_kwDOLBPL888AAAABfUNrqg` | `7057ff` | Good for newcomers |
| `invalid` | `LA_kwDOLBPL888AAAABfUNrrQ` | `e4e669` | This doesn't seem right |
| `question` | `LA_kwDOLBPL888AAAABfUNrsA` | `d876e3` | Further information is requested |
| `wontfix` | `LA_kwDOLBPL888AAAABfUNrsw` | `ffffff` | This will not be worked on |
| `dependencies` | `LA_kwDOLBPL888AAAABlNVvRg` | `0366d6` | Pull requests that update a dependency file |
| `rust` | `LA_kwDOLBPL888AAAAB9Pkkyw` | `000000` | Pull requests that update rust code |
| `javascript` | `LA_kwDOLBPL888AAAACYwQrtQ` | `168700` | Pull requests that update javascript code |

## Session Memory

To avoid redundant API calls within a single conversation:

- **Labels**: Always use the hardcoded Labels table above. Do NOT run `gh label list`.

---

## CLI Tools Reference

All operations use the `gh` CLI (pre-installed). Run commands via the Bash tool.

### Issue Operations

| Operation | Command |
|-----------|---------|
| Create issue | `gh issue create --repo igait-niu/igait --title "..." --body "..." --label "enhancement" --assignee hiibolt` |
| Get issue | `gh issue view <N> --repo igait-niu/igait` |
| Get issue (JSON) | `gh issue view <N> --repo igait-niu/igait --json title,body,state,labels,assignees,comments` |
| Update issue | `gh issue edit <N> --repo igait-niu/igait [--title "..." --body "..." --add-label "..." --add-assignee "..."]` |
| Close issue | `gh issue close <N> --repo igait-niu/igait` |
| Comment on issue | `gh issue comment <N> --repo igait-niu/igait --body "..."` |
| Search issues | `gh search issues --repo igait-niu/igait "<query>"` |
| List open issues | `gh issue list --repo igait-niu/igait --state open --json number,title,labels,assignees` |

### Branch & PR Operations

| Operation | Command |
|-----------|---------|
| Create branch | `git checkout -b <branch-name> main && git push -u origin <branch-name>` |
| Create PR | `gh pr create --repo igait-niu/igait --title "..." --body "..." --base main --head <branch>` |
| Get PR | `gh pr view <N> --repo igait-niu/igait` |
| Get PR (JSON) | `gh pr view <N> --repo igait-niu/igait --json title,body,state,files,reviews,comments,statusCheckRollup` |
| PR changed files | `gh pr diff <N> --repo igait-niu/igait` |
| PR status/checks | `gh pr checks <N> --repo igait-niu/igait` |
| Review PR | `gh pr review <N> --repo igait-niu/igait --approve --body "..."` (or `--request-changes` / `--comment`) |
| Merge PR | `gh pr merge <N> --repo igait-niu/igait [--squash\|--merge\|--rebase]` |
| List open PRs | `gh pr list --repo igait-niu/igait --state open --json number,title,headRefName,author,createdAt` |
| Update PR branch | `gh pr update-branch <N> --repo igait-niu/igait` |

---

## Routing

Determine which workflow to use based on the user's request:

| Request type | Workflow |
|-------------|----------|
| Create a new issue / plan a feature / report a bug | **Issue Creation** (Phases 0-3 below) |
| Everything else (update, comment, close, branch, PR, search) | **Quick Actions** (see below) |

---

## Quick Actions

For non-creation operations, skip the interview. Just do the thing and confirm.

### Issue Operations
- **Read issue**: `gh issue view` — show title, body, state, assignees, labels
- **Update issue**: `gh issue edit` with the relevant flags
- **Close issue**: `gh issue close`
- **Comment on issue**: `gh issue comment` — post the comment, confirm with link
- **Search issues**: `gh search issues --repo igait-niu/igait`

### Starting work on an issue
When the user says "start #N" / "work on #N" / "pick up #N":
1. **Assign the issue**: `gh issue edit <N> --repo igait-niu/igait --add-assignee hiibolt` (if not already assigned).
2. **Create a feature branch**: `git checkout -b <issue-number>-<short-description> main && git push -u origin <branch>`. Skip if a branch for this issue already exists.
3. **Post a kickoff comment**: `gh issue comment <N> --repo igait-niu/igait --body '**[Started]** Working on branch \`<branch-name>\`'`
4. **Confirm** with: the assignee and branch name so the user can `git fetch && git checkout` immediately.

### Branch & PR Operations
- **Create branch for issue**: use pattern `<issue-number>-<short-description>`. All issue work must happen on a feature branch, never directly on `main`.
- **Create PR**: `gh pr create` — set title, body, head branch, base branch (`main`). Always include `Closes #<issue-number>` in the PR body so GitHub auto-links and auto-closes the issue on merge.
- **Check PR status**: `gh pr view` or `gh pr checks`
- **Review PR**: `gh pr diff` to see changes, then `gh pr review`
- **Merge PR**: `gh pr merge` — confirm merge method with user first.
- **List open PRs**: `gh pr list --state open`

For all quick actions: execute, confirm the result with a link or summary, done.
If any call fails, report the error clearly.

---

## Live Issue Updates

When actively working on an issue and writing code, keep the issue thread alive
as a development log using `gh issue comment`. This gives visibility without
anyone having to ask "how's it going?"

### When to Comment

| Trigger | What to post |
|---------|-------------|
| **Starting work** | Branch name, initial approach/plan, files you expect to touch |
| **Major milestone reached** | What was completed, what's next |
| **Approach change** | Why the original plan didn't work, what you're doing instead |
| **Blocker hit** | What's blocking, what you've tried, whether you need input |
| **PR opened** | Link to the PR, brief summary of what's included |
| **Work complete** | Final summary: what was done, any follow-up items or tech debt noted |

### Tone & Format

Keep comments **short and scannable**. Use this rough format:

```
**[Status]** Brief headline

- Bullet points with details
- Keep to 2-4 bullets max
```

Status tags: `[Started]`, `[Progress]`, `[Milestone]`, `[Pivot]`, `[Blocked]`, `[PR Ready]`, `[Done]`

**Do not** narrate every file edit or minor refactor. Comment at meaningful boundaries —
when something is *done*, *changed*, or *stuck*.

### Commit Cadence

While working on an issue, **commit frequently** — don't let large amounts of work
accumulate uncommitted. Follow these guidelines:

- **Commit after each logical unit of work**: a new route, a completed stage change,
  a test passing. Roughly every 15-30 minutes of active coding.
- **Commit before pivoting**: if you're about to change approach or move to a different
  part of the task, commit what you have first.
- **Commit before anything risky**: about to refactor something that might break? Commit
  the working state first.
- **Use descriptive commit messages** that reference the issue number (e.g. `#51: update
  prediction output to binary ASD/NO-ASD`).
- **Never go more than ~30 minutes of coding without a commit.**

Proactively ask the user if they'd like to commit when you've completed a meaningful
chunk of work — don't wait for them to remember.

---

## Issue Creation Workflow

### Phase 0 — Read the Room (silent)

Before speaking, do these steps:

1. **Classify** the issue as **feature**, **bug**, or **chore/refactor**.

2. **Gauge readiness**: does the user already have a fully-formed spec, or are they
   still figuring it out?
   - **Fast-track**: If they arrive with a clear goal, acceptance criteria, and enough
     detail — skip the interview. Go straight to Phase 2, draft the issue, and confirm.
   - **Standard**: If they're still thinking, start the conversation (Phase 1).

3. **Scan for duplicates**:
   - `gh search issues --repo igait-niu/igait "<keywords>"`
   - If likely duplicates exist, mention them conversationally before proceeding

Then begin. Don't announce the phases. Just talk.

### Phase 1 — The Interview

Your goal is to draw out a fully specified work item through natural dialogue.
Internally track what you still need — but never recite the list.

**Required for all issue types:**
- One-sentence goal
- Acceptance criteria (checkboxes — how do we know it's done?)
- At least one test case (happy path + one edge case)

**Additionally required by type:**

| Feature | Bug | Chore |
|---------|-----|-------|
| Why now / motivation | Steps to reproduce | What & why |
| Concrete deliverables | Expected vs actual behavior | Approach / plan |
| Out of scope | Environment / context | Behavior changes (if any) |

**Nice-to-have (capture if mentioned, don't force):**
- Dependencies or risks
- Related issues
- Which pipeline stage(s) or service(s) affected

**iGait-specific considerations:**
- Does this affect a pipeline stage? Which one(s)?
- Does this change the Firebase RTDB queue schema?
- Does this require changes to igait-lib (shared by all services)?
- Does this affect the Docker build or K8s deployment?
- Does this involve ML model changes (stages 2, 4, 5, 6)?
- Does this require credential or environment variable changes?

**Tone:**
- Make creative suggestions — "What about the edge case where the video has no walking frames?"
- Notice what's missing and ask sideways — "So if stage 4 fails on this input, does stage 7 report it correctly?"
- Offer your opinion where warranted, but keep it technical and productive.

**Exit condition:** When you have a complete picture, present a draft summary and ask:
*"Does this capture it? Anything to add or change before I make it real?"*

### Phase 2 — Structured Draft

Format the issue body with clear sections. Show the user:
- The title (using conventional commit prefix: `feat:`, `bugfix:`, `chore:`)
- The full body with sections: Goal, Acceptance Criteria, Test Cases, Notes

One final confirmation: *"Look good?"*

### Phase 3 — Act

Once confirmed, create the issue:

```bash
gh issue create --repo igait-niu/igait \
  --title "<from Phase 2>" \
  --body "<from Phase 2>" \
  --label "<matching type: enhancement, bug, etc.>" \
  --assignee "<agreed assignees>"
```

Report to the user:
- Issue URL
- One-line summary

If the call fails, report the error clearly and offer to retry.

---

## What Good Looks Like

**For issue creation:** A great issue should be passable directly to someone with
zero iGait context and they should know exactly what to build, how to test it, and
when it's done. If it couldn't do that, go back to Phase 1.

**For quick actions:** Fast, correct, confirmed. Execute the operation, report the
result with a link, move on. No ceremony needed.
