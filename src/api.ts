import { invoke } from "@tauri-apps/api/core";

export type Entry = {
  path: string;
  name: string;
  is_dir: boolean;
  // Dirs only: an attachments folder lives inside (the tree hides those).
  has_assets: boolean;
};
export type Hit = { path: string; line: number; text: string };
export type Task = {
  file: string;
  line: number;
  text: string;
  done: boolean;
  done_at: string | null;
  // Heading the task is listed under, computed by the backend from the path.
  group: string;
};

// Local calendar date, ISO. Dates live in the vault as `✅ YYYY-MM-DD`, so they
// must follow the user's timezone, not UTC.
export const isoDate = (d = new Date()) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
export const isoDaysAgo = (days: number) =>
  isoDate(new Date(Date.now() - days * 864e5));

export const readTree = () => invoke<Entry[]>("read_tree");
export const grep = (query: string) => invoke<Hit[]>("grep", { query });
export const readNote = (rel: string) => invoke<string>("read_note", { rel });
export const writeNote = (rel: string, body: string) =>
  invoke<void>("write_note", { rel, body });
export const moveNote = (from: string, to: string) =>
  invoke<void>("move_note", { from, to });
export type DeleteResult = { deleted_assets: string[]; empty_dirs: string[] };
// Files that deleting this note would take with it (read off disk, not the editor).
export const deletePlan = (rel: string) => invoke<string[]>("delete_plan", { rel });
export const deleteNote = (rel: string, withAssets: boolean) =>
  invoke<DeleteResult>("delete_note", { rel, withAssets });
// Removes a folder only if it is empty.
export const deleteDir = (rel: string) => invoke<boolean>("delete_dir", { rel });
export const runNote = (kind: string, text: string, cont = false) =>
  invoke<string>("run_note", { kind, text, cont });
// Copies `src` next to `note` and returns the path to put in the markdown link.
export const attachFile = (note: string, src: string) =>
  invoke<string>("attach_file", { note, src });
// Vault image as a data URI (see read_asset: 20 MB cap, images only).
export const readAsset = (rel: string) => invoke<string>("read_asset", { rel });
// URL → browser, vault file → whatever app owns the type.
export const openExternal = (target: string) => invoke<void>("open_external", { target });
// Deletes attachments the note stopped linking to; returns what went.
export const pruneAttachments = (note: string, removed: string[]) =>
  invoke<string[]>("prune_attachments", { note, removed });
export const gitStatus = () => invoke<number>("git_status");
export const gitSync = () => invoke<string>("git_sync");
export const listTasks = () => invoke<Task[]>("list_tasks");
export const toggleTask = (file: string, line: number) =>
  invoke<boolean>("toggle_task", { file, line, today: isoDate() });

export type EnvCheck = {
  claude: boolean;
  rg: boolean;
  git: boolean;
  vault: boolean;
  vault_path: string;
};
export const checkEnv = () => invoke<EnvCheck>("check_env");
export const setVault = (path: string) => invoke<void>("set_vault", { path });

export type Skill = { label: string; cmd: string; arg: boolean };
export type Config = {
  vault: string;
  hidden_folders: string[];
  projects_dir: string;
  people_dir: string;
  notes_dir: string;
  inbox_dir: string;
  tasks_file: string;
  task_glob: string;
  transcripts_dir: string;
  // Per-note folder dropped files are copied into, relative to the note.
  attachments_dir: string;
  // Default max width for images in the preview, any CSS length (e.g. "50%").
  image_width: string;
  model: string;
  // Language Claude writes note content in; empty = mirror the request.
  note_language: string;
  archive_days: number;
  skills: Skill[];
};
export const getConfig = () => invoke<Config>("get_config");
// Layout detected from the vault's actual contents; writes nothing.
export const detectConfig = () => invoke<Config>("detect_config");
export const saveConfig = (config: Config) => invoke<void>("save_config", { config });

// Czech-friendly inbox slug (strip diacritics, keep ascii words).
export function inboxName(text: string, dir = "inbox"): string {
  const now = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  const ts = `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}-${p(now.getHours())}${p(now.getMinutes())}`;
  const slug =
    text
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^\w\s-]/g, "")
      .trim()
      .replace(/\s+/g, "-")
      .slice(0, 40) || "note";
  return `${dir}/${ts}-${slug}.md`;
}
