#!/usr/bin/env bash
# host-builder.sh — runs on Mac, executes build commands for Docker agent
# Usage: ./host-builder.sh /path/to/project
set -uo pipefail

BRIDGE="${TMPDIR:-/tmp}/hltm-bridge"
WORKSPACE="${1:-.}"

C_RESET='\033[0m' C_DIM='\033[2m' C_GREEN='\033[32m' C_RED='\033[31m' C_CYAN='\033[36m' C_YELLOW='\033[33m'

mkdir -p "$BRIDGE"
rm -f "$BRIDGE/request" "$BRIDGE/response" "$BRIDGE/exit_code"

printf "${C_CYAN}host-builder${C_RESET} listening\n"
printf "${C_DIM}workspace${C_RESET}  %s\n" "$(cd "$WORKSPACE" && pwd)"
printf "${C_DIM}bridge${C_RESET}     %s\n\n" "$BRIDGE"

cleanup() {
  rm -rf "$BRIDGE"
  printf "\n${C_YELLOW}stopped${C_RESET}\n"
  exit 0
}
trap cleanup INT TERM

# allowed commands (first word must match whitelist)
run_cmd() {
  local full="$1"
  local cmd="${full%% *}"
  case "$cmd" in
    ping)
      echo "pong" ;;
    check|clippy|build|tauri)
      cd "$WORKSPACE" && cargo $full 2>&1 ;;
    *)
      echo "ERROR: command not allowed: $cmd"
      return 1 ;;
  esac
}

while true; do
  if [ -f "$BRIDGE/request" ]; then
    CMD=$(cat "$BRIDGE/request")
    rm -f "$BRIDGE/request"

    printf "$(date +%H:%M:%S) ${C_GREEN}→${C_RESET} %s\n" "$CMD"

    OUTPUT=$(run_cmd "$CMD")
    EXIT=$?

    # replace host paths with Docker paths so agent sees /workspace
    ABS_WORKSPACE="$(cd "$WORKSPACE" && pwd)"
    echo "${OUTPUT//$ABS_WORKSPACE//workspace}" > "$BRIDGE/response"
    echo "$EXIT" > "$BRIDGE/exit_code"

    if [ "$EXIT" -eq 0 ]; then
      printf "$(date +%H:%M:%S) ${C_GREEN}✓${C_RESET} exit %d\n" "$EXIT"
    else
      printf "$(date +%H:%M:%S) ${C_RED}✗${C_RESET} exit %d\n" "$EXIT"
    fi
  fi
  sleep 0.2
done
