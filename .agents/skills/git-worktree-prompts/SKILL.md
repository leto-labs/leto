---
name: git-worktree-prompts
description: Create sibling git worktrees and seed each one with a root `prompt.txt` handoff for parallel agent work. Use when the user wants to split work across multiple branches, isolate features or refactors in separate worktrees, or start agents with a one-shot `read prompt.txt` workflow.
---

# Git Worktree Prompts

Use this skill when the user wants parallel implementation lanes with isolated branches and a root `prompt.txt` in each worktree. The skill helps you inspect the active repo, choose disjoint tracks, create sibling worktrees, seed prompt handoff files, and verify the layout.

## When To Use It

- The user wants to work on multiple features concurrently.
- The user asks for sibling git worktrees or branch-isolated agent tasks.
- The user wants each agent to start from `read prompt.txt`.
- You need to hand off tightly scoped work with file ownership and verification commands.

## Workflow

### 1. Inspect the active repo first

Run:

```bash
git rev-parse --show-toplevel
git branch --show-current
git worktree list
git status --short
```

Treat the active worktree root as the project root. Prefer sibling worktrees next to that root.

### 2. Choose cleanly separated tracks

- Split by crate, subsystem, or planning lane.
- Avoid assigning overlapping write scopes to different worktrees.
- Keep planning-only or spec-only work separate from implementation branches.
- Give each track a short worktree directory name and branch name.

### 3. Create each worktree and seed `prompt.txt`

Use the helper script in this skill:

```bash
cat <<'EOF' | python3 .agents/skills/git-worktree-prompts/scripts/create_worktree_with_prompt.py \
  --name runtime-worktrees \
  --branch feature/runtime-worktrees \
  --prompt-stdin
Read this file once, follow it, then delete this file before making other edits.

Worktree: /abs/path/runtime-worktrees
Branch: feature/runtime-worktrees

Goal
Implement the runtime-managed git worktree feature.

Read first
- path/to/spec.md
- path/to/tasks.md

Scope
- Own only specific crates or directories.
- Do not edit overlapping modules owned by other worktrees.

Deliverables
- Implementation or planning outcomes.

Verification
- Specific tests or validation commands.
EOF
```

Defaults:

- The script creates a sibling worktree at `<repo-parent>/<name>`.
- The prompt file is written as `<worktree>/prompt.txt`.
- The branch is created from the current `HEAD` unless you pass `--base-ref`.

If the branch already exists and should be checked out instead of created, use `--checkout-existing`.

### 4. Keep prompt handoffs crisp

Each `prompt.txt` should contain:

- A one-time instruction to delete the file after reading.
- Worktree path and branch.
- A concrete goal.
- A short `Read first` list.
- Explicit scope and non-goals.
- Deliverables.
- Verification commands.

Do not write generic prompts. Assign ownership so two agents do not edit the same subsystem.

### 5. Verify before handing off

Run:

```bash
git worktree list
sed -n '1,80p' /path/to/worktree/prompt.txt
```

Confirm that:

- The worktree path is correct.
- The branch is correct.
- `prompt.txt` exists at the worktree root.
- The prompt names the right files, specs, and validation commands.

## Script

Use `scripts/create_worktree_with_prompt.py` for deterministic setup.

Key options:

- `--name`: Worktree directory name under the repo parent.
- `--branch`: Branch to create or check out.
- `--base-ref`: Base ref for a new branch. Defaults to `HEAD`.
- `--checkout-existing`: Attach an existing branch instead of creating a new one.
- `--prompt-file`: Read prompt contents from a file.
- `--prompt-stdin`: Read prompt contents from stdin.
- `--prompt-name`: Prompt filename. Defaults to `prompt.txt`.
- `--force-prompt`: Overwrite an existing prompt file in the new worktree.

When possible, use stdin with a heredoc so the prompt content is visible in the command history and easy to review.
