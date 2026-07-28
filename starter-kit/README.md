# Just Klauding Notes — Setup

A native macOS front-end for a plain-markdown notes vault, powered by Claude
Code. Browse & edit notes, triage your inbox, chat with your vault, and run
skills (`/meeting`, `/weekly`, …) — all over local markdown files in git.

The AI is **Claude Code**, which you install and log in to yourself. The app
just drives it against your vault.

---

## 1. Prerequisites

```sh
# Claude Code — install per https://claude.com/claude-code, then log in:
claude login

# CLI tools the app shells out to:
brew install ripgrep git
```

- **Claude Code** (`claude`) — the AI. Must be installed and logged in.
- **ripgrep** (`rg`) — powers search and the Tasks view.
- **git** — sync, move, delete (history-preserving).

## 2. Create your vault

Two starters ship here — pick one:

| Template | For |
| --- | --- |
| [`vault-template/`](vault-template) | **Neutral, English.** English folder names (`projects/ people/ notes/ inbox/`), English skills, note language set in `CLAUDE.md`. Start here if you're not sure. |
| [`vault-template-cs/`](vault-template-cs) | **Opinionated, Czech.** The author's own layout: `projekty/ lidi/ poznamky/ inbox/`, `ukoly.md`, `schuzky/`, person frontmatter `firma`/`pozice`/`email`, Czech note content, `/meeting` wired to MacWhisper exports. |

Copy one anywhere; `~/Notes` is found automatically:

```sh
cp -R "vault-template" ~/Notes      # or vault-template-cs
cd ~/Notes
git init && git add -A && git commit -m "init vault"
# optional: git remote add origin <your-repo> && git push -u origin main
```

Any other location works too — pick it in the app (Getting Started → **Change…**)
or set `NOTES_VAULT` (see step 4).

Either way the app detects the layout on first launch, so you can also rename
anything afterwards — just keep `CLAUDE.md`'s structure table in sync, since the
skills read it.

Edit `~/Notes/CLAUDE.md` to set the note language and add your project-name
aliases (how you *say* a project vs its folder slug) — it makes `ask` and triage
much smarter.

## 3. Install the app

Drag **Just Klauding Notes.app** to `/Applications` (or open the `.dmg`).

First launch shows a **Getting Started** dialog with a live checklist — it goes
green as each prerequisite is met. Reopen it anytime with the `?` button.

> Unsigned build: on first open, right-click the app → **Open** → **Open** to
> get past Gatekeeper.

## 4. Custom vault location

Everything is editable in-app via the **⚙ Settings** panel (sidebar footer):
vault (with a folder picker), folder names, task file, Claude model, hidden
folders, and the skills list. First-run also offers a vault **Change…** picker
in Getting Started.

On first launch the app **detects your vault's layout** (which folder is
projects, people, notes, inbox, and what the per-project task file is called) and
writes the result to `~/.config/just-klauding-notes/config.json`. It never
overwrites an existing config, and you can hand-edit it or re-run the scan with
**Settings → Re-detect layout**:

```json
{
  "vault": "~/Notes",
  "hidden_folders": ["archive", "templates"],
  "projects_dir": "projects",
  "people_dir": "people",
  "notes_dir": "notes",
  "inbox_dir": "inbox",
  "tasks_file": "tasks.md",
  "task_glob": "projects/**/tasks.md",
  "transcripts_dir": "~/Documents/transcripts",
  "model": "sonnet",
  "note_language": "",
  "archive_days": 7,
  "skills": [
    { "label": "Meeting summary", "cmd": "/meeting", "arg": false },
    { "label": "Weekly digest", "cmd": "/weekly", "arg": false },
    { "label": "Triage inbox", "cmd": "/inbox-triage", "arg": false },
    { "label": "New project", "cmd": "/new-project", "arg": true }
  ]
}
```

- **hidden_folders** — folders kept out of the sidebar tree.
- **projects_dir / people_dir / notes_dir / inbox_dir** — your structure; drive the Triage targets, capture location, and new-note default.
- **tasks_file** — the per-project task file name (detected).
- **task_glob** — ripgrep glob deciding which files the Tasks tab scans. A flat vault can use `**/*.md`, several names `**/{tasks,todo}.md`; `hidden_folders` are always excluded.
- **transcripts_dir** — extra folder granted to skills like `/meeting`.
- **model** — Claude model for ask / ai / skills (`sonnet`, `haiku`, …).
- **note_language** — language Claude writes note content in (e.g. `Czech`). Empty = it mirrors whatever language you asked in. The vault's `CLAUDE.md` wins if it says something different.
- **archive_days** — how long a done task stays in the Tasks view's **Done** list.
- **attachments_dir** — where dropped files land, relative to the note (default `assets`).
- **image_width** — default max width for images in the preview (default `50%`; per-image override with `![alt|300](x.png)`).
- **skills** — buttons in the sidebar, Chat bar, and native **Skills** menu. `arg: true` prefills the input instead of running immediately. Add your own vault skills here.

Using `vault-template-cs`? The detected config comes out like this instead —
`note_language` is the only field worth setting by hand:

```json
{
  "hidden_folders": ["templates"],
  "projects_dir": "projekty",
  "people_dir": "lidi",
  "notes_dir": "poznamky",
  "tasks_file": "ukoly.md",
  "task_glob": "projekty/**/ukoly.md",
  "note_language": "Czech"
}
```

The app also passes the resolved layout and `note_language` to Claude as a system
prompt on every call, so skills work against your folder names without being
edited.

(A `NOTES_VAULT` env var still works as a fallback for the vault path, as do
`~/Development/Notes`, `~/Notes`, `~/Documents/Notes` and `~/vault` if they exist.)

## 5. `note` shell commands (optional)

Mirror the app's capture / ask / ai on the command line:

```sh
echo 'source /path/to/note-commands.zsh' >> ~/.zshrc
```

- `note <text>` — instant capture to `inbox/` + commit
- `note ai <text>` — Claude files the note into the right place
- `note ask [-c] <question>` — Q&A over the vault (`-c` continues the last chat)

---

## What's in the vault

(Paths below are the English template's; `vault-template-cs` uses
`projekty/ lidi/ poznamky/` and `ukoly.md`.)

- `.claude/skills/` — `note`, `inbox-triage`, `meeting`, `weekly`, `new-project`.
  These are what the app's **Skills** menu and the Chat `/commands` run.
- `CLAUDE.md` — vault conventions + Q&A rules the assistant reads on every call.
- `templates/` — scaffolds for meetings, projects, people.
- `inbox/ projects/ people/ notes/` — your content. Rename them freely; the app detects the layout and `CLAUDE.md` tells the skills where things live.

## Using it

- **Notes** tab — browse the tree, read/edit, tick task checkboxes inline, see vault images rendered.
- **Drag & drop** a file onto a note — copied into `assets/` next to the note and linked at the caret. Delete the link and the file is removed (unless another note references it). Deleting a note asks whether to take its attachments and its emptied folder with it. Attachments folders are hidden from the tree; their parent gets a 📎 marker. Clicking a linked file opens it in its default app; URLs open in the browser.
- **Triage** — file inbox notes into a project (type to filter, or **Suggest**).
- **Chat** — talk to your vault; `--continue` keeps context. Skill buttons +
  the native **Skills** menu run workflows.
- **Tasks** — every open `- [ ]` across projects in one list.
- **⌘K** ask / ai / capture · **⌘P** quick-open + full-text search · **Sync** commit + push.

## Notes on `/meeting`

`/meeting` with no args processes every unprocessed transcript in
`~/Documents/transcripts/` (a MacWhisper export folder), oldest first, tracking
what it has done in `.processed-transcripts` at the vault root. The app grants access
to that folder automatically. Or run `/meeting <path-to-transcript>`.
