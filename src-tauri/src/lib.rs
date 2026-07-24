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
    transcripts_dir: Option<String>,
    model: Option<String>,
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
    transcripts_dir: String,
    model: String,
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

fn resolved() -> ResolvedConfig {
    let c = read_config();
    let s = |o: Option<String>, d: &str| o.filter(|v| !v.trim().is_empty()).unwrap_or_else(|| d.into());
    ResolvedConfig {
        vault: s(c.vault, "~/Development/Notes"),
        hidden_folders: c.hidden_folders.unwrap_or_else(|| vec!["archiv".into(), "peceni".into()]),
        projects_dir: s(c.projects_dir, "projekty"),
        people_dir: s(c.people_dir, "lidi"),
        notes_dir: s(c.notes_dir, "poznamky"),
        inbox_dir: s(c.inbox_dir, "inbox"),
        tasks_file: s(c.tasks_file, "ukoly.md"),
        transcripts_dir: s(c.transcripts_dir, "~/Documents/transcripts"),
        model: s(c.model, "sonnet"),
        skills: c.skills.unwrap_or_else(default_skills),
    }
}

// Vault root, resolved: config file → NOTES_VAULT env → default.
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
    Path::new(&home()).join("Development/Notes")
}

#[tauri::command]
fn get_config() -> ResolvedConfig {
    resolved()
}

// Persist a full config from the Settings panel.
#[tauri::command]
fn save_config(config: ResolvedConfig) -> Result<(), String> {
    let p = config_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        p,
        serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

// Persist the full resolved config (self-documenting) with a new vault path.
#[tauri::command]
fn set_vault(path: String) -> Result<(), String> {
    let mut r = resolved();
    r.vault = path;
    let p = config_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
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
}

// Flat list of every note + folder, vault-relative. Frontend builds the tree.
// Skips .git, templates, and dotfiles.
#[tauri::command]
fn read_tree() -> Result<Vec<Entry>, String> {
    let root = vault();
    let mut hidden = resolved().hidden_folders;
    hidden.push("templates".into());
    let mut out = Vec::new();
    walk(&root, &root, &hidden, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, hidden: &[String], out: &mut Vec<Entry>) -> Result<(), String> {
    for e in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || hidden.iter().any(|h| h == &name) {
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
        });
        if is_dir {
            walk(root, &p, hidden, out)?;
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
fn delete_note(rel: String) -> Result<(), String> {
    std::fs::remove_file(resolve(&rel)?).map_err(|e| e.to_string())
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
}

// Every checkbox task across projekty/*/ukoly.md.
#[tauri::command]
async fn list_tasks() -> Result<Vec<Task>, String> {
    let r = resolved();
    let glob = format!("{}/**/{}", r.projects_dir, r.tasks_file);
    let out = proc("rg")
        .current_dir(vault())
        .args([
            "--line-number", "--no-heading", "--color", "never",
            "-g", &glob, r"^\s*- \[[ xX]\]",
        ])
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
            let clean = raw
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim_start_matches("- [X]")
                .trim()
                .to_string();
            Some(Task { file, line, text: clean, done })
        })
        .collect())
}

// Flip a single checkbox and write the file back.
#[tauri::command]
fn toggle_task(file: String, line: u32) -> Result<bool, String> {
    let p = resolve(&file)?;
    let content = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let l = lines
        .get_mut((line - 1) as usize)
        .ok_or("line out of range")?;
    let done;
    if l.contains("- [ ]") {
        *l = l.replacen("- [ ]", "- [x]", 1);
        done = true;
    } else if l.contains("- [x]") || l.contains("- [X]") {
        *l = l.replacen("- [x]", "- [ ]", 1).replacen("- [X]", "- [ ]", 1);
        done = false;
    } else {
        return Err("not a task line".into());
    }
    std::fs::write(&p, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(done)
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
    let mut args: Vec<String> = Vec::new();
    match kind.as_str() {
        "ask" => {
            args.extend(base);
            if cont {
                args.push("--continue".into());
            }
            args.push(text);
        }
        "ai" => {
            args.extend(base);
            args.extend(["--permission-mode", "acceptEdits"].iter().map(|s| s.to_string()));
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
            delete_note, git_status, git_sync, list_tasks, toggle_task, check_env,
            set_vault, get_config, save_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
