# Readiness Checks

Binary checks. No reasoning.

## File Existence

```
[ ] docs/prd.md
[ ] docs/architecture.md
[ ] docs/epics/*.md (at least one)
[ ] docs/ux-design.md (if UI)
```

Missing → **FAIL**

## Drift Detection

```
[ ] PRD features ↔ architecture support
[ ] Architecture ↔ epic stories
```

Mismatch → **FAIL**

## Output

```json
{
  "missing": [],
  "conflicts": [],
  "verdict": "PASS"
}
```

## Verdict

```
missing.length > 0 OR conflicts.length > 0 → FAIL
Otherwise → PASS
```

Arrays win. Reasoning ignored.
