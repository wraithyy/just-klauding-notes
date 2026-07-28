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

- **Notes** — folder tree, rendered markdown with inline **clickable task checkboxes**, a raw-markdown editor, frontmatter shown as a clean meta card, clickable relative links.
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
- A markdown vault — bring your own, or start from
  [`starter-kit/vault-template`](starter-kit/vault-template)

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
| `archive_days` | How long a done task stays in the **Done** list (default `7`) |
| `skills` | Buttons + native menu (`{ label, cmd, arg }`) |

Resolution order: config file → `NOTES_VAULT` env → default (`~/Development/Notes`).

Nothing is ever deleted from your files: `archive_days` only controls what the
Tasks view renders. Old `- [x] … ✅ <date>` lines stay in `ukoly.md` — grep or
Claude can still read them.

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

[`starter-kit/`](starter-kit) has a shareable skeleton vault, optional `note`
shell commands, and a full setup guide.

## Tech

Tauri v2 (Rust) · React + TypeScript + Vite · CodeMirror 6 · ripgrep · git.

## License

MIT © Josef Kvapil
