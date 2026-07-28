<div align="center">

# Just Klauding Notes

A tiny (7 MB download) native macOS app for a plain-markdown notes vault, powered by
[Claude Code](https://claude.com/claude-code).

Browse & edit notes, triage your inbox, chat with your vault, and run your own
Claude skills — all over local markdown files in git. No lock-in, no database:
your notes stay plain `.md`.

[![release](https://img.shields.io/github/v/release/wraithyy/just-klauding-notes?display_name=tag)](../../releases)
[![macOS](https://img.shields.io/badge/macOS-11%2B-black?logo=apple)](../../releases)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

```sh
brew install --cask wraithyy/tap/just-klauding-notes
```

</div>

---

## Features

- **Notes** — folder tree, rendered markdown with inline **clickable task checkboxes**, a raw-markdown editor, frontmatter shown as a clean meta card, clickable relative links, and **images from the vault** rendered inline.
- **Drag & drop** — drop any file on a note: it is copied into the note's `assets/` folder and linked at the caret — images inline, other files as a link with a document icon. Remove the link and the file goes with it; attachment folders stay out of the tree, with a paperclip on the nearest folder instead.
- **Links that leave the app** — `https://` opens in your browser, a linked `.docx`/`.pdf`/`.xlsx` opens in whatever app owns the type, `.md` opens in the app.
- **Chat** — multi-turn conversation with your vault (context kept via `claude --continue`), with buttons for your skills.
- **Triage** — file inbox notes into projects: pick a note, fuzzy-filter the target folder or let Claude **Suggest** one.
- **Tasks** — every `- [ ]` across your projects in one checklist, grouped by project; tick to write back. Ticking stamps the line with `✅ 2026-07-28`, and finished tasks drop into a collapsed **Done** section that only keeps the last few days (configurable).
- **⌘P** quick-open + full-text search (ripgrep) · **⌘K** ask / file / capture · on-demand **git sync**.
- **Skills** — your vault's Claude skills as sidebar buttons, a chat bar, and a native macOS menu.
- **Config UI** — point it at any vault; folder names, task file, model and skills are all editable in **Settings**.
- Brand-themed, light + dark, deliberately not-generic UI.

## Requirements

The app drives Claude Code against your vault — you install and log in yourself.

- macOS 11+
- [Claude Code](https://claude.com/claude-code) (`claude`), logged in
- `ripgrep` (search + tasks) and `git` (sync) — `brew install ripgrep git`; the
  cask installs `ripgrep` for you, `git` ships with the Xcode CLI tools
- A markdown vault — bring your own, or start from a template:
  [`vault-template`](starter-kit/vault-template) (neutral, English) or
  [`vault-template-cs`](starter-kit/vault-template-cs) (opinionated Czech: `projekty/`,
  `lidi/`, `ukoly.md`, Czech note content)

## Install

### Homebrew (recommended)

```sh
brew install --cask wraithyy/tap/just-klauding-notes
```

The cask pulls in `ripgrep` and strips the quarantine flag for you, so the app
opens on first double-click.

```sh
brew upgrade --cask just-klauding-notes    # update
brew uninstall --cask just-klauding-notes  # remove
brew uninstall --zap --cask just-klauding-notes  # remove + config
```

### Manual

Download the latest `.dmg` from [**Releases**](../../releases) (universal, Intel
+ Apple silicon), open it, drag the app to Applications.

> The build is **unsigned**, so Gatekeeper blocks the first launch. Either
> right-click the app → **Open** → **Open**, or run
> `xattr -dr com.apple.quarantine "/Applications/Just Klauding Notes.app"`.

### First run

A **Getting Started** dialog checks `claude`, `ripgrep`, `git` and your vault,
and lets you pick the vault folder. The installed version is shown next to the
title in **⚙ Settings**.

The app then **detects your vault's layout** instead of assuming one: it looks
for the folder that plays each role — projects (`projects`, `projekty`,
`clients`, `work`, …), people (`people`, `contacts`, …), notes (`notes`,
`zettel`, …), inbox (`inbox`, `00-inbox`, …) — and the per-project task file
(`tasks.md`, `todo.md`, `ukoly.md`). Whatever it finds is written to
`~/.config/just-klauding-notes/config.json`, which you can then edit freely;
the app never overwrites an existing config. Nothing recognised → neutral
defaults (`projects`, `people`, `notes`, `inbox`, `tasks.md`).

Renamed or reorganised your vault later? **⚙ Settings → Re-detect layout**
re-runs the scan and fills the form; it only takes effect when you hit Save.

## Configuration

Set via **⚙ Settings** in the app (or edit
`~/.config/just-klauding-notes/config.json`). Every field has a sensible
default; override to match your own vault layout:

| Key | What |
| --- | --- |
| `vault` | Vault location (folder picker in Settings) |
| `hidden_folders` | Folders kept out of the tree |
| `projects_dir` / `people_dir` / `notes_dir` / `inbox_dir` | Your structure |
| `tasks_file` | Per-project task file name (detected) |
| `task_glob` | ripgrep glob deciding which files the Tasks view scans. Default `{projects_dir}/**/{tasks_file}`; a flat vault can use `**/*.md`, several names `**/{tasks,todo}.md`. `hidden_folders` are always excluded. |
| `transcripts_dir` | Extra folder granted to skills like `/meeting` |
| `attachments_dir` | Folder dropped files are copied into, relative to the note (default `assets`) |
| `image_width` | Default max width for images in the preview (default `50%`; any CSS length) |
| `model` | Claude model for ask / file / skills |
| `note_language` | Language Claude writes note content in (e.g. `Czech`). Empty = mirrors the language you asked in. |
| `archive_days` | How long a done task stays in the **Done** list (default `7`) |
| `skills` | Buttons + native menu (`{ label, cmd, arg }`) |

Vault resolution order: config file → `NOTES_VAULT` env → the first of
`~/Development/Notes`, `~/Notes`, `~/Documents/Notes`, `~/vault` that exists.
Every other field: config file → detected from the vault → neutral default, so a
partial config works fine.

Nothing is ever deleted from your files: `archive_days` only controls what the
Tasks view renders. Old `- [x] … ✅ <date>` lines stay in your task files — grep or
Claude can still read them.

## Images & attachments

Vault-relative and absolute-from-root paths both work: `![shot](assets/a.png)`,
`![shot](/inbox/a.png)`. Remote `https://` and inline `data:` sources render as-is.

Width defaults to `image_width` (50% of the reading column) and can be overridden
per image with an Obsidian-style hint in the alt text — `![diagram|300](a.png)`
for pixels, `![shot|80%](a.png)` for a percentage.

Images are read through the backend and inlined as data URIs, so nothing outside
the vault is reachable from the preview. Files over 20 MB are not inlined.

**Dropping files.** Drag any file onto an open note. It is copied to
`<note-folder>/<attachments_dir>/` (default `assets/`) with an ascii-slugged name
— an existing name is never overwritten, it gets `-1`, `-2`, … — and a link is
inserted at the caret (at the end of the note when you're in preview mode).
Images get `![name](assets/name.png)`, everything else `[name.ext](assets/name.ext)`.

**Deleting.** Remove an attachment's link from the note and the file is deleted
on the next autosave — but only if it sits in that note's attachments folder and
**no other note in the vault mentions it**, so shared images stay. Recoverable
from git if the file was committed; a file dropped and never committed is not.

Deleting a note asks, so nothing goes silently: first whether to delete the local
files it links to — **any** linked file, not just the ones in `attachments_dir`,
and never other notes — then, for every folder the delete left empty, whether to
remove that folder too. Files another note still references are kept either way.
An emptied attachments folder is removed without asking, since the app is the
only thing that puts files there.

**Out of the way.** Folders that hold no notes — `assets/` and any other folder
you keep files in — are left out of the sidebar tree. The nearest folder that is
in the tree gets a paperclip instead, as does a folder with loose files of its
own, so the marker sits next to where the files are used. Hover it for a tooltip.

**Opening.** Clicking a linked file opens it in its default app; a `https://`
link opens the browser. Paths are confined to the vault.

## Keyboard

| Keys | Action |
| --- | --- |
| `⌘P` | Quick-open a note / full-text search the vault |
| `⌘K` | Claude palette — ask, file a note, capture to inbox |

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| *"app is damaged / can't be opened"* | Unsigned build — `xattr -dr com.apple.quarantine "/Applications/Just Klauding Notes.app"` |
| Search & Tasks stay empty | `ripgrep` missing — `brew install ripgrep`. Getting Started shows which checks fail. |
| Chat/skills return an error | `claude` not on PATH or not logged in — run `claude` once in a terminal. |
| Skills can't see a folder | Only the vault (plus `transcripts_dir`) is granted to Claude; move the folder in or repoint the setting. |
| Wrong vault | **⚙ Settings → Vault → Change…** |

## Build from source

```sh
# prerequisites: Node 22+, pnpm 11, Rust toolchain
pnpm install
pnpm tauri dev      # run with hot reload
pnpm tauri build    # produce .app + .dmg in src-tauri/target/release/bundle
```

Install your own build:

```sh
cp -R "src-tauri/target/release/bundle/macos/Just Klauding Notes.app" /Applications/
xattr -dr com.apple.quarantine "/Applications/Just Klauding Notes.app"
```

### Releasing

1. Bump the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. `git tag vX.Y.Z && git push origin vX.Y.Z` — [`release.yml`](.github/workflows/release.yml) builds a universal `.dmg` and opens a **draft** release.
3. `gh release edit vX.Y.Z --draft=false`.
4. Bump `version` + `sha256` in [`wraithyy/homebrew-tap`](https://github.com/wraithyy/homebrew-tap)`/Casks/just-klauding-notes.rb` (`shasum -a 256` on the released dmg).

## The vault

The "AI" lives in your **vault**, not the app: `.claude/skills/` and `CLAUDE.md`
are read by Claude Code on each call (the app runs `claude -p … --setting-sources project`
with the vault as the working directory).

The app adds one thing the vault can't know: your resolved layout and
`note_language`, passed as `--append-system-prompt`. That way the shipped skills
work against your folder names, in your language, without being edited. Your
`CLAUDE.md` still wins where the two disagree.

[`starter-kit/`](starter-kit) has two skeleton vaults (English and opinionated
Czech), optional `note` shell commands, and a full setup guide.

## Tech

Tauri v2 (Rust) · React + TypeScript + Vite · CodeMirror 6 · ripgrep · git.

## License

MIT © Josef Kvapil
