---
name: docs
description: >
  Audit and update project documentation after code changes — CLAUDE.md, skills,
  and memory. Use after any session that changed the pipeline stages, backend routes,
  frontend routes, build/deploy setup, or igait-lib interfaces. Also use when the user
  says "update docs", "sync docs", "are the docs up to date", or "refresh documentation".
  Can be invoked proactively at the end of a work session when significant structural changes
  were made.
---

# Docs Sync

Audit project documentation for staleness after code changes. The goal is to keep
docs accurate without the user having to remember which files to update.

## What to audit

Check each of these sources against the current codebase. Only update what's actually
stale — don't rewrite things that are still accurate.

### 1. Root CLAUDE.md

**File:** `/CLAUDE.md`

Check these sections against reality:
- **Project Overview** — Does the description still match?
- **Architecture** — Are all stages listed and described correctly? Any new services or removed stages?
- **Build & Run Commands** — Are all build/test/lint commands listed and correct? Did ports, tools, or prerequisites change?
- **CI/CD** — Are the workflow triggers and image registries still accurate?
- **Key Patterns** — Do the stated patterns (StageWorker trait, queue schema, submodules, auth) still hold?

How to check: Read CLAUDE.md, then glob for key files it references. If a referenced file
doesn't exist or a described pattern no longer applies, update the section.

### 2. igait-lib Public Interface

**Files:**
- `igait-lib/src/lib.rs`
- `igait-lib/src/microservice/worker.rs`
- `igait-lib/src/microservice/queue.rs`
- `igait-lib/src/microservice/types.rs`
- `igait-lib/Cargo.toml`

Check whether CLAUDE.md's description of the StageWorker trait, feature flags (`microservice`,
`email`), and key modules still matches reality. If exports, trait signatures, or features
changed, update the CLAUDE.md Architecture section.

### 3. Backend Routes

**Directory:** `igait-backend/src/routes/`

If routes were added, removed, or renamed, verify the CLAUDE.md Architecture section still
accurately describes the backend's responsibilities.

### 4. Frontend Routes

**Directory:** `igait-frontend/src/routes/`

If SvelteKit routes were added, removed, or restructured (especially under `(authenticated)/`),
verify CLAUDE.md still reflects the frontend's route layout.

### 5. Pipeline Stages

**Directory:** `igait-stages/`

If stages were added, removed, or had their purpose changed, verify the stage listing in
CLAUDE.md and the Firebase RTDB queue schema description are still accurate. Cross-check
with `database.rules.json` for queue entry schemas.

### 6. Skills

**Directory:** `.claude/commands/skills/*/SKILL.md`

For each skill, check whether the patterns it describes still match the code:

| Skill | What to verify |
|-------|---------------|
| `docs` | This skill — should it be auditing other areas too? |

Only read skills that are likely affected by the changes made in this session.

### 7. Memory

**File:** `/home/hiibolt/.claude/projects/-home-hiibolt-Development-igait/memory/MEMORY.md`

Check if any memory entries reference things that no longer exist (deleted files,
renamed patterns, old architecture). Update or remove stale entries.

## How to run

1. **Identify what changed** — Look at the git diff or the conversation history to
   understand what was modified in this session.
2. **Scope the audit** — Only check docs that could be affected. A frontend-only change
   doesn't need the pipeline stages or igait-lib audited. A new stage doesn't need
   frontend routes checked.
3. **Read and compare** — For each doc in scope, read it and verify against the code.
4. **Fix silently** — Update stale sections without asking. Only flag things to the
   user if there's ambiguity (e.g., a doc describes a pattern and you're not sure
   if the old or new way is intended).
5. **Report** — Output a short summary of what was updated and what was already current.

## Output format

```
## Docs Audit

| Document | Status |
|----------|--------|
| CLAUDE.md | current |
| igait-lib interface | updated — added new feature flag `notifications` |
| backend routes | current |
| frontend routes | updated — added /contribute/review route |
| pipeline stages | current |
| docs skill | current |
| memory | current |

N files updated, M already current.
```

Keep it brief. The user wants confidence that docs are synced, not a detailed changelog.
