# Test Quality Review

## When

Before code-review. Verify test quality.

## Command

```
/hltm-testing review
```

## What Happens

1. **Run tests**
   ```bash
   bun test
   ```

2. **Audit test quality**
   - Real assertions vs placeholders?
   - Edge cases covered?
   - Error paths tested?
   - Tests independent?

## Quality Checklist

| Good | Bad |
|------|-----|
| `expect(result).toBe(5)` | `expect(true).toBeTruthy()` |
| Edge cases tested | Only happy path |
| Error handling tested | No error tests |
| Independent tests | Tests depend on order |
| Fast | Slow tests |
| No hard waits | `waitForTimeout(5000)` |

## Verdict

```
PASS → proceed to code-review
FAIL → FIX → re-review (max 5)
```

No scores. Either good or not.

## If FAIL

1. Identify issues
2. Fix tests
3. `/hltm-testing run` (verify green)
4. `/hltm-testing review` (re-audit)
5. Max 5 attempts, then STOP and report
