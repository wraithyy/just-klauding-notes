// Notes GUI backend. All vault IO + `note` claude bridge live here; the
// frontend never touches the filesystem directly (no fs plugin needed).
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}
fn expand(v: &str) -> PathBuf {
    match v.strip_prefix("~/") {
        Some(rest) => Path::new(&home()).join(rest),
        None => PathBuf::from(v),
    }
}

// App config file: ~/.config/just-klauding-notes/config.json
fn config_path() -> PathBuf {
    Path::new(&home()).join(".config/just-klauding-notes/config.json")
}

#[derive(Serialize, Deserialize, Clone)]
struct Skill {
    label: String,
    cmd: String,
    arg: bool,
}

// Stored config — every field optional so the file can be partial.
#[derive(Serialize, Deserialize, Default)]
struct Config {
    vault: Option<String>,
    hidden_folders: Option<Vec<String>>,
    projects_dir: Option<String>,
    people_dir: Option<String>,
    notes_dir: Option<String>,
    inbox_dir: Option<String>,
    tasks_file: Option<String>,
    task_glob: Option<String>,
    transcripts_dir: Option<String>,
    attachments_dir: Option<String>,
    image_width: Option<String>,
    model: Option<String>,
    note_language: Option<String>,
    archive_days: Option<u32>,
    skills: Option<Vec<Skill>>,
}

// Config with defaults applied — what the app actually uses.
#[derive(Serialize, Deserialize, Clone)]
struct ResolvedConfig {
    vault: String,
    hidden_folders: Vec<String>,
    projects_dir: String,
    people_dir: String,
    notes_dir: String,
    inbox_dir: String,
    tasks_file: String,
    task_glob: String,
    transcripts_dir: String,
    attachments_dir: String,
    image_width: String,
    model: String,
    note_language: String,
    archive_days: u32,
    skills: Vec<Skill>,
}

fn default_skills() -> Vec<Skill> {
    let s = |label: &str, cmd: &str, arg: bool| Skill {
        label: label.into(),
        cmd: cmd.into(),
        arg,
    };
    vec![
        s("Meeting summary", "/meeting", false),
        s("Weekly digest", "/weekly", false),
        s("Triage inbox", "/inbox-triage", false),
        s("New project", "/new-project", true),
    ]
}

fn read_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// Layout detection: vaults are organised (and named) differently per person, so
// every folder role has a list of aliases. First one that actually exists in the
// vault wins; if none do, a neutral English name is used.
const PROJECTS_DIRS: &[&str] = &["projekty", "projects", "clients", "klienti", "work", "1-projects"];
const PEOPLE_DIRS: &[&str] = &["lidi", "people", "contacts", "kontakty"];
const NOTES_DIRS: &[&str] = &["poznamky", "notes", "zapisky", "zettel"];
const INBOX_DIRS: &[&str] = &["inbox", "00-inbox", "0-inbox", "_inbox"];
// ponytail: "peceni" and "tables" are the author's own folders, kept so his
// first launch is byte-identical; drop them once his config.json exists.
const HIDDEN_DIRS: &[&str] =
    &["archiv", "archive", "peceni", "tables", "templates", "attachments", "assets"];
const TASK_FILES: &[&str] = &["ukoly.md", "tasks.md", "todo.md", "TODO.md"];

// Sorted names of the directories directly inside `p`.
fn dir_names(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

// First candidate that exists in `present`, matched case-insensitively but
// returned with the vault's own spelling.
fn pick(cands: &[&str], present: &[String]) -> Option<String> {
    cands.iter().find_map(|c| {
        present
            .iter()
            .find(|p| p.eq_ignore_ascii_case(c))
            .cloned()
    })
}

// The task file name used inside the projects folder, e.g. `ukoly.md`. Looks one
// level down, since that is where per-project files live — and falls back to the
// project scaffold, so a fresh vault with no projects yet still detects the name
// its own template would create.
fn pick_task_file(vault: &Path, projects_dir: &str) -> Option<String> {
    let projects = vault.join(projects_dir);
    let subdirs = dir_names(&projects);
    let scaffold = vault.join("templates/project");
    TASK_FILES.iter().find_map(|f| {
        let in_project = subdirs.iter().any(|d| projects.join(d).join(f).is_file());
        (in_project || scaffold.join(f).is_file()).then(|| (*f).to_string())
    })
}

fn resolved() -> ResolvedConfig {
    resolved_from(read_config(), &vault())
}

// Config with defaults filled in. Anything the config file leaves out is
// detected from `v`'s actual contents, so a partial config still works.
fn resolved_from(c: Config, v: &Path) -> ResolvedConfig {
    let s = |o: Option<String>, d: &str| o.filter(|v| !v.trim().is_empty()).unwrap_or_else(|| d.into());
    let present = dir_names(v);
    let detect = |cands: &[&str], fallback: &str| {
        pick(cands, &present).unwrap_or_else(|| fallback.into())
    };
    let projects_dir = s(c.projects_dir, &detect(PROJECTS_DIRS, "projects"));
    let tasks_file = s(
        c.tasks_file,
        &pick_task_file(v, &projects_dir).unwrap_or_else(|| "tasks.md".into()),
    );
    ResolvedConfig {
        vault: v.to_string_lossy().to_string(),
        hidden_folders: c.hidden_folders.unwrap_or_else(|| {
            HIDDEN_DIRS
                .iter()
                .filter_map(|d| pick(&[d], &present))
                .collect()
        }),
        task_glob: s(c.task_glob, &format!("{projects_dir}/**/{tasks_file}")),
        projects_dir,
        people_dir: s(c.people_dir, &detect(PEOPLE_DIRS, "people")),
        notes_dir: s(c.notes_dir, &detect(NOTES_DIRS, "notes")),
        inbox_dir: s(c.inbox_dir, &detect(INBOX_DIRS, "inbox")),
        tasks_file,
        transcripts_dir: s(c.transcripts_dir, "~/Documents/transcripts"),
        // Per-note folder for dropped files, relative to the note itself.
        attachments_dir: s(c.attachments_dir, "assets"),
        // Half the reading column by default; `![alt|300](x.png)` overrides it.
        image_width: s(c.image_width, "50%"),
        model: s(c.model, "sonnet"),
        // Empty = say nothing and let Claude mirror the input language.
        note_language: c.note_language.unwrap_or_default(),
        archive_days: c.archive_days.unwrap_or(7),
        skills: c.skills.unwrap_or_else(default_skills),
    }
}

// Vault root — the only place the path is resolved: config file → NOTES_VAULT
// env → the first candidate that exists on disk → neutral default.
fn vault() -> PathBuf {
    if let Some(v) = read_config().vault {
        if !v.trim().is_empty() {
            return expand(v.trim());
        }
    }
    if let Ok(v) = std::env::var("NOTES_VAULT") {
        if !v.trim().is_empty() {
            return expand(v.trim());
        }
    }
    ["~/Development/Notes", "~/Notes", "~/Documents/Notes", "~/vault"]
        .iter()
        .map(|c| expand(c))
        .find(|p| p.is_dir())
        .unwrap_or_else(|| expand("~/Notes"))
}

#[tauri::command]
fn get_config() -> ResolvedConfig {
    let r = resolved();
    // First launch: freeze the detected layout into a file the user can edit.
    // Never touches an existing config, even a partial one.
    if !config_path().exists() {
        if let Err(e) = write_resolved(&r) {
            eprintln!("could not write initial config: {e}");
        }
    }
    r
}

// Detected layout for the current vault, ignoring whatever layout the config
// file holds. Returns it for the Settings form; writes nothing.
#[tauri::command]
fn detect_config() -> ResolvedConfig {
    let mut c = read_config();
    c.projects_dir = None;
    c.people_dir = None;
    c.notes_dir = None;
    c.inbox_dir = None;
    c.tasks_file = None;
    c.task_glob = None;
    c.hidden_folders = None;
    resolved_from(c, &vault())
}

fn write_resolved(r: &ResolvedConfig) -> Result<(), String> {
    let p = config_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, serde_json::to_string_pretty(r).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

// Persist a full config from the Settings panel.
#[tauri::command]
fn save_config(config: ResolvedConfig) -> Result<(), String> {
    write_resolved(&config)
}

// Persist the full resolved config (self-documenting) with a new vault path.
// The layout is re-detected against the new vault — keeping the old vault's
// folder names would leave the app pointing at directories that don't exist.
#[tauri::command]
fn set_vault(path: String) -> Result<(), String> {
    let mut c = read_config();
    c.vault = Some(path.clone());
    c.projects_dir = None;
    c.people_dir = None;
    c.notes_dir = None;
    c.inbox_dir = None;
    c.tasks_file = None;
    c.task_glob = None;
    c.hidden_folders = None;
    write_resolved(&resolved_from(c, &expand(path.trim())))
}

// A GUI app launched from Finder inherits a minimal PATH (no Homebrew/npm), so
// `claude`/`rg` wouldn't be found. Resolve the login shell's PATH once and use
// it for every spawned process.
fn shell_path() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let base = Command::new(&shell)
            .args(["-lic", "printf %s \"$PATH\""])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty());
        let cur = std::env::var("PATH").unwrap_or_default();
        match base {
            Some(b) => format!("{b}:{cur}"),
            None => cur,
        }
    })
}

// Spawn a command with the resolved login PATH.
fn proc(bin: &str) -> Command {
    let mut c = Command::new(bin);
    c.env("PATH", shell_path());
    c
}

// Resolve a vault-relative path, refusing anything that escapes the root.
fn resolve(rel: &str) -> Result<PathBuf, String> {
    let root = vault();
    let p = root.join(rel);
    let canon_root = root.canonicalize().map_err(|e| e.to_string())?;
    // Canonicalize the parent (file may not exist yet on write).
    let parent = p.parent().ok_or("no parent")?;
    let canon_parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if !canon_parent.starts_with(&canon_root) {
        return Err(format!("path escapes vault: {rel}"));
    }
    Ok(p)
}

#[derive(Serialize)]
struct Entry {
    path: String, // vault-relative
    name: String,
    is_dir: bool,
    // Dirs only: an attachments folder sits inside. The tree hides those folders
    // and shows a marker on the parent instead.
    has_assets: bool,
}

// Flat list of every note + folder, vault-relative. Frontend builds the tree.
// Skips .git, templates, and dotfiles.
#[tauri::command]
fn read_tree() -> Result<Vec<Entry>, String> {
    let root = vault();
    let r = resolved();
    let mut out = Vec::new();
    walk(&root, &root, &r.hidden_folders, &r.attachments_dir, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    for e in out.iter_mut().filter(|e| e.is_dir) {
        e.has_assets = root.join(&e.path).join(&r.attachments_dir).is_dir();
    }
    Ok(out)
}

fn walk(
    root: &Path,
    dir: &Path,
    hidden: &[String],
    attachments: &str,
    out: &mut Vec<Entry>,
) -> Result<(), String> {
    for e in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == attachments || hidden.iter().any(|h| h == &name) {
            continue;
        }
        let p = e.path();
        let is_dir = p.is_dir();
        if !is_dir && !name.ends_with(".md") {
            continue;
        }
        let rel = p.strip_prefix(root).map_err(|e| e.to_string())?;
        out.push(Entry {
            path: rel.to_string_lossy().to_string(),
            name,
            is_dir,
            has_assets: false,
        });
        if is_dir {
            walk(root, &p, hidden, attachments, out)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct Hit {
    path: String,
    line: u32,
    text: String,
}

// Full-text search via ripgrep (respects .gitignore, so .git is skipped).
// async so Tauri runs it off the main thread (sync commands block the UI).
#[tauri::command]
async fn grep(query: String) -> Result<Vec<Hit>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let out = proc("rg")
        .current_dir(vault())
        .args([
            "--line-number", "--no-heading", "--color", "never",
            "--max-count", "5", "--smart-case", "-g", "*.md", &query,
        ])
        .output()
        .map_err(|e| format!("rg failed (is ripgrep installed?): {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let hits = text
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, ':');
            Some(Hit {
                path: it.next()?.to_string(),
                line: it.next()?.parse().ok()?,
                text: it.next()?.trim().chars().take(120).collect(),
            })
        })
        .take(50)
        .collect();
    Ok(hits)
}

#[tauri::command]
fn read_note(rel: String) -> Result<String, String> {
    std::fs::read_to_string(resolve(&rel)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_note(rel: String, body: String) -> Result<(), String> {
    let p = resolve(&rel)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, body).map_err(|e| e.to_string())
}

// Triage drag: move a note, preserving git history.
#[tauri::command]
fn move_note(from: String, to: String) -> Result<(), String> {
    resolve(&from)?;
    let dest = resolve(&to)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    git(&["mv", &from, &to])
}

fn git(args: &[&str]) -> Result<(), String> {
    let out = proc("git")
        .current_dir(vault())
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

#[tauri::command]
fn delete_note(rel: String, with_assets: bool) -> Result<DeleteResult, String> {
    let r = resolved();
    let note_dir = Path::new(&rel)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    // Read the links before the file goes, so the attachments can be found.
    let links = if with_assets {
        note_links(&resolve(&rel)?)
    } else {
        Vec::new()
    };
    std::fs::remove_file(resolve(&rel)?).map_err(|e| e.to_string())?;

    let mut deleted_assets = Vec::new();
    if with_assets {
        deleted_assets = prune_attachments(rel.clone(), links)?;
    }
    // An emptied attachments folder always goes without asking — it is app
    // bookkeeping, not something the user put there.
    let dir_abs = if note_dir.is_empty() { vault() } else { resolve(&note_dir)? };
    remove_if_empty(&dir_abs.join(&r.attachments_dir));

    Ok(DeleteResult {
        deleted_assets,
        // The note's own folder is the user's, so only offer it.
        folder: (!note_dir.is_empty() && dir_is_empty(&dir_abs)).then_some(note_dir),
    })
}

#[derive(Serialize)]
struct DeleteResult {
    deleted_assets: Vec<String>,
    // Set when the note's folder is now empty and could be removed too.
    folder: Option<String>,
}

// Every link/image target in a markdown file.
fn note_links(p: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(p).unwrap_or_default();
    let mut out = Vec::new();
    // `](target)` — enough for the links this app writes; no markdown parser.
    for part in text.split("](").skip(1) {
        if let Some(end) = part.find(')') {
            let t = part[..end].trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

// Directories holding nothing but macOS cruft count as empty.
fn dir_is_empty(p: &Path) -> bool {
    std::fs::read_dir(p)
        .map(|it| it.flatten().all(|e| e.file_name() == ".DS_Store"))
        .unwrap_or(false)
}

// Delete a directory that is (effectively) empty. No-op otherwise.
fn remove_if_empty(p: &Path) -> bool {
    if !p.is_dir() || !dir_is_empty(p) {
        return false;
    }
    for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
        let _ = std::fs::remove_file(e.path());
    }
    std::fs::remove_dir(p).is_ok()
}

// Remove a folder the user was asked about. Refuses anything not empty.
#[tauri::command]
fn delete_dir(rel: String) -> Result<bool, String> {
    Ok(remove_if_empty(&resolve(&rel)?))
}

// Count of uncommitted changes.
#[tauri::command]
fn git_status() -> Result<u32, String> {
    let out = proc("git")
        .current_dir(vault())
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count() as u32)
}

// On-demand commit + push (autosync only runs every 30 min).
#[tauri::command]
async fn git_sync() -> Result<String, String> {
    let dirty = git_status()?;
    if dirty > 0 {
        git(&["add", "-A"])?;
        git(&["commit", "-m", "notes: sync from GUI"])?;
    }
    let push = proc("git")
        .current_dir(vault())
        .arg("push")
        .output()
        .map_err(|e| e.to_string())?;
    if !push.status.success() {
        return Err(String::from_utf8_lossy(&push.stderr).to_string());
    }
    Ok(format!("Synced {dirty} change(s)."))
}

#[derive(Serialize)]
struct Task {
    file: String,
    line: u32,
    text: String,
    done: bool,
    done_at: Option<String>,
    // Heading the task is listed under: the project it belongs to, at whatever
    // depth it sits, or its parent folder when it lives outside projects_dir.
    group: String,
}

fn task_group(file: &str, projects_dir: &str) -> String {
    let rel = file
        .strip_prefix(projects_dir)
        .map(|r| r.trim_start_matches('/'))
        .unwrap_or(file);
    let mut segs: Vec<&str> = rel.split('/').collect();
    segs.pop();
    if segs.is_empty() {
        rel.to_string()
    } else {
        segs.join("/")
    }
}

// Completion stamp appended to a ticked line: `- [x] foo ✅ 2026-07-27`.
// Split a task line's body into (text, done_at).
fn split_stamp(body: &str) -> (String, Option<String>) {
    match body.rsplit_once('✅') {
        Some((head, date)) => {
            let date = date.trim();
            let looks_iso = date.len() == 10 && date.as_bytes()[4] == b'-';
            if looks_iso && date.chars().all(|c| c.is_ascii_digit() || c == '-') {
                return (head.trim_end().to_string(), Some(date.to_string()));
            }
            (body.trim().to_string(), None)
        }
        None => (body.trim().to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pick, split_stamp, task_group, HIDDEN_DIRS, INBOX_DIRS, NOTES_DIRS, PEOPLE_DIRS,
        PROJECTS_DIRS,
    };

    #[test]
    fn stamp_parsing() {
        assert_eq!(split_stamp("koupit mléko"), ("koupit mléko".into(), None));
        assert_eq!(
            split_stamp("koupit mléko ✅ 2026-07-27"),
            ("koupit mléko".into(), Some("2026-07-27".into()))
        );
        // A bare emoji or junk date is text, not a stamp.
        assert_eq!(split_stamp("hotovo ✅"), ("hotovo ✅".into(), None));
        assert_eq!(split_stamp("✅ zítra"), ("✅ zítra".into(), None));
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn pick_aliases() {
        // The author's vault must keep resolving to exactly today's layout.
        let mine = names(&[
            "archiv", "inbox", "lidi", "peceni", "poznamky", "projekty", "tables", "templates",
        ]);
        assert_eq!(pick(PROJECTS_DIRS, &mine).as_deref(), Some("projekty"));
        assert_eq!(pick(PEOPLE_DIRS, &mine).as_deref(), Some("lidi"));
        assert_eq!(pick(NOTES_DIRS, &mine).as_deref(), Some("poznamky"));
        assert_eq!(pick(INBOX_DIRS, &mine).as_deref(), Some("inbox"));
        let hidden: Vec<String> = HIDDEN_DIRS.iter().filter_map(|d| pick(&[d], &mine)).collect();
        assert_eq!(hidden, names(&["archiv", "peceni", "tables", "templates"]));

        // An English vault detects its own names, whatever the case.
        let theirs = names(&["Inbox", "Notes", "People", "Projects", "archive"]);
        assert_eq!(pick(PROJECTS_DIRS, &theirs).as_deref(), Some("Projects"));
        assert_eq!(pick(INBOX_DIRS, &theirs).as_deref(), Some("Inbox"));

        // Nothing recognised → callers fall back to the neutral names.
        assert_eq!(pick(PROJECTS_DIRS, &names(&["stuff"])), None);
    }

    // Detection against a real (temporary) English-named vault: nothing but the
    // directory contents decides the layout.
    #[test]
    fn detect_foreign_layout() {
        let root = std::env::temp_dir().join(format!("jkn-test-{}", std::process::id()));
        for d in ["projects/acme", "people", "notes", "inbox", "archive"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        std::fs::write(root.join("projects/acme/tasks.md"), "- [ ] foo\n").unwrap();

        let r = super::resolved_from(super::Config::default(), &root);
        assert_eq!(r.projects_dir, "projects");
        assert_eq!(r.people_dir, "people");
        assert_eq!(r.notes_dir, "notes");
        assert_eq!(r.inbox_dir, "inbox");
        assert_eq!(r.tasks_file, "tasks.md");
        assert_eq!(r.task_glob, "projects/**/tasks.md");
        assert_eq!(r.hidden_folders, names(&["archive"]));

        // An explicit config field always wins over detection.
        let c = super::Config { task_glob: Some("**/*.md".into()), ..Default::default() };
        assert_eq!(super::resolved_from(c, &root).task_glob, "**/*.md");

        // A fresh vault with no projects yet: the scaffold names the task file.
        // (This is what copying either starter template gives you.)
        std::fs::remove_dir_all(root.join("projects/acme")).unwrap();
        std::fs::create_dir_all(root.join("templates/project")).unwrap();
        std::fs::write(root.join("templates/project/ukoly.md"), "- [ ]\n").unwrap();
        let r = super::resolved_from(super::Config::default(), &root);
        assert_eq!(r.tasks_file, "ukoly.md");
        assert_eq!(r.task_glob, "projects/**/ukoly.md");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn links_and_empty_dirs() {
        let root = std::env::temp_dir().join(format!("jkn-del-{}", std::process::id()));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        let note = root.join("note.md");
        std::fs::write(
            &note,
            "# T\n\n![a](assets/a.png)\n\n[doc](assets/b.docx) and [ext](https://x.dev)\n",
        )
        .unwrap();
        assert_eq!(
            super::note_links(&note),
            vec!["assets/a.png", "assets/b.docx", "https://x.dev"],
        );

        // A folder with only .DS_Store counts as empty and gets removed.
        let dir = root.join("assets");
        std::fs::write(dir.join(".DS_Store"), "junk").unwrap();
        assert!(super::dir_is_empty(&dir));
        assert!(super::remove_if_empty(&dir));
        assert!(!dir.exists());
        // A folder with real content is left alone.
        assert!(!super::remove_if_empty(&root));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scheme_detection() {
        use super::has_scheme;
        assert!(has_scheme("https://example.com"));
        assert!(has_scheme("mailto:a@b.cz"));
        assert!(!has_scheme("assets/report.docx"));
        assert!(!has_scheme("./notes/a.md"));
        assert!(!has_scheme("C:/tmp/x.txt"));
    }

    #[test]
    fn attachment_slug() {
        use super::slug;
        assert_eq!(slug("Screenshot 2026-07-28"), "screenshot-2026-07-28");
        assert_eq!(slug("Nabídka — Žížala v1"), "nabidka-zizala-v1");
        assert_eq!(slug("!!!"), "file");
        // Path separators and spaces can't survive into a filename.
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
    }

    #[test]
    fn grouping() {
        assert_eq!(task_group("projekty/acme/ukoly.md", "projekty"), "acme");
        assert_eq!(task_group("projekty/a/b/ukoly.md", "projekty"), "a/b");
        // Outside projects_dir: grouped by the folder it sits in.
        assert_eq!(task_group("poznamky/tasks.md", "projekty"), "poznamky");
        // Vault root: nothing to group by, so the file names itself.
        assert_eq!(task_group("tasks.md", "projekty"), "tasks.md");
        // A task file directly in projects_dir, not in a project folder.
        assert_eq!(task_group("projekty/ukoly.md", "projekty"), "ukoly.md");
    }
}

// Every checkbox task matching the configured task glob.
#[tauri::command]
async fn list_tasks() -> Result<Vec<Task>, String> {
    let r = resolved();
    let mut args: Vec<String> = ["--line-number", "--no-heading", "--color", "never"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    args.push("-g".into());
    args.push(r.task_glob.clone());
    // Archived/hidden folders stay out of the list even if the glob is wide.
    for h in &r.hidden_folders {
        args.push("-g".into());
        args.push(format!("!{h}/**"));
    }
    args.push(r"^\s*- \[[ xX]\]".into());
    let out = proc("rg")
        .current_dir(vault())
        .args(&args)
        .output()
        .map_err(|e| format!("rg failed: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, ':');
            let file = it.next()?.to_string();
            let line = it.next()?.parse().ok()?;
            let raw = it.next()?.trim();
            let done = raw.starts_with("- [x]") || raw.starts_with("- [X]");
            let body = raw
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim_start_matches("- [X]")
                .trim();
            let (text, done_at) = split_stamp(body);
            let group = task_group(&file, &r.projects_dir);
            Some(Task { file, line, text, done, done_at, group })
        })
        .collect())
}

// Max size inlined into the preview. Bigger images stay as a link — base64 in a
// webview is ~1.4x the file, and nothing good comes of a 50 MB <img>.
const MAX_INLINE_BYTES: u64 = 20 * 1024 * 1024;

// Read a vault image as a data URI. Going through a command instead of the
// asset:// protocol keeps this working the same in dev and release, with no
// protocol scope or CSP to get wrong.
#[tauri::command]
fn read_asset(rel: String) -> Result<String, String> {
    use base64::Engine;
    let p = resolve(&rel)?;
    let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    if meta.len() > MAX_INLINE_BYTES {
        return Err(format!("too large to inline: {} bytes", meta.len()));
    }
    let mime = match p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "heic" | "heif" => "image/heic",
        other => return Err(format!("not an image: .{other}")),
    };
    let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

// Open a URL in the browser, or a file in whatever app owns its type. Vault
// paths are resolved (and confined) first; anything with a scheme is treated as
// a URL. Used for the links the preview can't handle itself.
#[tauri::command]
fn open_external(target: String) -> Result<(), String> {
    if has_scheme(&target) {
        tauri_plugin_opener::open_url(target, None::<&str>).map_err(|e| e.to_string())
    } else {
        let p = resolve(&target)?;
        if !p.exists() {
            return Err(format!("not found: {target}"));
        }
        tauri_plugin_opener::open_path(p.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

// `scheme:` prefix check. Single-letter prefixes are excluded so a Windows-style
// `C:\…` (or a stray `a:`) isn't mistaken for a URL.
fn has_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(i) if i > 1 => s[..i].chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-'),
        _ => false,
    }
}

// Delete attachments a note no longer links to. Only touches files inside that
// note's attachments folder, and only when nothing else in the vault mentions
// the filename — a shared image stays put. Returns what was deleted.
#[tauri::command]
fn prune_attachments(note: String, removed: Vec<String>) -> Result<Vec<String>, String> {
    let r = resolved();
    let note_dir = Path::new(&note)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let prefix = format!("{}/", r.attachments_dir);
    let mut deleted = Vec::new();
    for rel in removed {
        if !rel.starts_with(&prefix) || rel.contains("..") {
            continue;
        }
        let name = match Path::new(&rel).file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        let vault_rel = if note_dir.is_empty() { rel.clone() } else { format!("{note_dir}/{rel}") };
        let abs = resolve(&vault_rel)?;
        if !abs.is_file() {
            continue;
        }
        // Still referenced anywhere (including this note, if it came back)? Keep it.
        let hits = proc("rg")
            .current_dir(vault())
            .args(["--files-with-matches", "--fixed-strings", "-g", "*.md", &name])
            .output()
            .map_err(|e| format!("rg failed: {e}"))?;
        if !String::from_utf8_lossy(&hits.stdout).trim().is_empty() {
            continue;
        }
        std::fs::remove_file(&abs).map_err(|e| e.to_string())?;
        deleted.push(vault_rel);
        // Tidy up the folder once its last attachment is gone.
        if let Some(dir) = abs.parent() {
            remove_if_empty(dir);
        }
    }
    Ok(deleted)
}

// Copy a dropped file into the vault next to the note that received it, under
// the configured attachments folder. Returns the path relative to the note, i.e.
// what goes inside the markdown link. Never overwrites: a clashing name gets
// `-1`, `-2`, … appended.
#[tauri::command]
fn attach_file(note: String, src: String) -> Result<String, String> {
    let src = PathBuf::from(&src);
    if !src.is_file() {
        return Err(format!("not a file: {}", src.to_string_lossy()));
    }
    let stem = src.file_stem().map(|s| slug(&s.to_string_lossy())).unwrap_or_else(|| "file".into());
    let ext = src
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    let r = resolved();
    let note_dir = Path::new(&note).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let rel_dir = if note_dir.is_empty() {
        r.attachments_dir.clone()
    } else {
        format!("{note_dir}/{}", r.attachments_dir)
    };
    let abs_dir = resolve(&rel_dir)?;
    std::fs::create_dir_all(&abs_dir).map_err(|e| e.to_string())?;

    let mut name = format!("{stem}{ext}");
    for n in 1.. {
        if !abs_dir.join(&name).exists() {
            break;
        }
        name = format!("{stem}-{n}{ext}");
    }
    std::fs::copy(&src, abs_dir.join(&name)).map_err(|e| e.to_string())?;
    Ok(format!("{}/{}", r.attachments_dir, name))
}

// Filename-safe ascii slug: strip accents' diacritics crudely, keep word chars.
fn slug(s: &str) -> String {
    let out: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'å' => 'a',
            'č' | 'ç' => 'c',
            'ď' => 'd',
            'é' | 'è' | 'ě' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ň' | 'ñ' => 'n',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ř' => 'r',
            'š' => 's',
            'ť' => 't',
            'ú' | 'ù' | 'û' | 'ü' | 'ů' => 'u',
            'ý' | 'ÿ' => 'y',
            'ž' => 'z',
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => c.to_ascii_lowercase(),
            _ => '-',
        })
        .collect();
    let out = out.trim_matches('-').to_string();
    // Collapse runs of dashes.
    let mut s = String::with_capacity(out.len());
    for c in out.chars() {
        if c != '-' || !s.ends_with('-') {
            s.push(c);
        }
    }
    if s.is_empty() { "file".into() } else { s }
}

// Flip a single checkbox and write the file back. Ticking stamps the line with
// `✅ <today>` (date comes from the frontend so it is in the user's timezone);
// unticking strips the stamp again.
#[tauri::command]
fn toggle_task(file: String, line: u32, today: String) -> Result<bool, String> {
    let p = resolve(&file)?;
    let content = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let l = lines
        .get_mut((line - 1) as usize)
        .ok_or("line out of range")?;
    let done;
    if l.contains("- [ ]") {
        *l = l.replacen("- [ ]", "- [x]", 1);
        if !l.contains('✅') {
            l.push_str(&format!(" ✅ {}", today.trim()));
        }
        done = true;
    } else if l.contains("- [x]") || l.contains("- [X]") {
        *l = l.replacen("- [x]", "- [ ]", 1).replacen("- [X]", "- [ ]", 1);
        if let Some((head, _)) = l.rsplit_once('✅') {
            *l = head.trim_end().to_string();
        }
        done = false;
    } else {
        return Err("not a task line".into());
    }
    std::fs::write(&p, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(done)
}

// Which folder plays which role in THIS vault, plus the language notes should be
// written in. Passed as a system prompt (not glued onto the prompt text) so it
// reaches slash-command skills too, without being mistaken for their arguments.
// The vault's own CLAUDE.md still wins where the two disagree.
fn vault_context(cfg: &ResolvedConfig) -> String {
    let mut s = format!(
        "This vault's layout, relative to its root: projects in `{}/`, people in `{}/`, \
         notes in `{}/`, inbox in `{}/`, per-project task file `{}`. \
         Use these paths rather than assuming any other folder names.",
        cfg.projects_dir, cfg.people_dir, cfg.notes_dir, cfg.inbox_dir, cfg.tasks_file
    );
    let lang = cfg.note_language.trim();
    if !lang.is_empty() {
        s.push_str(&format!(
            " Write note content in {lang}, regardless of the language of the request."
        ));
    }
    s
}

// Bridge to the `note` claude commands. Flags mirror dot_zshrc.tmpl exactly.
// kind: "ask" | "ai" | "capture" | "triage".
// async so the long claude call runs off the main thread (a sync command would
// freeze the whole UI while waiting). Frontend shows a spinner meanwhile.
// Live token streaming via tauri::ipc::Channel is the upgrade path.
#[tauri::command]
async fn run_note(kind: String, text: String, cont: bool) -> Result<String, String> {
    let cfg = resolved();
    let base: Vec<String> = ["-p", "--model", cfg.model.as_str(), "--strict-mcp-config",
                "--disable-slash-commands", "--setting-sources", "project"]
        .iter().map(|s| s.to_string()).collect();
    let ctx = vault_context(&cfg);
    let mut args: Vec<String> = Vec::new();
    match kind.as_str() {
        "ask" => {
            args.extend(base);
            args.push("--append-system-prompt".into());
            args.push(ctx);
            if cont {
                args.push("--continue".into());
            }
            args.push(text);
        }
        "ai" => {
            args.extend(base);
            args.extend(["--permission-mode", "acceptEdits"].iter().map(|s| s.to_string()));
            args.push("--append-system-prompt".into());
            args.push(ctx);
            // =form: --allowedTools is variadic and would consume the prompt.
            args.push("--allowedTools=Bash(git:*),Bash(date:*)".into());
            args.push(text);
        }
        // Vault skills (/meeting, /weekly, …). Slash commands MUST stay enabled
        // (no --disable-slash-commands). acceptEdits plus an explicit tool allow
        // list so skills can read transcripts (outside the vault), write notes,
        // and commit without the interactive prompts that -p can't answer.
        "skill" => {
            args.extend(["-p", "--model", cfg.model.as_str(), "--setting-sources", "project",
                         "--permission-mode", "acceptEdits"].iter().map(|s| s.to_string()));
            // =form: --allowedTools is variadic and would consume the prompt.
            args.push("--allowedTools=Read,Edit,Write,Glob,Grep,Bash".into());
            args.push("--append-system-prompt".into());
            args.push(ctx);
            // Some skills (e.g. /meeting) read transcripts outside the vault —
            // grant that dir if it exists.
            // --add-dir is variadic — as a separate token it swallows the
            // prompt. Use the =form so it stays one token and `text` remains
            // the positional prompt.
            let transcripts = expand(&cfg.transcripts_dir);
            if transcripts.is_dir() {
                args.push(format!("--add-dir={}", transcripts.to_string_lossy()));
            }
            args.push(text);
        }
        _ => return Err(format!("unknown kind: {kind}")),
    }
    // No usable stdin when spawned from the GUI; claude treats a slash-command
    // prompt as needing stdin input and errors otherwise. /dev/null tells it to
    // skip stdin and just run the command/prompt.
    let out = proc("claude")
        .current_dir(vault())
        .stdin(Stdio::null())
        .args(&args)
        .output()
        .map_err(|e| format!("spawn claude failed: {e}"))?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        s.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok(s)
}

#[derive(Serialize)]
struct EnvCheck {
    claude: bool,
    rg: bool,
    git: bool,
    vault: bool,
    vault_path: String,
}

// First-run readiness check for the getting-started dialog.
#[tauri::command]
fn check_env() -> EnvCheck {
    let has = |bin: &str| {
        proc("sh")
            .args(["-c", &format!("command -v {bin}")])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    let v = vault();
    EnvCheck {
        claude: has("claude"),
        rg: has("rg"),
        git: has("git"),
        vault: v.is_dir(),
        vault_path: v.to_string_lossy().to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::menu::{Menu, SubmenuBuilder};
    use tauri::Emitter;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Native "Skills" menu in the macOS menu bar (item id = skill:<cmd>),
            // built from the configured skills.
            let mut sb = SubmenuBuilder::new(app.handle(), "Skills");
            for sk in resolved().skills {
                sb = sb.text(format!("skill:{}", sk.cmd), sk.label);
            }
            let skills = sb.build()?;
            let menu = Menu::default(app.handle())?;
            menu.append(&skills)?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                if let Some(cmd) = event.id().0.strip_prefix("skill:") {
                    let _ = app.emit("run-skill", cmd.to_string());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_tree, grep, read_note, write_note, move_note, run_note,
            delete_note, git_status, git_sync, list_tasks, toggle_task, check_env, detect_config, attach_file, read_asset, open_external, prune_attachments, delete_dir,
            set_vault, get_config, save_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
