#!/bin/bash
# Selective copy of credentials from read-only mounts into container home.

if [ -d /mnt/claude ]; then
    mkdir -p "$HOME/.claude"
    for f in .credentials.json settings.json settings.local.json CLAUDE.md; do
        [ -e "/mnt/claude/$f" ] && cp -L "/mnt/claude/$f" "$HOME/.claude/$f" 2>/dev/null || true
    done
    for d in commands skills hooks agents plugins; do
        [ -d "/mnt/claude/$d" ] && cp -rL "/mnt/claude/$d" "$HOME/.claude/" 2>/dev/null || true
    done
fi

# Claude credentials extracted from macOS Keychain
if [ -f /mnt/claude-credentials.json ]; then
    mkdir -p "$HOME/.claude"
    cp /mnt/claude-credentials.json "$HOME/.claude/.credentials.json"
    chmod 600 "$HOME/.claude/.credentials.json"
fi

# Codex credentials
if [ -d /mnt/codex ]; then
    mkdir -p "$HOME/.codex"
    cp -rL /mnt/codex/* "$HOME/.codex/" 2>/dev/null || true
fi

exec "$@"
