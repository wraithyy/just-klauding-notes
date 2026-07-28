import { useEffect, useMemo, useRef, useState } from "react";
import CodeMirror, { type ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  type Entry,
  type Hit,
  type Task,
  type EnvCheck,
  type Config,
  readTree,
  grep,
  checkEnv,
  setVault,
  getConfig,
  saveConfig,
  detectConfig,
  readNote,
  writeNote,
  moveNote,
  deleteNote,
  deleteDir,
  runNote,
  attachFile,
  readAsset,
  openExternal,
  pruneAttachments,
  gitStatus,
  gitSync,
  listTasks,
  toggleTask,
  isoDate,
  isoDaysAgo,
  inboxName,
} from "./api";
import logo from "./assets/logo.svg";
import "./App.css";

type Msg = { role: "user" | "claude"; text: string };

// Join a vault-relative path against the note it appears in. A leading `/`
// means vault root; `.`/`..` segments are collapsed.
function joinRel(fromRel: string, target: string): string {
  const dir = target.startsWith("/") ? [] : fromRel.split("/").slice(0, -1);
  for (const p of target.split("/")) {
    if (p === "..") dir.pop();
    else if (p !== "." && p !== "") dir.push(p);
  }
  return dir.join("/");
}

// Resolve a relative md link against the note it lives in. Returns null for
// external/non-note links (left to default handling).
function resolveLink(fromRel: string, href: string): string | null {
  const clean = decodeURIComponent(href.split("#")[0]);
  if (/^[a-z]+:\/\//i.test(href) || !clean.endsWith(".md")) return null;
  return joinRel(fromRel, clean);
}

const IMAGE_EXT = /\.(png|jpe?g|gif|webp|avif|svg|bmp|heic)$/i;

// Obsidian-style size hint in the alt text: `![diagram|300](x.png)` or
// `![shot|60%](x.png)`. A bare number means pixels.
function splitAlt(alt: string): { alt: string; width?: string } {
  const m = /^(.*)\|\s*(\d+%?|\d+px)\s*$/.exec(alt);
  if (!m) return { alt };
  const w = m[2];
  return { alt: m[1].trim(), width: /^\d+$/.test(w) ? `${w}px` : w };
}

// Vault images are read through a command and inlined as data URIs — no asset
// protocol, so dev and release behave identically. Cached per path because
// react-markdown re-renders the whole tree on every keystroke.
const assetCache = new Map<string, string>();

function VaultImage({
  rel,
  alt,
  width,
}: {
  rel: string;
  alt: string;
  width?: string;
}) {
  const [uri, setUri] = useState(() => assetCache.get(rel));
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    const hit = assetCache.get(rel);
    if (hit) {
      setUri(hit);
      setFailed(false);
      return;
    }
    let live = true;
    setUri(undefined);
    setFailed(false);
    readAsset(rel)
      .then((u) => {
        assetCache.set(rel, u);
        if (live) setUri(u);
      })
      .catch(() => live && setFailed(true));
    return () => {
      live = false;
    };
  }, [rel]);
  if (failed) return <span className="img-missing">{alt || rel} — not found</span>;
  if (!uri) return <span className="img-loading" />;
  return <img src={uri} alt={alt} style={width ? { maxWidth: width } : undefined} />;
}

// Every link/image target in a note, decoded. Used to spot attachments the note
// stopped referencing.
function linkTargets(md: string): string[] {
  return [...md.matchAll(/!?\[[^\]]*\]\(([^)\s]+)\)/g)].map((m) =>
    decodeURIComponent(m[1]),
  );
}

// Version straight from tauri.conf.json — one source of truth, no duplication.
function useAppVersion() {
  const [v, setV] = useState("");
  useEffect(() => {
    getVersion().then(setV).catch(() => {});
  }, []);
  return v;
}

function usePrefersDark() {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const m = window.matchMedia("(prefers-color-scheme: dark)");
    const h = () => setDark(m.matches);
    m.addEventListener("change", h);
    return () => m.removeEventListener("change", h);
  }, []);
  return dark;
}

export default function App() {
  const dark = usePrefersDark();
  const [entries, setEntries] = useState<Entry[]>([]);
  const [view, setView] = useState<"editor" | "triage" | "chat" | "tasks">(
    () => (localStorage.getItem("view") as "editor" | "triage" | "chat" | "tasks") || "editor",
  );
  const [dirty, setDirty] = useState(0);
  const [syncing, setSyncing] = useState(false);
  const [newNote, setNewNote] = useState(false);
  const [gettingStarted, setGettingStarted] = useState(false);
  const [settings, setSettings] = useState(false);
  // Config comes from the backend (which detects the vault layout); the app
  // waits for it rather than guessing defaults that could contradict Rust.
  const [cfg, setCfg] = useState<Config | null>(null);
  const cfgRef = useRef(cfg);
  cfgRef.current = cfg;

  useEffect(() => {
    getConfig().then(setCfg).catch(() => {});
  }, []);

  // Show onboarding on first run, or whenever the setup isn't ready.
  useEffect(() => {
    checkEnv()
      .then((env) => {
        if (!env.claude || !env.vault || !localStorage.getItem("onboarded")) {
          setGettingStarted(true);
        }
      })
      .catch(() => setGettingStarted(true));
  }, []);
  const [chat, setChat] = useState<Msg[]>([]);
  const [open, setOpen] = useState<string | null>(() => localStorage.getItem("open"));
  const [body, setBody] = useState("");
  const [cmdk, setCmdk] = useState(false);
  const [quickOpen, setQuickOpen] = useState(false);
  const [sidebarW, setSidebarW] = useState(() => Number(localStorage.getItem("sidebarW")) || 260);
  // A skill fired from the sidebar; the Chat view picks it up.
  const [chatCmd, setChatCmd] = useState<{ cmd: string; run: boolean } | null>(null);

  // Persist UI state across restarts.
  useEffect(() => localStorage.setItem("view", view), [view]);
  useEffect(() => localStorage.setItem("sidebarW", String(sidebarW)), [sidebarW]);
  useEffect(() => {
    if (open) localStorage.setItem("open", open);
  }, [open]);
  // Reopen the last note on launch.
  useEffect(() => {
    if (open)
      readNote(open)
        .then((text) => {
          savedBody.current = text;
          setBody(text);
        })
        .catch(() => setOpen(null));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const move = (ev: PointerEvent) => setSidebarW(Math.min(500, Math.max(160, ev.clientX)));
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const refresh = () => {
    readTree().then(setEntries).catch(console.error);
    gitStatus().then(setDirty).catch(() => {});
  };
  useEffect(() => {
    refresh();
  }, []);

  const sync = async () => {
    setSyncing(true);
    try {
      await gitSync();
    } catch (e) {
      console.error(e);
    } finally {
      setSyncing(false);
      refresh();
    }
  };

  const onDelete = async () => {
    if (!open || !cfg || !confirm(`Delete ${open}?`)) return;
    // Linked local files are offered separately — they may be worth keeping, and
    // anything another note references is kept regardless. Notes are never
    // collateral, only attachments.
    const assets = linkTargets(body).filter(
      (t) => !/^[a-z][a-z0-9+-]*:/i.test(t) && !t.endsWith(".md"),
    );
    const withAssets =
      assets.length > 0 &&
      confirm(
        `Delete the ${assets.length} file(s) this note links to as well?\n\n` +
          assets.join("\n"),
      );
    try {
      const res = await deleteNote(open, withAssets);
      // Every folder the delete emptied gets its own question.
      for (const dir of res.empty_dirs) {
        if (confirm(`${dir} is empty now. Delete the folder too?`)) await deleteDir(dir);
      }
    } catch (e) {
      console.error("delete failed:", e);
      alert(`Delete failed: ${e}`);
      return;
    }
    setOpen(null);
    setBody("");
    localStorage.removeItem("open");
    refresh();
  };

  const onCreate = async (path: string, body: string) => {
    await writeNote(path, body);
    refresh();
    openNote(path);
  };

  const openNote = async (rel: string) => {
    setOpen(rel);
    setView("editor");
    const text = await readNote(rel);
    savedBody.current = text;
    setBody(text);
  };

  // Cmd-K = claude palette, Cmd-P = quick open.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key === "k") {
          e.preventDefault();
          setCmdk((v) => !v);
        } else if (e.key === "p") {
          e.preventDefault();
          setQuickOpen((v) => !v);
        }
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  // Native macOS "Skills" menu → run the skill in Chat.
  useEffect(() => {
    const un = listen<string>("run-skill", (e) => {
      const s = cfgRef.current?.skills.find((x) => x.cmd === e.payload);
      setView("chat");
      setChatCmd({ cmd: e.payload, run: s ? !s.arg : true });
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Debounced autosave; autosync launchd commits every 30 min.
  const saveTimer = useRef<number | undefined>(undefined);
  // Last content written to disk, so a save can tell what links disappeared.
  const savedBody = useRef("");
  const onChange = (v: string) => {
    setBody(v);
    if (!open) return;
    clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(async () => {
      const before = savedBody.current;
      await writeNote(open, v);
      savedBody.current = v;
      // Attachment removed from the note → remove the file too (the backend
      // keeps anything still referenced elsewhere in the vault).
      const kept = new Set(linkTargets(v));
      const gone = linkTargets(before).filter((t) => !kept.has(t));
      if (!gone.length) return;
      try {
        if ((await pruneAttachments(open, gone)).length) refresh();
      } catch (e) {
        console.error("prune_attachments failed:", e);
      }
    }, 600);
  };

  async function runTriage() {
    return runNote("skill", "/inbox-triage");
  }

  if (!cfg) return <div className="empty">Loading…</div>;

  return (
    <div className="app">
      <div
        className="titlebar"
        data-tauri-drag-region
        onMouseDown={(e) => {
          if (e.buttons === 1) getCurrentWindow().startDragging();
        }}
      />
      <div className="workspace">
      <aside className="sidebar" style={{ flex: `0 0 ${sidebarW}px`, width: sidebarW }}>
        <div className="brand">
          <img src={logo} className="logo" alt="JK" />
          <span className="wordmark">
            Just Klauding <em>Notes</em>
          </span>
          <button className="new-btn" title="New note" onClick={() => setNewNote(true)}>
            ＋
          </button>
        </div>
        <div className="tabs">
          {(["editor", "triage", "chat", "tasks"] as const).map((v) => (
            <button key={v} className={view === v ? "on" : ""} onClick={() => setView(v)}>
              {v === "editor" ? "Notes" : v[0].toUpperCase() + v.slice(1)}
            </button>
          ))}
        </div>
        <Tree entries={entries} open={open} onOpen={openNote} />
        <div className="skills-side">
          <div className="skills-label">Skills</div>
          {cfg.skills.map((s) => (
            <button
              key={s.cmd}
              onClick={() => {
                setView("chat");
                setChatCmd({ cmd: s.cmd, run: !s.arg });
              }}
            >
              {s.label}
            </button>
          ))}
        </div>
        <div className="sidebar-foot">
          <button onClick={() => setQuickOpen(true)}>⌘P</button>
          <button onClick={() => setCmdk(true)}>⌘K</button>
          <button className="sync-btn" onClick={sync} disabled={syncing}>
            {syncing ? <Spinner /> : dirty > 0 ? `Sync (${dirty})` : "Synced"}
          </button>
          <button title="Settings" onClick={() => setSettings(true)}>
            ⚙
          </button>
          <button title="Getting started" onClick={() => setGettingStarted(true)}>
            ?
          </button>
        </div>
      </aside>

      <div className="resizer" onPointerDown={startResize} />

      <main className="main">
        {view === "editor" && (
          <Editor
            open={open}
            body={body}
            dark={dark}
            imageWidth={cfg.image_width}
            onChange={onChange}
            onDelete={onDelete}
            onOpenLink={(href) => {
              const p = open && resolveLink(open, href);
              if (p) openNote(p);
            }}
          />
        )}
        {view === "triage" && (
          <Triage entries={entries} cfg={cfg} onMoved={refresh} onRunTriage={runTriage} />
        )}
        {view === "chat" && (
          <Chat
            messages={chat}
            setMessages={setChat}
            skills={cfg.skills}
            onChanged={refresh}
            pending={chatCmd}
            onConsumed={() => setChatCmd(null)}
          />
        )}
        {view === "tasks" && <Tasks onOpen={openNote} archiveDays={cfg.archive_days} />}
      </main>
      </div>

      {cmdk && (
        <CmdK inboxDir={cfg.inbox_dir} onClose={() => setCmdk(false)} onDone={refresh} />
      )}
      {quickOpen && (
        <QuickOpen entries={entries} onOpen={openNote} onClose={() => setQuickOpen(false)} />
      )}
      {newNote && (
        <NewNote
          entries={entries}
          notesDir={cfg.notes_dir}
          onClose={() => setNewNote(false)}
          onCreate={onCreate}
        />
      )}
      {gettingStarted && (
        <GettingStarted
          onClose={() => {
            localStorage.setItem("onboarded", "1");
            setGettingStarted(false);
            refresh();
          }}
        />
      )}
      {settings && (
        <Settings
          cfg={cfg}
          onClose={() => setSettings(false)}
          onSaved={(next) => {
            setCfg(next);
            setSettings(false);
            refresh();
          }}
        />
      )}
    </div>
  );
}

// First-run onboarding: live readiness checklist + setup steps.
function GettingStarted({ onClose }: { onClose: () => void }) {
  const [env, setEnv] = useState<EnvCheck | null>(null);
  const recheck = () => checkEnv().then(setEnv).catch(() => setEnv(null));
  useEffect(() => {
    recheck();
  }, []);
  const chooseVault = async () => {
    const picked = await openDialog({ directory: true, title: "Choose your notes vault" });
    if (typeof picked === "string") {
      await setVault(picked);
      recheck();
    }
  };
  const Row = ({ ok, label }: { ok: boolean; label: string }) => (
    <div className={"gs-check" + (ok ? " ok" : "")}>
      <span className="gs-mark">{ok ? "✓" : "○"}</span>
      {label}
    </div>
  );
  const ready = env && env.claude && env.git && env.vault;
  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk gs" onClick={(e) => e.stopPropagation()}>
        <h2 className="gs-title">
          Welcome to Just Klauding <em>Notes</em>
        </h2>
        <p className="gs-sub">
          A native front-end for your markdown vault, powered by Claude Code. It needs a
          few things on your machine:
        </p>
        {env && (
          <div className="gs-checks">
            <Row ok={env.claude} label="Claude Code (`claude`) — installed & logged in" />
            <Row ok={env.git} label="git" />
            <Row ok={env.rg} label="ripgrep (`rg`) — search & tasks" />
            <div className={"gs-check gs-vault" + (env.vault ? " ok" : "")}>
              <span className="gs-mark">{env.vault ? "✓" : "○"}</span>
              <span className="gs-vault-path">{env.vault_path}</span>
              <button className="ghost" onClick={chooseVault}>
                Change…
              </button>
            </div>
          </div>
        )}
        <ol className="gs-steps">
          <li>
            Install Claude Code and run <code>claude login</code>.
          </li>
          <li>
            <code>brew install ripgrep git</code>
          </li>
          <li>
            Point the app at your vault with <b>Change…</b> above (saved to the app
            config), or copy the starter template to{" "}
            <code>{env?.vault_path ?? "~/Development/Notes"}</code>.
          </li>
          <li>Restart the app, hit ⌘K to ask, or the Skills menu to run a workflow.</li>
        </ol>
        <div className="cmdk-foot">
          <span className="gs-status">
            {ready ? "All set — you're good to go." : "Some steps are still pending."}
          </span>
          <button className="ghost" onClick={recheck}>
            Re-check
          </button>
          <button onClick={onClose}>Got it</button>
        </div>
      </div>
    </div>
  );
}

// Rotating nerd status words — the "it's working" cue, ported from _note_spin.
const SPIN_WORDS = [
  "Grepping the archives", "Summoning the vault", "Consulting the Oracle",
  "Reticulating splines", "Indexing memories", "Cross-referencing lore",
  "Triaging the inbox", "Parsing ancient scrolls", "Rolling for insight",
  "Compiling context", "Traversing the graph", "Decoding runes",
  "Filing under wisdom", "Warming the neurons", "Casting detect note",
  "Scanning the codex", "Untangling threads", "Aligning the flux capacitor",
];
const FRAMES = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

function Spinner() {
  const [i, setI] = useState(() => Math.floor(Math.random() * SPIN_WORDS.length * 15));
  useEffect(() => {
    const t = setInterval(() => setI((x) => x + 1), 120);
    return () => clearInterval(t);
  }, []);
  const word = SPIN_WORDS[Math.floor(i / 15) % SPIN_WORDS.length];
  return (
    <span className="spinner">
      {FRAMES[i % 10]} {word}…
    </span>
  );
}

function Tree({
  entries,
  open,
  onOpen,
}: {
  entries: Entry[];
  open: string | null;
  onOpen: (rel: string) => void;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(
    () => new Set(JSON.parse(localStorage.getItem("collapsed") || "[]")),
  );
  useEffect(() => {
    localStorage.setItem("collapsed", JSON.stringify([...collapsed]));
  }, [collapsed]);
  const toggle = (p: string) =>
    setCollapsed((s) => {
      const n = new Set(s);
      n.has(p) ? n.delete(p) : n.add(p);
      return n;
    });
  // An entry is hidden if any ancestor folder is collapsed.
  const hidden = (path: string) => [...collapsed].some((c) => path.startsWith(c + "/"));

  return (
    <div className="tree">
      {entries
        .filter((e) => !hidden(e.path))
        .map((e) => {
          const depth = e.path.split("/").length - 1;
          return e.is_dir ? (
            <div
              key={e.path}
              className="tree-dir"
              style={{ paddingLeft: depth * 12 }}
              onClick={() => toggle(e.path)}
            >
              <span className="chev">{collapsed.has(e.path) ? "▸" : "▾"}</span> {e.name}
              {e.has_assets && (
                <span className="has-assets" title="Contains an assets folder">
                  📎
                </span>
              )}
            </div>
          ) : (
            <div
              key={e.path}
              className={"tree-file" + (open === e.path ? " on" : "")}
              style={{ paddingLeft: depth * 12 + 14 }}
              onClick={() => onOpen(e.path)}
            >
              {e.name.replace(/\.md$/, "")}
            </div>
          );
        })}
    </div>
  );
}

// Split YAML frontmatter (flat key: value) from the markdown body.
function parseFrontmatter(src: string): { meta: [string, string][]; content: string } {
  const m = src.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!m) return { meta: [], content: src };
  const meta: [string, string][] = [];
  for (const line of m[1].split("\n")) {
    const i = line.indexOf(":");
    if (i > 0) meta.push([line.slice(0, i).trim(), line.slice(i + 1).trim()]);
  }
  return { meta, content: src.slice(m[0].length) };
}

// Rendered view by default; toggle to a raw-markdown CodeMirror editor.
function Editor({
  open,
  body,
  dark,
  imageWidth,
  onChange,
  onDelete,
  onOpenLink,
}: {
  open: string | null;
  body: string;
  dark: boolean;
  imageWidth: string;
  onChange: (v: string) => void;
  onDelete: () => void;
  onOpenLink: (href: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [dropping, setDropping] = useState(false);
  useEffect(() => {
    setEditing(false);
  }, [open]);

  // The drop listener lives for as long as the note does, so it reads the body
  // and change handler through refs instead of closing over stale ones.
  const bodyRef = useRef(body);
  bodyRef.current = body;
  const changeRef = useRef(onChange);
  changeRef.current = onChange;
  const cm = useRef<ReactCodeMirrorRef>(null);

  // Insert at the caret when the editor is open (CodeMirror owns the document
  // then, so go through its transaction API); otherwise append to the end.
  const insert = (text: string) => {
    const view = cm.current?.view;
    if (view) {
      const { from, to } = view.state.selection.main;
      const pad = from > 0 && view.state.doc.sliceString(from - 1, from) !== "\n" ? "\n" : "";
      view.dispatch({
        changes: { from, to, insert: pad + text + "\n" },
        selection: { anchor: from + pad.length + text.length + 1 },
      });
      view.focus();
      return;
    }
    changeRef.current(bodyRef.current.replace(/\s*$/, "") + "\n\n" + text + "\n");
  };

  // Dropped files are copied next to the note and linked into it: images inline,
  // everything else as a plain link (the preview gives those a file-type icon).
  useEffect(() => {
    if (!open) return;
    const un = getCurrentWebview().onDragDropEvent(async (e) => {
      if (e.payload.type === "enter" || e.payload.type === "over") return setDropping(true);
      if (e.payload.type === "leave") return setDropping(false);
      setDropping(false);
      const added: string[] = [];
      for (const path of e.payload.paths) {
        try {
          const rel = await attachFile(open, path);
          const name = path.split("/").pop() ?? rel;
          added.push(IMAGE_EXT.test(rel) ? `![${name}](${rel})` : `[${name}](${rel})`);
        } catch (err) {
          console.error("attach_file failed:", path, err);
        }
      }
      if (added.length) insert(added.join("\n"));
    });
    return () => {
      un.then((f) => f());
    };
  }, [open]);

  if (!open) return <div className="empty">Pick a note.</div>;

  const { meta, content } = parseFrontmatter(body);
  // Body line indices of every checkbox, in document order.
  const bodyLines = body.split("\n");
  const taskLines = bodyLines.reduce<number[]>((acc, l, i) => {
    if (/^\s*- \[[ xX]\]/.test(l)) acc.push(i);
    return acc;
  }, []);
  // Same `✅ <date>` stamping as the tasks view, so both paths agree.
  const flip = (i: number) => {
    const lines = body.split("\n");
    const l = lines[i];
    lines[i] = l.includes("- [ ]")
      ? l.replace("- [ ]", "- [x]") + (/✅\s*\d{4}-\d{2}-\d{2}/.test(l) ? "" : ` ✅ ${isoDate()}`)
      : l.replace(/- \[[xX]\]/, "- [ ]").replace(/\s*✅\s*\d{4}-\d{2}-\d{2}\s*$/, "");
    onChange(lines.join("\n"));
  };
  // Checkboxes render in the same order as taskLines; a per-render counter maps
  // each rendered box back to its source line.
  let taskIdx = 0;

  return (
    <div className={"editor" + (dropping ? " dropping" : "")}>
      <div className="editor-bar">
        <span className="path">{open}</span>
        <div className="editor-actions">
          <button onClick={() => setEditing((v) => !v)}>{editing ? "View" : "Edit"}</button>
          <button
            className="danger"
            onClick={onDelete}
          >
            Delete
          </button>
        </div>
      </div>
      {editing ? (
        <CodeMirror
          ref={cm}
          value={body}
          height="100%"
          className="cm-pane"
          extensions={[markdown()]}
          onChange={onChange}
          theme={dark ? "dark" : "light"}
        />
      ) : (
        <div
          className="preview markdown"
          style={{ "--img-w": imageWidth } as React.CSSProperties}
        >
          {meta.length > 0 && (
            <div className="fm-card">
              {meta.map(([k, v]) =>
                v ? (
                  <div className="fm-row" key={k}>
                    <span className="fm-key">{k}</span>
                    <span className="fm-val">{v}</span>
                  </div>
                ) : null,
              )}
            </div>
          )}
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
              a: ({ href, children }) => (
                <a
                  href={href}
                  className={href && !resolveLinkable(href) ? "ext" : undefined}
                  onClick={(e) => {
                    if (!href) return;
                    e.preventDefault();
                    if (resolveLinkable(href)) return onOpenLink(href);
                    // Anything else leaves the app: URLs to the browser, vault
                    // files to their default app.
                    const target = /^[a-z][a-z0-9+-]*:/i.test(href)
                      ? href
                      : joinRel(open, decodeURIComponent(href.split("#")[0]));
                    openExternal(target).catch((err) => console.error("open failed:", err));
                  }}
                >
                  {children}
                </a>
              ),
              img: ({ src, alt }) => {
                const { alt: text, width } = splitAlt(alt ?? "");
                if (typeof src !== "string") return null;
                // Remote and inline sources need no vault round-trip.
                return /^(https?:|data:|blob:)/i.test(src) ? (
                  <img src={src} alt={text} style={width ? { maxWidth: width } : undefined} />
                ) : (
                  <VaultImage rel={joinRel(open, decodeURIComponent(src))} alt={text} width={width} />
                );
              },
              input: ({ type, checked }) =>
                type === "checkbox" ? (
                  (() => {
                    const line = taskLines[taskIdx++];
                    return (
                      <input
                        type="checkbox"
                        checked={!!checked}
                        onChange={() => flip(line)}
                      />
                    );
                  })()
                ) : (
                  <input type={type} defaultChecked={checked} />
                ),
            }}
          >
            {content}
          </ReactMarkdown>
        </div>
      )}
    </div>
  );
}

// Cheap check so the link component doesn't hijack external links.
function resolveLinkable(href: string) {
  return !/^[a-z]+:\/\//i.test(href) && decodeURIComponent(href.split("#")[0]).endsWith(".md");
}

// Cmd-P: fuzzy quick-open by filename + debounced full-text content search.
function QuickOpen({
  entries,
  onOpen,
  onClose,
}: {
  entries: Entry[];
  onOpen: (rel: string) => void;
  onClose: () => void;
}) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  const files = entries.filter((e) => !e.is_dir);
  const nameMatches = (
    q ? files.filter((f) => f.path.toLowerCase().includes(q.toLowerCase())) : files
  ).slice(0, 30);

  useEffect(() => {
    if (!q.trim()) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => grep(q).then(setHits).catch(() => setHits([])), 250);
    return () => clearTimeout(t);
  }, [q]);

  const open = (p: string) => {
    onOpen(p);
    onClose();
  };

  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="target-input"
          placeholder="Open note by name / search content…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && nameMatches[0]) open(nameMatches[0].path);
            if (e.key === "Escape") onClose();
          }}
        />
        <div className="qo-results">
          {nameMatches.map((f) => (
            <div key={f.path} className="qo-file" onClick={() => open(f.path)}>
              {f.path}
            </div>
          ))}
          {hits.length > 0 && <div className="qo-sep">content matches</div>}
          {hits.map((h, idx) => (
            <div key={idx} className="qo-hit" onClick={() => open(h.path)}>
              <b>
                {h.path}:{h.line}
              </b>{" "}
              {h.text}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// Triage: pick an inbox note, filter/Suggest the target folder, Enter to file.
function Triage({
  entries,
  cfg,
  onMoved,
  onRunTriage,
}: {
  entries: Entry[];
  cfg: Config;
  onMoved: () => void;
  onRunTriage: () => Promise<string>;
}) {
  const [busy, setBusy] = useState(false);
  const [out, setOut] = useState("");
  const [sel, setSel] = useState<string | null>(null);
  const [selBody, setSelBody] = useState("");
  const [q, setQ] = useState("");
  const [suggesting, setSuggesting] = useState(false);

  const inbox = entries.filter((e) => !e.is_dir && e.path.startsWith(cfg.inbox_dir + "/"));
  // Every folder under the configured roots, at any depth — projects nested two
  // levels deep are as valid a target as top-level ones.
  const targets = useMemo(() => {
    const under = (dir: string) =>
      entries.filter((e) => e.is_dir && e.path.startsWith(dir + "/")).map((e) => e.path);
    return [
      ...under(cfg.projects_dir),
      cfg.people_dir,
      ...under(cfg.people_dir),
      cfg.notes_dir,
      ...under(cfg.notes_dir),
    ].sort();
  }, [entries, cfg]);
  const matches = targets.filter((t) => t.toLowerCase().includes(q.toLowerCase()));

  const select = async (p: string) => {
    setSel(p);
    setQ("");
    setSelBody(await readNote(p));
  };
  const move = async (target: string) => {
    if (!sel) return;
    const file = sel.split("/").pop()!;
    await moveNote(sel, `${target}/${file}`);
    setSel(null);
    setSelBody("");
    onMoved();
  };
  // Ask claude which folder this note belongs in (uses vault alias map).
  const suggest = async () => {
    setSuggesting(true);
    try {
      const prompt =
        "This note will be filed into exactly one folder. Reply with ONLY one " +
        "exact path from this list, nothing else:\n" +
        targets.join("\n") +
        "\n\nNote content:\n" +
        selBody;
      const r = (await runNote("ask", prompt)).trim().split("\n").pop() ?? "";
      setQ(r.trim());
    } finally {
      setSuggesting(false);
    }
  };

  return (
    <div className="triage">
      <div className="triage-head">
        <span>{inbox.length} in inbox</span>
        <button
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            setOut("");
            try {
              setOut(await onRunTriage());
            } finally {
              setBusy(false);
              onMoved();
            }
          }}
        >
          {busy ? <Spinner /> : "Auto-triage all (claude)"}
        </button>
      </div>

      <div className="triage-body">
        <div className="inbox-list">
          {inbox.length === 0 && <div className="empty">Inbox empty.</div>}
          {inbox.map((e) => (
            <div
              key={e.path}
              className={"inbox-item" + (sel === e.path ? " on" : "")}
              onClick={() => select(e.path)}
            >
              {e.name}
            </div>
          ))}
        </div>

        <div className="triage-detail">
          {!sel ? (
            <div className="empty">Select an inbox note to file it.</div>
          ) : (
            <>
              <div className="target-row">
                <input
                  autoFocus
                  className="target-input"
                  placeholder="Filter target folder…  (Enter = top match)"
                  value={q}
                  onChange={(e) => setQ(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && matches[0]) move(matches[0]);
                  }}
                />
                <button disabled={suggesting} onClick={suggest}>
                  {suggesting ? <Spinner /> : "Suggest"}
                </button>
              </div>
              <div className="target-list">
                {matches.map((t) => (
                  <button key={t} className="target" onClick={() => move(t)}>
                    {t}
                  </button>
                ))}
              </div>
              <pre className="note-preview">{selBody}</pre>
            </>
          )}
        </div>
      </div>

      {out && <pre className="triage-out">{out}</pre>}
    </div>
  );
}

// Multi-turn chat with the vault. Plain messages -> `ask` (--continue after
// the first turn keeps context). A message starting with "/" runs a vault
// skill instead. Skill buttons prefill the input.
function Chat({
  messages,
  setMessages,
  skills,
  onChanged,
  pending,
  onConsumed,
}: {
  messages: Msg[];
  setMessages: React.Dispatch<React.SetStateAction<Msg[]>>;
  skills: { label: string; cmd: string; arg: boolean }[];
  onChanged: () => void;
  pending: { cmd: string; run: boolean } | null;
  onConsumed: () => void;
}) {
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [messages, busy]);

  // A skill fired from the sidebar: run it, or prefill the input for arg skills.
  useEffect(() => {
    if (!pending) return;
    if (pending.run) send(pending.cmd);
    else {
      setInput(pending.cmd + " ");
      inputRef.current?.focus();
    }
    onConsumed();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending]);

  const send = async (explicit?: string) => {
    const text = (explicit ?? input).trim();
    if (!text || busy) return;
    const isSkill = text.startsWith("/");
    // cont only for plain ask turns; skills are separate invocations.
    const cont = !isSkill && messages.some((m) => m.role === "user");
    setMessages((m) => [...m, { role: "user", text }]);
    setInput("");
    setBusy(true);
    try {
      const reply = isSkill
        ? await runNote("skill", text)
        : await runNote("ask", text, cont);
      setMessages((m) => [...m, { role: "claude", text: reply.trim() || "(no output)" }]);
      if (isSkill) onChanged(); // skills may write files
    } catch (e) {
      setMessages((m) => [...m, { role: "claude", text: String(e) }]);
    } finally {
      setBusy(false);
    }
  };

  const pickSkill = (s: { cmd: string; arg: boolean }) => {
    if (s.arg) {
      setInput(s.cmd + " ");
      inputRef.current?.focus();
    } else {
      send(s.cmd);
    }
  };

  return (
    <div className="chat">
      <div className="chat-head">
        <div className="skill-bar">
          {skills.map((s) => (
            <button key={s.cmd} disabled={busy} onClick={() => pickSkill(s)}>
              {s.label}
            </button>
          ))}
        </div>
        <button className="ghost" disabled={busy} onClick={() => setMessages([])}>
          New chat
        </button>
      </div>

      <div className="chat-log" ref={logRef}>
        {messages.length === 0 && (
          <div className="empty">Ask the vault, or run a skill above.</div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={"msg " + m.role}>
            <div className="msg-body markdown">
              <ReactMarkdown>{m.text}</ReactMarkdown>
            </div>
          </div>
        ))}
        {busy && (
          <div className="msg claude">
            <div className="msg-body">
              <Spinner />
            </div>
          </div>
        )}
      </div>

      <div className="chat-input">
        <textarea
          ref={inputRef}
          value={input}
          placeholder="Ask… (Enter to send, Shift+Enter = newline, / for a skill)"
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
        />
      </div>
    </div>
  );
}

function slugify(s: string): string {
  return (
    s
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^\w\s-]/g, "")
      .trim()
      .replace(/\s+/g, "-")
      .slice(0, 50) || "note"
  );
}

// Global task list: every `- [ ]` matching the configured task glob, grouped by
// project. Ticking writes the flip back to the file.
const taskKey = (t: Task) => `${t.file}:${t.line}`;

function Tasks({
  onOpen,
  archiveDays,
}: {
  onOpen: (rel: string) => void;
  archiveDays: number;
}) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [showDone, setShowDone] = useState(false);
  // Ticked in this session: stays put in its project group so the list doesn't
  // jump under the cursor. Moves down to Done on the next load.
  const [sticky, setSticky] = useState<Set<string>>(new Set());
  const load = () => {
    listTasks()
      .then((ts) => {
        setTasks(ts);
        setSticky(new Set());
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  };
  useEffect(load, []);

  // Open tasks grouped by project; recently done ones collected separately.
  // Anything finished before the cutoff (or undated — it predates stamping)
  // stays in the file but drops out of the list, so Done can't grow forever.
  const { groups, done, hidden } = useMemo(() => {
    const cutoff = isoDaysAgo(archiveDays);
    const m = new Map<string, Task[]>();
    const done: Task[] = [];
    let hidden = 0;
    for (const t of tasks) {
      if (t.done && !sticky.has(taskKey(t))) {
        if (!t.done_at || t.done_at < cutoff) hidden++;
        else done.push(t);
        continue;
      }
      if (!m.has(t.group)) m.set(t.group, []);
      m.get(t.group)!.push(t);
    }
    return { groups: [...m.entries()], done, hidden };
  }, [tasks, sticky, archiveDays]);

  // Optimistic: flip locally, no reload, so nothing re-mounts on tick.
  const toggle = async (t: Task) => {
    const k = taskKey(t);
    const set = (patch: Partial<Task>) =>
      setTasks((prev) => prev.map((x) => (taskKey(x) === k ? { ...x, ...patch } : x)));
    set({ done: !t.done, done_at: t.done ? null : isoDate() });
    setSticky((prev) => new Set(prev).add(k));
    try {
      await toggleTask(t.file, t.line);
    } catch (e) {
      console.error("toggle_task failed:", e);
      set({ done: t.done, done_at: t.done_at });
    }
  };

  const openCount = tasks.filter((t) => !t.done).length;
  const row = (t: Task) => (
    <label key={taskKey(t)} className={"task" + (t.done ? " done" : "")}>
      <input type="checkbox" checked={t.done} onChange={() => toggle(t)} />
      <span>{t.text}</span>
      {t.done && t.done_at && <span className="task-date">{t.done_at}</span>}
    </label>
  );

  return (
    <div className="tasks">
      <div className="tasks-head">
        <span>
          {openCount} open · {groups.length} project(s) · {done.length} done
          {hidden > 0 && ` · ${hidden} archived`}
        </span>
        <button onClick={load}>Refresh</button>
      </div>
      <div className="tasks-list">
        {loading && <div className="empty">Loading…</div>}
        {!loading && tasks.length === 0 && <div className="empty">No tasks.</div>}
        {groups.map(([proj, ts]) => (
          <div key={proj} className="task-group">
            <div className="task-proj" onClick={() => onOpen(ts[0].file)}>
              {proj}
            </div>
            {ts.map(row)}
          </div>
        ))}
        {done.length > 0 && (
          <div className="task-group task-done-group">
            <button className="task-done-toggle" onClick={() => setShowDone((v) => !v)}>
              {showDone ? "▾" : "▸"} Done, last {archiveDays}d ({done.length})
            </button>
            {showDone && done.map(row)}
          </div>
        )}
      </div>
    </div>
  );
}

// Create a new note: pick a folder, type a title.
function NewNote({
  entries,
  notesDir,
  onClose,
  onCreate,
}: {
  entries: Entry[];
  notesDir: string;
  onClose: () => void;
  onCreate: (path: string, body: string) => void;
}) {
  const dirs = useMemo(() => entries.filter((e) => e.is_dir).map((e) => e.path), [entries]);
  const [dir, setDir] = useState(dirs.includes(notesDir) ? notesDir : dirs[0] ?? "");
  const [name, setName] = useState("");
  const create = () => {
    const t = name.trim();
    if (!t || !dir) return;
    onCreate(`${dir}/${slugify(t)}.md`, `# ${t}\n`);
    onClose();
  };
  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk" onClick={(e) => e.stopPropagation()}>
        <select className="target-input" value={dir} onChange={(e) => setDir(e.target.value)}>
          {dirs.map((d) => (
            <option key={d} value={d}>
              {d}
            </option>
          ))}
        </select>
        <input
          autoFocus
          className="target-input"
          placeholder="Note title…"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") create();
            if (e.key === "Escape") onClose();
          }}
        />
        <div className="cmdk-foot">
          <span className="newnote-preview">
            {dir}/{name.trim() ? slugify(name) : "…"}.md
          </span>
          <button onClick={create}>Create</button>
        </div>
      </div>
    </div>
  );
}

// Settings: edit the whole app config (written to config.json).
type StrKey =
  | "model"
  | "projects_dir"
  | "people_dir"
  | "notes_dir"
  | "inbox_dir"
  | "tasks_file"
  | "task_glob"
  | "note_language"
  | "attachments_dir"
  | "image_width"
  | "transcripts_dir";

function Settings({
  cfg,
  onClose,
  onSaved,
}: {
  cfg: Config;
  onClose: () => void;
  onSaved: (c: Config) => void;
}) {
  const [d, setD] = useState<Config>(cfg);
  const [hiddenText, setHiddenText] = useState(cfg.hidden_folders.join(", "));
  const [saving, setSaving] = useState(false);
  const up = (patch: Partial<Config>) => setD((p) => ({ ...p, ...patch }));
  const setSkill = (i: number, patch: Partial<Config["skills"][number]>) =>
    up({ skills: d.skills.map((s, j) => (j === i ? { ...s, ...patch } : s)) });

  const chooseVault = async () => {
    const picked = await openDialog({ directory: true, title: "Choose your notes vault" });
    if (typeof picked === "string") up({ vault: picked });
  };

  // Fill the form from what the vault actually contains; the user still has to
  // hit Save, so nothing is written behind their back.
  const redetect = async () => {
    try {
      const det = await detectConfig();
      up({
        projects_dir: det.projects_dir,
        people_dir: det.people_dir,
        notes_dir: det.notes_dir,
        inbox_dir: det.inbox_dir,
        tasks_file: det.tasks_file,
        task_glob: det.task_glob,
        hidden_folders: det.hidden_folders,
      });
      setHiddenText(det.hidden_folders.join(", "));
    } catch (e) {
      console.error("detect_config failed:", e);
    }
  };

  const field = (key: StrKey, label: string) => (
    <label className="set-row">
      <span>{label}</span>
      <input
        className="target-input"
        value={d[key]}
        onChange={(e) => up({ [key]: e.target.value } as Partial<Config>)}
      />
    </label>
  );

  const save = async () => {
    setSaving(true);
    const final: Config = {
      ...d,
      hidden_folders: hiddenText
        .split(/[,\n]+/)
        .map((s) => s.trim())
        .filter(Boolean),
    };
    try {
      await saveConfig(final);
      onSaved(final);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk settings" onClick={(e) => e.stopPropagation()}>
        <div className="set-title">
          <h2 className="gs-title">Settings</h2>
          <span className="set-version">v{useAppVersion()}</span>
        </div>
        <div className="set-grid">
          <label className="set-row">
            <span>Vault</span>
            <div className="set-vault">
              <input
                className="target-input"
                value={d.vault}
                onChange={(e) => up({ vault: e.target.value })}
              />
              <button className="ghost" onClick={chooseVault}>
                Change…
              </button>
            </div>
          </label>
          {field("model", "Claude model")}
          {field("note_language", "Note language")}
          {field("projects_dir", "Projects dir")}
          {field("people_dir", "People dir")}
          {field("notes_dir", "Notes dir")}
          {field("inbox_dir", "Inbox dir")}
          {field("tasks_file", "Tasks file")}
          {field("task_glob", "Tasks glob")}
          {field("transcripts_dir", "Transcripts dir")}
          {field("attachments_dir", "Attachments dir (per note)")}
          {field("image_width", "Image width")}
          <label className="set-row">
            <span>Keep done tasks (days)</span>
            <input
              className="target-input"
              type="number"
              min={0}
              value={d.archive_days}
              onChange={(e) => up({ archive_days: Math.max(0, Number(e.target.value) || 0) })}
            />
          </label>
          <label className="set-row">
            <span>Hidden folders</span>
            <input
              className="target-input"
              value={hiddenText}
              placeholder="comma-separated"
              onChange={(e) => setHiddenText(e.target.value)}
            />
          </label>
        </div>

        <div className="set-skills">
          <div className="skills-label">Skills</div>
          {d.skills.map((s, i) => (
            <div className="skill-edit" key={i}>
              <input
                className="target-input"
                value={s.label}
                placeholder="label"
                onChange={(e) => setSkill(i, { label: e.target.value })}
              />
              <input
                className="target-input skill-cmd"
                value={s.cmd}
                placeholder="/command"
                onChange={(e) => setSkill(i, { cmd: e.target.value })}
              />
              <label className="skill-arg" title="Prefill the input instead of running immediately">
                <input
                  type="checkbox"
                  checked={s.arg}
                  onChange={(e) => setSkill(i, { arg: e.target.checked })}
                />
                arg
              </label>
              <button className="ghost" onClick={() => up({ skills: d.skills.filter((_, j) => j !== i) })}>
                ✕
              </button>
            </div>
          ))}
          <button
            className="ghost add-skill"
            onClick={() => up({ skills: [...d.skills, { label: "", cmd: "/", arg: false }] })}
          >
            + Add skill
          </button>
        </div>

        <div className="cmdk-foot">
          <span className="gs-status">Applies on save; the native Skills menu updates after restart.</span>
          <button className="ghost" onClick={redetect}>
            Re-detect layout
          </button>
          <button className="ghost" onClick={onClose}>
            Cancel
          </button>
          <button disabled={saving} onClick={save}>
            {saving ? <Spinner /> : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}

// Cmd-K palette: ask / ai / capture with a live status spinner.
function CmdK({
  inboxDir,
  onClose,
  onDone,
}: {
  inboxDir: string;
  onClose: () => void;
  onDone: () => void;
}) {
  const [kind, setKind] = useState<"ask" | "ai" | "capture">("ask");
  const [text, setText] = useState("");
  const [cont, setCont] = useState(false);
  const [busy, setBusy] = useState(false);
  const [out, setOut] = useState("");

  const run = async () => {
    if (!text.trim()) return;
    setBusy(true);
    setOut("");
    try {
      if (kind === "capture") {
        await writeNote(inboxName(text, inboxDir), text + "\n");
        setOut(await runNote("skill", "/inbox-triage"));
      } else {
        setOut(await runNote(kind, text, cont));
      }
      onDone();
    } catch (e) {
      setOut(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk" onClick={(e) => e.stopPropagation()}>
        <div className="cmdk-kinds">
          {(["ask", "ai", "capture"] as const).map((k) => (
            <button key={k} className={kind === k ? "on" : ""} onClick={() => setKind(k)}>
              {k}
            </button>
          ))}
          {kind === "ask" && (
            <label className="cont">
              <input type="checkbox" checked={cont} onChange={(e) => setCont(e.target.checked)} />
              continue
            </label>
          )}
        </div>
        <textarea
          autoFocus
          value={text}
          placeholder={
            kind === "ask"
              ? "Ask the vault…"
              : kind === "ai"
                ? "Note for claude to file…"
                : "Quick capture → inbox → triage"
          }
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "Enter") run();
          }}
        />
        <div className="cmdk-foot">
          {busy && <Spinner />}
          <button disabled={busy} onClick={run}>
            Run (⌘⏎)
          </button>
        </div>
        {out && <pre className="cmdk-out">{out}</pre>}
      </div>
    </div>
  );
}
