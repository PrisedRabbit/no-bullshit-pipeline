# Story Execution

## Story Loop

For each story:

```
/create-story   → ready-for-dev
/dev-story      → review (NOT done!)
/hltm-testing qa
/hltm-testing run
/hltm-testing review
/code-review    → done (ONLY here!)
```

**Use TodoWrite to track progress.**

---

## Gates

### Tests Gate

```
Red  → FIX before proceeding
Green → Continue
```

### Review Gate

```
Score < 70 → Add tests, re-review
Score 70+  → Continue
```

### Code Review Gate

```
Issues? → Fix, re-run tests
All good → Mark done
```

---

## Status Rules

| Status | Who Sets |
|--------|----------|
| ready-for-dev | /create-story |
| in-progress | /dev-story (auto) |
| review | /dev-story (max!) |
| done | **ONLY /code-review** |

```
FORBIDDEN: dev-story sets "done"
FORBIDDEN: skip code-review
```

---

## Error Handling

```
Fail → FIX → Retry (max 5)
After 5 → STOP, report to user
```
