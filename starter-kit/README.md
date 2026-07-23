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

Copy the template to `~/Development/Notes` (the default location):

```sh
cp -R "vault-template" ~/Development/Notes
cd ~/Development/Notes
git init && git add -A && git commit -m "init vault"
# optional: git remote add origin <your-repo> && git push -u origin main
```

Prefer a different location? Put it anywhere and tell the app via an env var
(see step 4).

Edit `~/Development/Notes/CLAUDE.md` to add your project-name aliases (how you
*say* a project vs its folder slug) — it makes `ask` and triage much smarter.

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

Settings are saved to `~/.config/just-klauding-notes/config.json`, which you can
also hand-edit — the app writes every field with its current value so you can
adapt it to your own vault layout:

```json
{
  "vault": "~/Development/Notes",
  "hidden_folders": ["archiv", "peceni"],
  "projects_dir": "projekty",
  "people_dir": "lidi",
  "notes_dir": "poznamky",
  "inbox_dir": "inbox",
  "tasks_file": "ukoly.md",
  "transcripts_dir": "~/Documents/transcripts",
  "model": "sonnet",
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
- **tasks_file** — the file scanned for `- [ ]` in each project (Tasks tab).
- **transcripts_dir** — extra folder granted to skills like `/meeting`.
- **model** — Claude model for ask / ai / skills (`sonnet`, `haiku`, …).
- **skills** — buttons in the sidebar, Chat bar, and native **Skills** menu. `arg: true` prefills the input instead of running immediately. Add your own vault skills here.

(A `NOTES_VAULT` env var still works as a fallback for the vault path.)

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

- `.claude/skills/` — `note`, `inbox-triage`, `meeting`, `weekly`, `new-project`.
  These are what the app's **Skills** menu and the Chat `/commands` run.
- `CLAUDE.md` — vault conventions + Q&A rules the assistant reads on every call.
- `templates/` — scaffolds for meetings, projects, people.
- `inbox/ projekty/ lidi/ poznamky/` — your content.

## Using it

- **Notes** tab — browse the tree, read/edit, tick task checkboxes inline.
- **Triage** — file inbox notes into a project (type to filter, or **Suggest**).
- **Chat** — talk to your vault; `--continue` keeps context. Skill buttons +
  the native **Skills** menu run workflows.
- **Tasks** — every open `- [ ]` across projects in one list.
- **⌘K** ask / ai / capture · **⌘P** quick-open + full-text search · **Sync** commit + push.

## Notes on `/meeting`

`/meeting` with no args processes the newest transcript in
`~/Documents/transcripts/` (a MacWhisper export folder). The app grants access
to that folder automatically. Or run `/meeting <path-to-transcript>`.
