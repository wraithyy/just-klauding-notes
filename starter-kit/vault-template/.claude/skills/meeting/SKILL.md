---
name: meeting
description: Create a meeting note, or process a transcript/raw notes into a summarized meeting note with action items. Use for "/meeting <project> <title>" or "/meeting <path-to-transcript>".
---

# /meeting

Folder names below are the defaults from `CLAUDE.md` — follow that file if this
vault renamed them. Write note content in the vault's language.

## New empty note
`/meeting <project> <title>`: copy `templates/meeting.md` to `projects/<slug>/meetings/YYYY-MM-DD-<title-slug>.md`, fill `{{date}}`, `{{project}}`, `{{title}}`. Commit `meeting: <project> <title>`.

## Process transcript / raw notes
`/meeting <file>` processes one transcript (a speech-to-text export or pasted notes).
Bare `/meeting` = process **every transcript in the transcripts folder that
hasn't been processed yet**, oldest first. The app grants that folder via
`--add-dir`; its path is the `transcripts_dir` setting (default
`~/Documents/transcripts`).

Processed files are tracked in `.processed-transcripts` at the vault root (one
filename per line). To find the unprocessed ones: `ls` the transcripts folder,
skip any filename already listed in the ledger (create the ledger if missing).

> Non-interactive: you are usually run headless (from the app), so never stop to
> ask or wait for approval. If nothing is unprocessed, say so and stop.

For each unprocessed transcript:
1. Identify project and date. If the project is unclear, pick the most likely `projects/` slug from the content; if none fits, file the note in `notes/`. Do not stop to ask.
2. Create the note (copy `templates/meeting.md`); fill `## Notes` with a structured summary (decisions, topics), `summary:` frontmatter with 1-2 sentences, `participants:`.
3. Extract tasks assigned to the vault owner → `- [ ]` lines appended to `projects/<slug>/tasks.md` (create from `templates/project/tasks.md` if missing) and mirrored in the note's `## My action items`.
4. New people mentioned with a role → create/update `people/<Name>.md` (template `templates/person.md`; frontmatter only `company`, `role`, `email`).
5. Append the transcript's filename to `.processed-transcripts`.

Then commit once and push. Do NOT commit the raw transcripts. Reply with each note path + extracted tasks.
