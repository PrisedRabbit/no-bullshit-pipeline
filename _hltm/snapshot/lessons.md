# Lessons

## 2026-02-22: UTF-8 panic in LLM connector on Cyrillic text

**What happened**: Processing Russian transcript via Qwen caused panic in `connectors/llm.rs`: `byte index 32768 is not a char boundary; it is inside 'ь'`. App crashed (nbp-5dc).

**Root cause**: Prompt truncation at fixed byte offset (32768 = 32KB) without checking UTF-8 char boundaries. Cyrillic chars are 2 bytes, slicing at arbitrary byte index splits a character.

**Fix**: Used `str::floor_char_boundary()` to find nearest safe truncation point before slicing.

**Prevention**: Always use UTF-8 safe string operations when truncating. Never slice Rust strings at arbitrary byte indices — use `floor_char_boundary()` or `char_indices()`.

---

## 2026-02-22: Rubato resampler type annotation needed for process_partial drain

**What happened**: `resampler.process_partial(None, None)` failed to compile — Rust couldn't infer generic type `V` for the `None` argument (nbp-lwl review fail #3).

**Root cause**: `process_partial<V: AsRef<[T]>>` needs explicit type when both arguments are `None`. The existing codebase in `realtime_mixer.rs` already had the correct pattern.

**Fix**: `resampler.process_partial(None::<&[Vec<f32>]>, None)` — same pattern as `realtime_mixer.rs`.

**Prevention**: When calling rubato's `process_partial` with `None`, always provide explicit type annotation. Check existing code in `realtime_mixer.rs` for the correct pattern.

---

## 2026-02-22: Feature branches diverged into stacked chains

**What happened**: Four feat/ branches accumulated as stacked chains (each branching from the previous) instead of being merged back to dev. Required untangling and conflict resolution to merge.

**Root cause**: Work was done sequentially on stacked branches without merging back to dev between tasks.

**Fix**: Identified two independent chains via git ancestry checks, merged sequentially into dev, resolved conflicts, deleted all feat branches.

**Prevention**: Merge feature branches back to dev promptly after review. Don't stack more than one branch deep without merging.

---
