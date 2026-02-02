# Run All Tests

## When

- After `/hltm-testing qa` (verify new tests pass)
- After any code change (regression check)
- Before `/code-review` (gate)

## Command

```
/hltm-testing run
```

## What Happens

```bash
bun test
```

Runs ALL tests:
- Unit tests
- API tests
- Integration tests

## Expected Output

```
✓ tests/unit/auth.test.ts (5 tests)
✓ tests/unit/profile.test.ts (3 tests)
✓ tests/api/users.api.test.ts (4 tests)

Test Files  3 passed (3)
Tests       12 passed (12)
Duration    2.34s
```

## If Tests Fail

```
✗ tests/unit/auth.test.ts (2 failed)
  ✗ should validate email format
  ✗ should reject weak password
```

**Action:**

1. Read failure message
2. Determine cause:

| Cause | Action |
|-------|--------|
| Code broke | Fix the code |
| Test outdated | Update the test |
| Flaky test | Fix the test |

3. Re-run until green

## Gate

```
Tests red? → Do NOT proceed to code-review
Tests green? → Proceed
```

No exceptions.
