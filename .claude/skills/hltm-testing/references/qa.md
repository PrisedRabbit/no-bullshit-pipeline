# QA Test Generation

## When

After `/dev-story`. Before code-review.

## Command

```
/hltm-testing qa
```

## What TEA Does

1. Reads: story file, code, test invariants
2. Converts invariants → tests
3. Generates: unit, API, E2E tests
4. Validates tests compile

## Validation Gate

```
Tests compile?
├── PASS → /hltm-testing run
└── FAIL → TEA fixes (max 3), then STOP
```

**Dev code is FROZEN during QA.**

## Dev vs QA

| | Dev | QA (TEA) |
|-|-----|----------|
| Writes code | ✅ | ❌ |
| Writes tests | ❌ | ✅ |
| Perspective | "How it works" | "How it breaks" |
