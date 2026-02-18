---
phase: 02-notion-connector
plan: 01
subsystem: connectors
tags: [rust, tauri, notion, notion-client, serde_json, btreemap, page-creation, connector]

# Dependency graph
requires:
  - phase: 01-notion-integration-infrastructure
    provides: "NotionIntegrationProfile, load_notion_profile(), get_notion_token(), PeopleMapping, NotionPropertyDef"
provides:
  - "connectors/notion.rs with execute() entry point matching standard connector signature"
  - "extract_json_array() handling bare JSON, json-fenced, and bare-fenced LLM output"
  - "resolve_select_value() with case-insensitive option matching and passthrough fallback"
  - "resolve_people_aliases() mapping alias strings to Notion User structs via profile people_mappings"
  - "build_notion_properties() covering all 12 writable Notion property types via BTreeMap<String, PageProperty>"
affects:
  - "02-02 (pipeline wiring — must add ConnectorType::Notion using this execute() entry point)"
  - "03 (prompt augmentation — Notion connector is the delivery target for structured LLM output)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Connector drives property iteration from profile.properties (not JSON keys) — ensures only schema-defined properties are sent"
    - "People value normalization: string→single-element array, null→skip, array→use directly"
    - "Multi-select value normalization: same string/array/null handling as people"
    - "DateOrDateTime::DateTime for strings containing T, DateOrDateTime::Date otherwise"
    - "serde_json::Number::from_f64() for number property conversion (NaN/Infinity skipped)"
    - "User struct constructed with explicit fields (avator_url typo preserved from notion-client)"

key-files:
  created:
    - "src-tauri/src/connectors/notion.rs"
  modified:
    - "src-tauri/src/connectors/mod.rs"

key-decisions:
  - "User struct constructed explicitly (not ..Default::default()) — notion-client User may not implement Default; explicit construction is safer and avoids compile uncertainty"
  - "Empty people array skipped entirely — sending empty Vec<User> to Notion API may clear existing assignees; safer to omit"
  - "Properties mapped from profile schema, not JSON keys — LLM output may have extra/unknown keys; only profile-defined properties are sent to Notion API"
  - "DateOrDateTime variant selected by T-character detection in date string — handles both date-only and datetime LLM output without additional parsing"

patterns-established:
  - "Notion property mapping: iterate profile.properties, look up JSON value, build PageProperty variant — used in build_notion_properties()"
  - "JSON fence extraction order: direct parse → json-fenced → bare-fenced → error with preview"

requirements-completed: [NOTN-08, EXEC-04]

# Metrics
duration: 4min
completed: 2026-02-18
---

# Phase 2 Plan 01: Notion Connector — Core Implementation Summary

**Notion connector with execute() that parses LLM JSON (fence-tolerant), resolves people aliases and select casing via stored profile, and creates typed Notion database pages via notion-client BTreeMap<String, PageProperty>**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-18T22:19:57Z
- **Completed:** 2026-02-18T22:23:02Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Created `connectors/notion.rs` implementing the full connector pipeline: JSON extraction from fenced or bare LLM output → select/people resolution via stored integration profile → typed PageProperty construction → Notion API page creation via notion-client
- Implemented all 12 writable Notion property types (title, rich_text, select, multi_select, people, date, number, checkbox, url, email, phone_number, status) with a single `build_notion_properties()` dispatch loop
- Verified all Phase 1 type references (profile field access, token helper signatures, error message quality, null/string/array edge cases for people and multi_select)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create connectors/notion.rs with helper functions and execute() entry point** - `c9317fe` (feat)
2. **Task 2: Verify connector integration with Phase 1 types** - (verification only, no code changes)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/connectors/notion.rs` - Full Notion connector: NotionConnectorConfig, extract_json_array(), resolve_select_value(), resolve_people_aliases(), build_notion_properties(), execute()
- `src-tauri/src/connectors/mod.rs` - Added `pub mod notion;` registration

## Decisions Made

- **User explicit construction over ..Default::default():** The notion-client User struct fields are known from Phase 1 research (id, name, avator_url, user_type — note crate's "avator" typo). Explicit construction avoids a potential compile failure if User doesn't derive Default, and the fields needed are exactly the four known ones.
- **Empty people array → skip property entirely:** Sending an empty `Vec<User>` to Notion's people property on page creation could clear existing assignees if the page already exists. Skipping the property when all aliases resolve to nothing is safer and matches Notion's own behavior for omitted properties.
- **Profile-driven property iteration (not JSON keys):** The LLM may output extra fields, renamed fields, or fields that don't exist in the Notion schema. Iterating `profile.properties` (not the JSON object keys) ensures only schema-known properties are sent, avoiding Notion API 400 errors for unknown property names.
- **DateOrDateTime variant by T-detection:** Rather than attempting full ISO 8601 parsing, the date string is checked for the presence of 'T'. This covers the two common LLM output formats (YYYY-MM-DD and YYYY-MM-DDTHH:MM:SSZ) without adding a dependency.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `cargo check` is not available in the execution environment (consistent with all Phase 1 plans). Code verified structurally:
  - All import paths match Phase 1 module structure (`crate::integrations::notion::`, `crate::connectors::strip_frontmatter`)
  - Field accesses match `NotionIntegrationProfile`, `NotionPropertyDef`, `PeopleMapping` structs from `integrations/notion.rs`
  - `execute()` signature matches `connectors/slack.rs` exactly (6 params: input_path, config, output_dir, step_name, input_step, description)
  - `BTreeMap<String, PageProperty>` used (not HashMap) as required by `CreateAPageRequest`
  - `User` constructed with 4 explicit fields matching Phase 1 research (id, name, avator_url, user_type)
  - `serde_json::Number::from_f64()` used for number properties (returns Option, NaN/Infinity skipped)
  - `DateOrDateTime::Date` / `DateOrDateTime::DateTime` variant names match research documentation

## User Setup Required

None - no external service configuration required for this plan. The connector reads from the stored integration profile created during Phase 1 setup.

## Next Phase Readiness

- Plan 02 (pipeline wiring) can now add `ConnectorType::Notion` to `pipelines.rs` and wire `connectors::notion::execute()` into `pipeline_engine.rs`
- The connector is complete and ready — no additional changes to `notion.rs` needed for Phase 2 Plan 02
- Real compilation verification remains deferred to first `cargo tauri dev` run (cargo not available in execution environment)

---
*Phase: 02-notion-connector*
*Completed: 2026-02-18*
