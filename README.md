<div align="center">

# Just Klauding Notes

A tiny (~9 MB) native macOS app for a plain-markdown notes vault, powered by
[Claude Code](https://claude.com/claude-code).

Browse & edit notes, triage your inbox, chat with your vault, and run your own
Claude skills — all over local markdown files in git. No lock-in, no database:
your notes stay plain `.md`.

</div>

---

## Features

- **Notes** — folder tree, rendered markdown with inline **clickable task checkboxes**, a raw-markdown editor, frontmatter shown as a clean meta card, clickable relative links.
- **Chat** — multi-turn conversation with your vault (context kept via `claude --continue`), with buttons for your skills.
- **Triage** — file inbox notes into projects: pick a note, fuzzy-filter the target folder or let Claude **Suggest** one.
- **Tasks** — every open `- [ ]` across your projects in one checklist; tick to write back.
- **⌘P** quick-open + full-text search (ripgrep) · **⌘K** ask / file / capture · on-demand **git sync**.
- **Skills** — your vault's Claude skills as sidebar buttons, a chat bar, and a native macOS menu.
- **Config UI** — point it at any vault; folder names, task file, model and skills are all editable in **Settings**.
- Brand-themed, light + dark, deliberately not-generic UI.

## Requirements

The app drives Claude Code against your vault — you install and log in yourself.

- [Claude Code](https://claude.com/claude-code) (`claude`), logged in
- `ripgrep` and `git` — `brew install ripgrep git`
- A markdown vault (see [`starter-kit/vault-template`](starter-kit/vault-template))

## Install

Download the latest `.dmg` from [**Releases**](../../releases), open it, and
drag the app to Applications.

> Unsigned build: first launch → right-click the app → **Open** → **Open**.

On first run a **Getting Started** dialog checks your setup and lets you pick
your vault folder.

## Build from source

```sh
# prerequisites: Node + pnpm, Rust toolchain
pnpm install
pnpm tauri dev      # run
pnpm tauri build    # produce .app + .dmg in src-tauri/target/release/bundle
```

## Configuration

Set via **⚙ Settings** in the app (or edit
`~/.config/just-klauding-notes/config.json`). Every field has a sensible
default; override to match your own vault layout:

| Key | What |
| --- | --- |
| `vault` | Vault location (folder picker in Settings) |
| `hidden_folders` | Folders kept out of the tree |
| `projects_dir` / `people_dir` / `notes_dir` / `inbox_dir` | Your structure |
| `tasks_file` | File scanned for `- [ ]` per project |
| `transcripts_dir` | Extra folder granted to skills like `/meeting` |
| `model` | Claude model for ask / file / skills |
| `skills` | Buttons + native menu (`{ label, cmd, arg }`) |

Resolution order: config file → `NOTES_VAULT` env → default (`~/Development/Notes`).

## The vault

The "AI" lives in your **vault**, not the app: `.claude/skills/` and `CLAUDE.md`
are read by Claude Code on each call (the app runs `claude -p … --setting-sources project`
with the vault as the working directory).

[`starter-kit/`](starter-kit) has a shareable skeleton vault, optional `note`
shell commands, and a full setup guide.

## Tech

Tauri v2 (Rust) · React + TypeScript + Vite · CodeMirror 6 · ripgrep · git.

## License

MIT © Josef Kvapil
