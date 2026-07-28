---
name: inbox-triage
description: Sort everything in inbox/ into the right vault folder. Use when the user asks to clean/triage/sort the inbox.
---

# /inbox-triage

Folder names below are the defaults from `CLAUDE.md` — follow that file if this
vault renamed them.

For each `.md` file in `inbox/` (skip `README.md`):

1. Read it. Decide the destination:
   - mentions a project (see `projects/` folder names, or grep project names) → that project's folder: task-like content appended to `tasks.md` as `- [ ]`, meeting content to `meetings/YYYY-MM-DD-title.md`, else appended under `## General notes` in the project `README.md` or kept as a standalone file in the project folder.
   - about a person → `people/` (create from `templates/person.md` if new).
   - otherwise → `notes/` with a sensible filename.
2. If genuinely unsure, leave the file in inbox and say why.
3. Prefer appending to existing notes over creating near-duplicate files.
4. `git mv`/edit, then one commit: `triage: <n> notes sorted`, push.
5. Report a table: file → destination.

Write note content in the vault's language (see `CLAUDE.md`).
