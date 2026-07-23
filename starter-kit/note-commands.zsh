# Just Klauding Notes — shell commands
# Source this from your ~/.zshrc:   source /path/to/note-commands.zsh
# Mirrors the app's capture / ask / ai actions on the command line.
#
# Vault location (override by exporting NOTES_VAULT before sourcing):
: "${NOTES_VAULT:=$HOME/Development/Notes}"

# Spinner shown while a silent `claude -p` runs (TTY only).
_note_spin() {
  local pid=$1
  RANDOM=$(( $(date +%s) ^ $$ ))
  local words=(
    "Grepping the archives" "Summoning the vault" "Consulting the Oracle"
    "Indexing memories" "Cross-referencing lore" "Parsing ancient scrolls"
    "Rolling for insight" "Compiling context" "Decoding runes"
    "Filing under wisdom" "Casting detect note" "Scanning the codex"
  )
  local frames='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
  local i=0 w=${words[$((RANDOM % ${#words[@]} + 1))]}
  while kill -0 "$pid" 2>/dev/null; do
    printf '\r\e[2m%s %s…\e[0m\e[K' "${frames:$((i % 10)):1}" "$w" >&2
    (( ++i % 30 == 0 )) && w=${words[$((RANDOM % ${#words[@]} + 1))]}
    sleep 0.1
  done
  printf '\r\e[2K' >&2
}

_note_claude() {
  setopt localoptions no_notify no_monitor
  if [[ ! -t 1 ]]; then claude "$@"; return; fi
  local out; out=$(mktemp)
  ( cd "$NOTES_VAULT" && claude "$@" ) > "$out" 2>&1 &
  local pid=$!
  _note_spin "$pid"
  wait "$pid"; local rc=$?
  cat "$out"; rm -f "$out"
  return $rc
}

# note                 → attach nothing; prints usage
# note <text>          → instant capture to inbox/ + commit
# note ai <text>       → claude files the note in the right place
# note ask [-c] <q>    → Q&A over the vault (-c = continue previous chat)
note() {
  local flags=(--model sonnet --strict-mcp-config --disable-slash-commands --setting-sources project)
  case "$1" in
    ai)
      shift
      _note_claude -p "${flags[@]}" --permission-mode acceptEdits \
        --allowedTools "Bash(git:*),Bash(date:*)" "$*"
      ;;
    ask)
      shift
      local cont=()
      [[ "$1" == "-c" ]] && { cont=(--continue); shift; }
      _note_claude -p "${flags[@]}" "${cont[@]}" "$*"
      ;;
    "")
      echo "usage: note <text> | note ai <text> | note ask [-c] <question>"
      ;;
    *)
      # instant capture
      local slug; slug=$(printf '%s' "$*" | iconv -t ascii//TRANSLIT 2>/dev/null \
        | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | sed 's/^-//;s/-$//' | cut -c1-40)
      [[ -z "$slug" ]] && slug=note
      local f="$NOTES_VAULT/inbox/$(date +%Y-%m-%d-%H%M)-$slug.md"
      printf '%s\n' "$*" > "$f"
      ( cd "$NOTES_VAULT" && git add "$f" && git commit -qm "note: $slug" && git push -q ) 2>/dev/null
      echo "$f"
      ;;
  esac
}
