---
name: new-project
description: Scaffold a new client project folder. Use for "/new-project <name> [customer]".
---

# /new-project

1. Slug = lowercase ascii of the name.
2. `cp -r templates/project projekty/<slug>` + `mkdir projekty/<slug>/schuzky`.
3. Replace `{{project}}` with the name and `{{customer}}` with the customer (empty if not given) in all copied files.
4. Add the project row to root `README.md` table.
5. Commit `feat: new project <name>`, push.
