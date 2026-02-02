# Test Framework Setup

## When

Before first epic. One time per project.

## Command

```
/hltm-testing setup
```

## What to Do

### 1. Install Dependencies

```bash
bun add -D vitest @testing-library/react @testing-library/jest-dom jsdom
bun add -D @playwright/test
```

### 2. Create Structure

```
tests/
├── unit/               # Vitest unit tests
├── component/          # Vitest component tests
├── e2e/                # Playwright E2E tests
├── fixtures/           # Shared test data
├── helpers/            # Test utilities
└── setup.ts            # Global setup
```

### 3. Create Configs

**vitest.config.ts:**
```typescript
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
    include: ["tests/**/*.test.{ts,tsx}"],
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

**tests/setup.ts:**
```typescript
import "@testing-library/jest-dom";
```

**playwright.config.ts:**
```typescript
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: process.env.BASE_URL || "http://localhost:3000",
    trace: "on-first-retry",
  },
});
```

### 4. Add Scripts

```json
{
  "scripts": {
    "test": "vitest",
    "test:watch": "vitest --watch",
    "test:e2e": "playwright test"
  }
}
```

## Verify

```bash
bun test        # Should run (0 tests ok)
```

## Skip If

- `vitest.config.ts` exists
- `tests/` directory has tests
