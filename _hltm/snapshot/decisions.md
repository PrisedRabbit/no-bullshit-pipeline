# Decisions

## 2026-02-22: No sandbox for CLI agent pipeline steps

**Context**: When running Claude Code / Codex as a pipeline step (nbp-h7h), what level of sandboxing is needed?

**Decision**: No sandbox — agent runs with full user permissions.

**Alternatives considered**:
- Docker container: rejected because overkill for desktop app, requires Docker installed, codebase access needs bind mounts anyway
- Transcript-only mode: rejected because file/codebase access is the core value prop of CLI agents
- Permission-based approval: rejected because breaks post-recording automation

**Why**: Pipelines are user-authored config, not external attack surface. Working directory scoping provides implicit blast radius control without a full sandbox layer.

**Consequences**: nbp-h7h implementation spawns CLI as subprocess with user-configured working_directory, passes transcript via temp file/stdin, captures stdout, enforces timeout.

---

## 2026-02-22: Standalone Prompts section in sidebar

**Context**: Prompt templates were buried inside Pipelines section. Templates are tied to both LLM (they're prompts) and Pipelines (they're used in processing steps). Needed a proper home.

**Decision**: Standalone "Prompts" section in sidebar, same level as Pipelines and Recordings.

**Alternatives considered**:
- Part of model/LLM settings: rejected — prompts are not model config
- Keep in Pipelines but more accessible: rejected — prompts are a separate entity
- Inline in step editor with shared library: rejected — too complex

**Why**: Prompts are reusable entities — owned independently, referenced by pipeline steps. Like a function vs a function call. Pipeline step editor gets a dropdown to pick a prompt + edit link.

**Consequences**: Need new sidebar nav item, Prompts CRUD view, and pipeline step editor integration (nbp-c66).

---

## 2026-02-22: Provider-first config storage

**Context**: Model and API key settings were originally role-based (transcription provider, processing provider). As more providers and capabilities were added, this structure became limiting.

**Decision**: Refactor to provider-first storage — HashMap keyed by provider ID, each with api_key, capabilities, and models list.

**Alternatives considered**:
- Keep role-based: rejected because a single provider can serve multiple roles (OpenAI does transcription + processing)

**Why**: Provider-first maps naturally to the settings UI (one section per provider) and scales to new providers without schema changes.

**Consequences**: Legacy role-based fields kept with `#[serde(default)]` for backwards compatibility. Migration on load/save syncs old fields into new provider map.

---
