import { invoke } from "@tauri-apps/api/core";

export type Entry = { path: string; name: string; is_dir: boolean };
export type Hit = { path: string; line: number; text: string };
export type Task = { file: string; line: number; text: string; done: boolean };

export const readTree = () => invoke<Entry[]>("read_tree");
export const grep = (query: string) => invoke<Hit[]>("grep", { query });
export const readNote = (rel: string) => invoke<string>("read_note", { rel });
export const writeNote = (rel: string, body: string) =>
  invoke<void>("write_note", { rel, body });
export const moveNote = (from: string, to: string) =>
  invoke<void>("move_note", { from, to });
export const deleteNote = (rel: string) => invoke<void>("delete_note", { rel });
export const runNote = (kind: string, text: string, cont = false) =>
  invoke<string>("run_note", { kind, text, cont });
export const gitStatus = () => invoke<number>("git_status");
export const gitSync = () => invoke<string>("git_sync");
export const listTasks = () => invoke<Task[]>("list_tasks");
export const toggleTask = (file: string, line: number) =>
  invoke<boolean>("toggle_task", { file, line });

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
  transcripts_dir: string;
  model: string;
  skills: Skill[];
};
export const getConfig = () => invoke<Config>("get_config");
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
