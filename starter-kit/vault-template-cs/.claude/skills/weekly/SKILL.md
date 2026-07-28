---
name: weekly
description: Cross-project weekly digest printed to terminal. Use for "/weekly" or when the user asks what happened this week.
---

# /weekly

Read-only — writes nothing, commits nothing.

1. `git log --since='1 week ago' --name-only --pretty=format:'%ad %s' --date=short` — group changes by top folder/project. Ignore `autosync` commit messages themselves; use their file lists.
2. Open tasks: grep `- [ ]` in `projekty/*/ukoly.md`.
3. New meetings this week: files added under `projekty/*/schuzky/`.
4. Inbox state: count of unsorted files in `inbox/`.
5. Print a Czech digest: per-project bullet summary, then open-tasks table, then inbox note.
