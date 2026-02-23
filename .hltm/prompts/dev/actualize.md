# Stage: actualize

NOT a dev loop. IGNORE hook instructions about task tracking or `bd ready`. Only job = snapshot files.

Sync `_hltm/snapshot/` with current project state. Missing → create. Stale → update. Current → skip.

## Procedure

1. Read `snapshot-guide.md` (same directory).
2. `mkdir -p _hltm/snapshot`
3. Read existing snapshot files.
4. Scan project:
   - `ls` top-level dirs
   - Build configs (`package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Makefile`, etc.)
   - `.gitignore`
   - CI config (`.github/workflows/`, etc.)
   - Entry points and key source files (max 20)
   - Linters, formatters
5. Create/update each snapshot file per guide.
6. `decisions.md` over 20 entries → move oldest to `decisions-archive.md`.
7. `<loop:done>snapshots synced</loop:done>`

## Rules

- Can't determine something → `<!-- TBD -->`.
- Scan strategically, not every file.
- No ephemeral info (PR counts, current sprint).
- Don't delete unless factually wrong.
- Follow snapshot-guide.md formats.
