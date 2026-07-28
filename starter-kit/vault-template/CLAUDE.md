# Second Brain Vault

Personal knowledge base. Plain markdown + git (no Obsidian).

## Language

Note content is written in **English**. Replace that word with your own language
and the skills follow it. The app can also pass a language from its
`note_language` setting; this file wins if the two disagree.

Folder and file names stay lowercase ascii slugs whatever the content language.

## Structure

Rename these folders to whatever suits you — the app detects the layout. The
skills read the paths below, so keep this table in sync if you rename.

| Folder | Purpose |
| --- | --- |
| `inbox/` | Quick capture (`YYYY-MM-DD-HHmm-slug.md`) — triaged by /inbox-triage |
| `projects/<slug>/` | Client projects. Scaffold: `README.md` (hub: people, notes), `links.md` (URLs + DEV/TEST/PROD), `credentials.md`, `tasks.md` (`- [ ]` tasks), `meetings/` (meeting notes) |
| `people/` | Person profiles. Frontmatter: `company`, `role`, `email` only |
| `notes/` | General notes + non-project meetings |
| `archive/` | Archived — read-only, do not modify without asking |
| `templates/` | Plain md templates with `{{placeholder}}` — used by skills |

## Conventions

- Meeting notes: `projects/<slug>/meetings/YYYY-MM-DD-title.md` (ascii slug filename). Frontmatter: `date`, `type: meeting`, `company`, `summary`, `project`, `participants`.
- Meetings for project X = list `projects/<slug>/meetings/`. Non-project meetings live in `notes/`.
- Standard markdown links (relative, URL-encoded). No wikilinks, no dataview, no Templater.
- Folder/file names for new project content: lowercase ascii slugs.
- New project = copy the `templates/project/` scaffold (use /new-project).
- Ticking a task stamps it `- [x] … ✅ YYYY-MM-DD`; keep that format when ticking by hand.

## Q&A rules

- Answer from vault content; cite file paths.
- Grep before claiming "not found" (for accented languages, try both accented and plain forms).
- Open tasks = `- [ ]` in `projects/*/tasks.md`.
- Spoken project names map to `projects/` slugs — resolve before searching. Add
  your own aliases here so the assistant recognises how you refer to projects,
  e.g.:
  `"the acme thing" → acme`, `"big client" → bigcorp-portal`.
  Unknown name → `ls projects/` first.

## Skills

`/note` (capture to inbox), `/inbox-triage` (sort inbox), `/meeting` (create + summarize meeting from a transcript/notes, extract tasks, update `people/`), `/weekly` (cross-project digest), `/new-project` (scaffold).

## Sync

Optional: commit + push on a schedule (launchd/cron) or use the app's Sync
button. Skills commit their own changes with real messages (`feat:`/`note:`/`meeting:` …).
