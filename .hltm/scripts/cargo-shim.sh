#!/usr/bin/env bash
# cargo-shim.sh — replaces cargo inside Docker, delegates to host Mac
# Installed as /usr/local/bin/cargo in Dockerfile

BRIDGE="/tmp/hltm-bridge"

# if bridge not mounted, fall back to real cargo
if [ ! -d "$BRIDGE" ]; then
  exec -a cargo /usr/local/cargo/bin/cargo-real "$@"
fi

CMD="$1"

# only delegate build-related commands to host
case "$CMD" in
  check|clippy|build|tauri)
    # send full command line: "check --manifest-path src-tauri/Cargo.toml" etc.
    echo "$*" > "$BRIDGE/request"
    # wait for response
    while [ ! -f "$BRIDGE/exit_code" ]; do
      sleep 0.2
    done
    cat "$BRIDGE/response"
    EXIT=$(cat "$BRIDGE/exit_code")
    rm -f "$BRIDGE/response" "$BRIDGE/exit_code"
    exit "$EXIT"
    ;;
  *)
    # everything else (install, etc.) runs locally in Docker
    exec -a cargo /usr/local/cargo/bin/cargo-real "$@"
    ;;
esac
