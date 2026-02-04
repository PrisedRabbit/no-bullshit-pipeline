---
name: hltm-impl-testing
description: Write tests + run tests for ONE story
version: 6.1.0
model: sonnet
context: fork
agent: general-purpose
---

# Testing
**Goal:** Write & run tests for ONE story.

## Logic

1. **Check Mode:** Read blueprint → get `testing.mode` (default: `balanced`)

2. **Execute:**

| Mode | Action |
|------|--------|
| **minimal** | Write smoke tests if missing → run tests |
| **balanced** | Call `/bmad-tea-testarch-automate` → run tests |
| **strict** | Verify ATDD coverage → automate → run tests |

3. **Return:**
   - Success: `{"status": "pass", "files": [...]}`
   - Failure: `{"status": "fail", "issues": [...]}`
