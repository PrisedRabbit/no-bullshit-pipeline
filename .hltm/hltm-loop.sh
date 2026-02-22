#!/usr/bin/env bash
# hltm-loop — fresh-context agent loop. tool & methodology agnostic.
set -uo pipefail

# ── docker passthrough ───────────────────────────────────────────────
if [ "${1:-}" = "--docker" ]; then
  shift
  SELF_DIR="$(cd "$(dirname "$0")" && pwd)"
  # when installed: .hltm-loop → .hltm/scripts/docker.py
  # when running from source: cli-agent/hltm-loop.sh → cli-agent/scripts/docker.py
  if [ -f "$SELF_DIR/.hltm/scripts/docker.py" ]; then
    exec python3 "$SELF_DIR/.hltm/scripts/docker.py" "$@"
  elif [ -f "$SELF_DIR/scripts/docker.py" ]; then
    exec python3 "$SELF_DIR/scripts/docker.py" "$@"
  else
    echo "error: docker.py not found" >&2; exit 1
  fi
fi

# ── colors ────────────────────────────────────────────────────────────
if [ -t 1 ]; then
  C_RESET='\033[0m'  C_BOLD='\033[1m'  C_DIM='\033[2m'
  C_RED='\033[31m'   C_GREEN='\033[32m' C_YELLOW='\033[33m'
  C_MAGENTA='\033[35m' C_CYAN='\033[36m'
else
  C_RESET='' C_BOLD='' C_DIM=''
  C_RED='' C_GREEN='' C_YELLOW=''
  C_MAGENTA='' C_CYAN=''
fi

# ── defaults ──────────────────────────────────────────────────────────
EXECUTOR=""
MODEL=""
PROMPTS=()
MAX=""
VERBOSE=0
HUMAN_BLOCK=0
declare -A STAGE_EXEC STAGE_MODEL

# ── parse args ────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help)
      cat <<'EOF'
usage: .hltm-loop [--docker] -m <model> -p <prompt> [-p ...] -e <executor> -n <max> [options]

required:
  -m, --model <name>        model (opus, sonnet, o3, ...)
  -p, --prompt <file|text>  prompt file or inline text (repeatable)
  -e, --executor <tool>     claude-code, codex
  -n, --max-rounds <n>      max iterations (0 = unlimited)

options:
  --docker                  run inside Docker container
  -v, --verbose             stream agent output live
  --human-block             stop on <loop:human> signals
  --stage <name:exec[:model]>  executor/model override per stage (repeatable)

examples:
  .hltm-loop -m sonnet -p .hltm/prompts/dev.md -e claude-code --stage review:codex -n 20
  .hltm-loop --docker -m sonnet -p .hltm/prompts/dev.md -e claude-code -n 20
EOF
      exit 0
      ;;
    -m|--model) MODEL="$2"; shift 2 ;;
    -p|--prompt) PROMPTS+=("$2"); shift 2 ;;
    -e|--executor) EXECUTOR="$2"; shift 2 ;;
    -n|--max-rounds) MAX="$2"; shift 2 ;;
    -v|--verbose) VERBOSE=1; shift ;;
    --human-block) HUMAN_BLOCK=1; shift ;;
    --stage)
      IFS=: read -r sname sexec smodel <<< "$2"
      STAGE_EXEC[$sname]="$sexec"
      STAGE_MODEL[$sname]="${smodel:-}"
      shift 2
      ;;
    *)
      printf "${C_RED}error:${C_RESET} unknown option '%s'\n" "$1" >&2
      exit 1
      ;;
  esac
done

# ── validate ──────────────────────────────────────────────────────────
MISSING=()
[ -z "$MODEL" ] && MISSING+=("-m")
[ ${#PROMPTS[@]} -eq 0 ] && MISSING+=("-p")
[ -z "$EXECUTOR" ] && MISSING+=("-e")
[ -z "$MAX" ] && MISSING+=("-n")

if [ ${#MISSING[@]} -gt 0 ]; then
  printf "${C_RED}error:${C_RESET} missing required: %s\n" "${MISSING[*]}" >&2
  echo "run hltm-loop --help" >&2
  exit 1
fi

case "$EXECUTOR" in
  claude-code|codex) ;;
  *) printf "${C_RED}error:${C_RESET} unknown executor '%s' (claude-code, codex)\n" "$EXECUTOR" >&2; exit 1 ;;
esac

# ── prompt display ────────────────────────────────────────────────────
PROMPT_DISPLAY=""
for p in "${PROMPTS[@]}"; do
  if [ -f "$p" ]; then
    PROMPT_DISPLAY="${PROMPT_DISPLAY:+$PROMPT_DISPLAY + }$p"
  else
    SHORT="${p:0:40}$([ ${#p} -gt 40 ] && echo '...')"
    PROMPT_DISPLAY="${PROMPT_DISPLAY:+$PROMPT_DISPLAY + }\"$SHORT\""
  fi
done

STAGE_DISPLAY=""
for sname in "${!STAGE_EXEC[@]}"; do
  s="${sname}→${STAGE_EXEC[$sname]}"
  [ -n "${STAGE_MODEL[$sname]:-}" ] && s="$s/${STAGE_MODEL[$sname]}"
  STAGE_DISPLAY="${STAGE_DISPLAY:+$STAGE_DISPLAY, }$s"
done

# ── signals appended to every prompt ──────────────────────────────────
SIGNALS='
---
# Loop Signals

You are running inside a fresh-context loop. Each round you have NO memory of previous rounds. All state must be read from files on disk.

Emit XML signals during your work:

- <loop:update>status</loop:update> — progress milestone
- <loop:done>summary</loop:done> — ALL work complete, loop exits
- <loop:failed>reason</loop:failed> — stuck/blocked, loop stops
- <loop:human>question</loop:human> — need human input
- <loop:stage>name</loop:stage> — switch executor/model for next round

Rules:
- <loop:done> = ENTIRE project finished. Not one step. If more work remains — do NOT emit.
- If unsure — just finish your step and exit. Loop brings you back.
- Do exactly ONE step per round. Read state → do one thing → update state → exit.
'

# ── helpers ───────────────────────────────────────────────────────────
HUMAN_FILE=".hltm/human.md"

extract_signals() {
  local tag="$1" input="$2"
  echo "$input" | sed -n "s/.*<loop:${tag}>\(.*\)<\/loop:${tag}>.*/\1/p"
}

ts() { printf "${C_DIM}[%s]${C_RESET} " "$(date +%H:%M:%S)"; }

# ── jq filters for claude stream-json ────────────────────────────────
JQ_STREAM='
  if .type == "content_block_delta" and .delta?.type == "text_delta" then
    .delta.text // empty
  elif .type == "assistant" then
    ([.message?.content[]? | select(.type == "text") | .text] | join(""))
  elif .type == "message_stop" then
    ([.message?.content[]? | select(.type == "text") | .text] | join(""))
  elif .type == "result" and (.result?.output | type) == "object" then
    ([.result.output.content[]? | select(.type == "text") | .text] | join(""))
  else empty end
'


# ── build prompt ──────────────────────────────────────────────────────
build_prompt() {
  local result=""
  # inject current stage if set
  if [ -n "$CUR_STAGE" ]; then
    result="Current stage: ${CUR_STAGE}"$'\n\n'
  fi
  for p in "${PROMPTS[@]}"; do
    if [ -f "$p" ]; then
      result="${result}$(cat "$p")"$'\n\n'
    else
      result="${result}${p}"$'\n\n'
    fi
  done
  if [ -f "$HUMAN_FILE" ]; then
    result="${result}# Human Q&A History"$'\n'
    result="${result}$(cat "$HUMAN_FILE")"$'\n\n'
  fi
  result="${result}${SIGNALS}"
  echo "$result"
}

# ── executors ─────────────────────────────────────────────────────────
run_claude() {
  local prompt="$1" mdl="$2" logfile="$3"
  local txtfile
  txtfile=$(mktemp)

  local model_args=()
  [ -n "$mdl" ] && model_args+=(--model "$mdl")

  if [ "$VERBOSE" = "1" ]; then
    printf "  ${C_DIM}┄┄┄${C_RESET}\n"
    claude -p --dangerously-skip-permissions \
      "${model_args[@]}" --verbose \
      --output-format stream-json \
      "$prompt" 2>&1 | tee "$logfile" | \
      jq --unbuffered -r "$JQ_STREAM" 2>/dev/null
    echo ""
    printf "  ${C_DIM}┄┄┄${C_RESET}\n"
  else
    claude -p --dangerously-skip-permissions \
      "${model_args[@]}" --verbose \
      --output-format stream-json \
      "$prompt" 2>&1 | tee "$logfile" | \
      jq --unbuffered -r "$JQ_STREAM" 2>/dev/null | \
      while IFS= read -r line; do
        local sig
        sig=$(echo "$line" | sed -n 's/.*<loop:update>\(.*\)<\/loop:update>.*/\1/p')
        [ -n "$sig" ] && printf "$(ts)  ${C_CYAN}▸${C_RESET} %s\n" "$sig"
      done
  fi

  # extract text output from raw stream log
  jq -rj "$JQ_STREAM" < "$logfile" > "$txtfile" 2>/dev/null
  mv "$txtfile" "$logfile"
}

run_codex() {
  local prompt="$1" mdl="$2" tmpfile="$3"
  local sandbox="full-auto"
  [ "${HLTM_DOCKER:-}" = "1" ] && sandbox="danger-full-access"

  local model_args=()
  [ -n "$mdl" ] && model_args+=(-c "model=$mdl")

  if [ "$VERBOSE" = "1" ]; then
    printf "  ${C_DIM}┄┄┄${C_RESET}\n"
    codex exec \
      --sandbox "$sandbox" \
      --skip-git-repo-check \
      "${model_args[@]}" \
      -c model_reasoning_effort=xhigh \
      -c stream_idle_timeout_ms=3600000 \
      "$prompt" 2>&1 | tee "$tmpfile"
    echo ""
    printf "  ${C_DIM}┄┄┄${C_RESET}\n"
  else
    codex exec \
      --sandbox "$sandbox" \
      --skip-git-repo-check \
      "${model_args[@]}" \
      -c model_reasoning_effort=xhigh \
      -c stream_idle_timeout_ms=3600000 \
      "$prompt" 2>&1 | tee "$tmpfile" | \
      while IFS= read -r line; do
        case "$line" in
          codex) in_agent=1; continue ;;
          user*|thinking*|exec*|"tokens used"*|"mcp startup"*|--------*) in_agent=0; continue ;;
        esac
        [ "${in_agent:-0}" = "1" ] || continue
        local sig
        sig=$(echo "$line" | sed -n 's/.*<loop:update>\(.*\)<\/loop:update>.*/\1/p')
        [ -n "$sig" ] && printf "$(ts)  ${C_CYAN}▸${C_RESET} %s\n" "$sig"
      done
  fi

  # extract only agent output (codex sections), strip prompt echo / thinking / exec
  local txtfile
  txtfile=$(mktemp)
  awk '/^codex$/{c=1;next} /^(user|thinking|exec|tokens used|mcp startup|--------)[[:space:]]*$/{c=0} c' "$tmpfile" > "$txtfile"
  mv "$txtfile" "$tmpfile"
}

# ── banner ────────────────────────────────────────────────────────────
printf "\n"
printf "  ${C_BOLD}${C_CYAN}hltm-loop${C_RESET}\n"
printf "  ${C_DIM}executor${C_RESET}  %s\n" "$EXECUTOR"
printf "  ${C_DIM}model${C_RESET}     %s\n" "$MODEL"
printf "  ${C_DIM}prompt${C_RESET}    %s\n" "$PROMPT_DISPLAY"
printf "  ${C_DIM}max${C_RESET}       %s\n" "$([ "$MAX" -gt 0 ] 2>/dev/null && echo "$MAX" || echo "unlimited")"
printf "  ${C_DIM}human${C_RESET}     %s\n" "$([ "$HUMAN_BLOCK" = "1" ] && echo "block" || echo "defer")"
[ -n "$STAGE_DISPLAY" ] && printf "  ${C_DIM}stages${C_RESET}    %s\n" "$STAGE_DISPLAY"
printf "\n"

# ── session logs ──────────────────────────────────────────────────────
SESSION_ID=$(date +%Y-%m-%d_%H%M%S)
LOG_DIR=".hltm/logs/$SESSION_ID"
mkdir -p "$LOG_DIR"
printf "  ${C_DIM}logs${C_RESET}      %s/\n\n" "$LOG_DIR"

# ── signals ──────────────────────────────────────────────────────────
trap 'printf "\n  ${C_YELLOW}⏹ interrupted${C_RESET}\n\n"; exit 130' INT TERM

# ── main loop ─────────────────────────────────────────────────────────
ROUND=0
FAILURES=0
CUR_EXEC="$EXECUTOR"
CUR_MODEL="$MODEL"
CUR_STAGE=""

while true; do
  ROUND=$((ROUND + 1))
  [ "$MAX" -gt 0 ] 2>/dev/null && [ "$ROUND" -gt "$MAX" ] && \
    printf "\n${C_YELLOW}max rounds (%s) reached.${C_RESET}\n" "$MAX" && break

  # round header
  STAGE_TAG=""
  if [ "$CUR_EXEC" != "$EXECUTOR" ] || [ "$CUR_MODEL" != "$MODEL" ]; then
    model_tag=""
    [ -n "$CUR_MODEL" ] && [ "$CUR_MODEL" != "$MODEL" ] && model_tag="/$CUR_MODEL"
    STAGE_TAG=" ${C_MAGENTA}[${CUR_EXEC}${model_tag}]${C_RESET}"
  fi
  printf "$(ts)${C_DIM}━━━${C_RESET} ${C_BOLD}round %d${C_RESET}%b ${C_DIM}%s${C_RESET}\n" \
    "$ROUND" "$STAGE_TAG" "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  START=$(date +%s)
  FULL_PROMPT=$(build_prompt)
  LOG_FILE=$(printf "%s/round-%03d.log" "$LOG_DIR" "$ROUND")

  case "$CUR_EXEC" in
    claude-code) run_claude "$FULL_PROMPT" "$CUR_MODEL" "$LOG_FILE" ;;
    codex)       run_codex  "$FULL_PROMPT" "$CUR_MODEL" "$LOG_FILE" ;;
  esac

  EXIT_CODE=$?
  OUTPUT=$(cat "$LOG_FILE")

  ELAPSED=$(( $(date +%s) - START ))

  # ── agent failure ────────────────────────────────────────────────────
  if [ "$EXIT_CODE" -ne 0 ]; then
    FAILURES=$((FAILURES + 1))
    printf "\n$(ts)  ${C_YELLOW}⚠ exit %d (%d/3 failures)${C_RESET}\n" "$EXIT_CODE" "$FAILURES"
    if [ "$FAILURES" -ge 3 ]; then
      printf "$(ts)  ${C_RED}✗ 3 consecutive failures, stopping.${C_RESET}\n"
      exit 1
    fi
    sleep 5
    continue
  fi
  FAILURES=0

  # ── short round ──────────────────────────────────────────────────────
  [ "$ELAPSED" -lt 5 ] && \
    printf "\n$(ts)  ${C_YELLOW}⚠ round too short (%ds) — agent may be stuck${C_RESET}\n" "$ELAPSED"

  # ── round summary ───────────────────────────────────────────────────
  printf "\n$(ts)  ${C_DIM}round %d · %ds${C_RESET}\n" "$ROUND" "$ELAPSED"

  # ── <loop:done> ─────────────────────────────────────────────────────
  if echo "$OUTPUT" | grep -q "<loop:done"; then
    SUMMARY=$(extract_signals "done" "$OUTPUT" | tail -1)
    printf "\n$(ts)  ${C_GREEN}✓ done in %d round(s)${C_RESET}\n" "$ROUND"
    [ -n "$SUMMARY" ] && printf "$(ts)  ${C_GREEN}↳${C_RESET} %s\n" "$SUMMARY"
    printf "\n"
    break
  fi

  # ── <loop:failed> ───────────────────────────────────────────────────
  if echo "$OUTPUT" | grep -q "<loop:failed"; then
    REASON=$(extract_signals "failed" "$OUTPUT" | tail -1)
    printf "\n$(ts)  ${C_RED}✗ failed at round %d${C_RESET}\n" "$ROUND"
    [ -n "$REASON" ] && printf "$(ts)  ${C_RED}↳${C_RESET} %s\n" "$REASON"
    printf "\n"
    exit 1
  fi

  # ── <loop:human> ────────────────────────────────────────────────────
  if echo "$OUTPUT" | grep -q "<loop:human"; then
    QUESTION=$(extract_signals "human" "$OUTPUT" | tail -1)
    if [ -n "$QUESTION" ]; then
      mkdir -p "$(dirname "$HUMAN_FILE")"
      printf '\n## Round %d\nQ: %s\nA: \n' "$ROUND" "$QUESTION" >> "$HUMAN_FILE"
      printf "\n$(ts)  ${C_YELLOW}? human input needed${C_RESET} → %s\n" "$HUMAN_FILE"
      printf "$(ts)  ${C_YELLOW}↳${C_RESET} %s\n" "$QUESTION"
      if [ "$HUMAN_BLOCK" = "1" ]; then
        printf "\n$(ts)  ${C_YELLOW}⏸ stopped (--human-block). Answer in %s and re-run.${C_RESET}\n\n" "$HUMAN_FILE"
        exit 0
      fi
    fi
  fi

  # ── <loop:stage> ────────────────────────────────────────────────────
  if echo "$OUTPUT" | grep -q "<loop:stage"; then
    NEXT_STAGE=$(extract_signals "stage" "$OUTPUT" | tail -1 | tr -d '[:space:]')
    # extract base name for executor lookup (fix:nbp-14s → fix)
    STAGE_BASE="${NEXT_STAGE%%:*}"
    if [ -n "$NEXT_STAGE" ] && [ -n "${STAGE_EXEC[$STAGE_BASE]:-}" ]; then
      CUR_STAGE="$NEXT_STAGE"
      CUR_EXEC="${STAGE_EXEC[$STAGE_BASE]}"
      CUR_MODEL="${STAGE_MODEL[$STAGE_BASE]:-}"
      stg_model_tag=""
      [ -n "$CUR_MODEL" ] && stg_model_tag="/$CUR_MODEL"
      printf "$(ts)  ${C_MAGENTA}⇢ stage: %s → %s%s${C_RESET}\n" \
        "$NEXT_STAGE" "$CUR_EXEC" "$stg_model_tag"
    elif [ -n "$NEXT_STAGE" ]; then
      CUR_STAGE="$NEXT_STAGE"
      CUR_EXEC="$EXECUTOR"
      CUR_MODEL="$MODEL"
      printf "$(ts)  ${C_MAGENTA}⇢ stage: %s → %s (default)${C_RESET}\n" "$NEXT_STAGE" "$CUR_EXEC"
    fi
  fi

  sleep 2
done
