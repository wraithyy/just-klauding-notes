# Second Brain Vault

Personal knowledge base. Plain markdown + git (no Obsidian). Write content in
your language; keep config/skills in English.

## Structure

| Folder | Purpose |
| --- | --- |
| `inbox/` | Quick capture (`YYYY-MM-DD-HHmm-slug.md`) — triaged by /inbox-triage |
| `projekty/<slug>/` | Client projects. Scaffold: `README.md` (hub: people, notes), `odkazy.md` (URLs + DEV/TEST/PROD), `pristupy.md` (credentials), `ukoly.md` (`- [ ]` tasks), `schuzky/` (meeting notes) |
| `lidi/` | Person profiles. Frontmatter: `firma`, `pozice`, `email` only |
| `poznamky/` | General notes + non-project meetings |
| `archiv/` | Archived — read-only, do not modify without asking |
| `templates/` | Plain md templates with `{{placeholder}}` — used by skills |

## Conventions

- Meeting notes: `projekty/<slug>/schuzky/YYYY-MM-DD-nazev.md` (ascii slug filename). Frontmatter: `date`, `type: meeting`, `firma`, `summary`, `project`, `participants`.
- Meetings for project X = list `projekty/<slug>/schuzky/`. Non-project meetings live in `poznamky/`.
- Standard markdown links (relative, URL-encoded). No wikilinks, no dataview, no Templater.
- Folder/file names for new project content: lowercase ascii slugs.
- New project = copy `templates/project/` scaffold (use /new-project).

## Q&A rules

- Answer from vault content; cite file paths.
- Grep before claiming "not found" (for accented languages, try both accented and plain forms).
- Open tasks = `- [ ]` in `projekty/*/ukoly.md`.
- Spoken project names map to `projekty/` slugs — resolve before searching. Add
  your own aliases here so the assistant recognises how you refer to projects,
  e.g.:
  `"the acme thing" → acme`, `"big client" → bigcorp-portal`.
  Unknown name → `ls projekty/` first.

## Skills

`/note` (capture to inbox), `/inbox-triage` (sort inbox), `/meeting` (create + summarize meeting from a transcript/notes, extract tasks, update lidi), `/weekly` (cross-project digest), `/new-project` (scaffold).

## Sync

Optional: commit + push on a schedule (launchd/cron) or use the app's Sync
button. Skills commit their own changes with real messages (`feat:`/`note:`/`meeting:` …).
