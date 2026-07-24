---
name: meeting
description: Create a meeting note, or process a transcript/raw notes into a summarized meeting note with action items. Use for "/meeting <project> <title>" or "/meeting <path-to-transcript>".
---

# /meeting

## New empty note
`/meeting <project> <title>`: copy `templates/meeting.md` to `projekty/<slug>/schuzky/YYYY-MM-DD-<title-slug>.md`, fill `{{date}}`, `{{project}}`, `{{title}}`. Commit `meeting: <project> <title>`.

## Process transcript / raw notes
`/meeting <file>` (MacWhisper transcript or pasted notes). Bare `/meeting` with no args = take the newest file in `~/Documents/transcripts/` (MacWhisper export folder) and process it directly.

> Non-interactive: you are usually run headless (from the app), so never stop to ask or wait for approval. Pick the newest transcript and proceed; read it directly.

1. Identify project and date. If the project is unclear, pick the most likely `projekty/` slug from the content; if none fits, file the note in `poznamky/`. Do not stop to ask.
2. Create the note as above; fill `## Poznámky` with a structured summary (decisions, topics), `summary:` frontmatter with 1-2 sentences, `participants:`.
3. Extract tasks assigned to the vault owner → `- [ ]` lines appended to `projekty/<slug>/ukoly.md` (create from `templates/project/ukoly.md` if missing) and mirrored in the note's `## Úkoly na mě`.
4. New people mentioned with a role → create/update `lidi/<Jméno>.md` (template `templates/person.md`; frontmatter only `firma`, `pozice`, `email`).
5. One commit `meeting: <project> YYYY-MM-DD <title>`, push. Do NOT commit the raw transcript.
6. Reply: note path + extracted tasks list.

Content in Czech.
