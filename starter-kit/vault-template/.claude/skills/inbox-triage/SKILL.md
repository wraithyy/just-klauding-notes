---
name: inbox-triage
description: Sort everything in inbox/ into the right vault folder. Use when the user asks to clean/triage/sort the inbox.
---

# /inbox-triage

For each `.md` file in `inbox/` (skip `README.md`):

1. Read it. Decide destination:
   - mentions a project (see `projekty/` folder names, or grep project names) → the project folder: task-like content append to `ukoly.md` as `- [ ]`, meeting content to `schuzky/YYYY-MM-DD-nazev.md`, else append under `## Obecné poznámky` in the project `README.md` or keep as standalone file in the project folder.
   - about a person → `lidi/` (create from `templates/person.md` if new).
   - otherwise → `poznamky/` with a sensible filename.
2. If genuinely unsure, leave the file in inbox and say why.
3. Prefer appending to existing notes over creating near-duplicate files.
4. `git mv`/edit, then one commit: `triage: <n> notes sorted`, push.
5. Report a table: file → destination.
