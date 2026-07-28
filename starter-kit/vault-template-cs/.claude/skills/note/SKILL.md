---
name: note
description: Quick capture a note into inbox/. Use when the user wants to jot something down fast ("/note <text>", "poznamenej si", "zapiš"). Creates timestamped file, commits and pushes.
---

# /note — quick capture

1. Take the argument text. Derive a short ascii slug (3-5 words, lowercase, dashes) from it.
2. Write the text verbatim to `inbox/YYYY-MM-DD-HHmm-<slug>.md` (use `date +%Y-%m-%d-%H%M`). No frontmatter, no headings unless the text has them.
3. `git add <file> && git commit -m "note: <slug>" && git push`.
4. Reply with the file path only. The GitHub Action will triage it.
