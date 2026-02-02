# Testing

## Structure

```
tests/
├── unit/               # Vitest unit tests
│   └── *.test.ts
├── component/          # Vitest component tests
│   └── *.test.tsx
├── e2e/                # Playwright E2E tests
│   └── *.spec.ts
├── fixtures/           # Shared test data
├── helpers/            # Test utilities
└── setup.ts            # Global setup
```

## Stack

```
vitest            # Unit + Component tests
playwright        # E2E tests
```

```bash
bun add -D vitest @testing-library/react @testing-library/jest-dom jsdom
bun add -D @playwright/test
```

## Config

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

## Scripts

```json
{
  "scripts": {
    "test": "vitest",
    "test:watch": "vitest --watch",
    "test:ui": "vitest --ui",
    "test:e2e": "playwright test",
    "test:e2e:ui": "playwright test --ui"
  }
}
```

## Unit Test Example

```typescript
// tests/unit/utils.test.ts
import { describe, it, expect } from "vitest";
import { formatDate } from "@/lib/utils";

describe("formatDate", () => {
  it("formats date correctly", () => {
    const date = new Date("2024-01-15");
    expect(formatDate(date)).toBe("Jan 15, 2024");
  });

  it("handles invalid date", () => {
    expect(formatDate(null)).toBe("");
  });
});
```

## Component Test Example

```typescript
// tests/component/button.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "@/components/ui/button";

describe("Button", () => {
  it("renders children", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText("Click me")).toBeInTheDocument();
  });

  it("calls onClick", () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Click</Button>);
    fireEvent.click(screen.getByText("Click"));
    expect(onClick).toHaveBeenCalled();
  });

  it("disabled state", () => {
    render(<Button disabled>Click</Button>);
    expect(screen.getByText("Click")).toBeDisabled();
  });
});
```

## E2E Test Example

```typescript
// tests/e2e/auth.spec.ts
import { test, expect } from "@playwright/test";

test.describe("Auth", () => {
  test("login flow", async ({ page }) => {
    await page.goto("/login");

    await page.fill('[name="email"]', "test@example.com");
    await page.fill('[name="password"]', "password123");
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL("/dashboard");
    await expect(page.getByText("Welcome")).toBeVisible();
  });

  test("shows error on invalid credentials", async ({ page }) => {
    await page.goto("/login");

    await page.fill('[name="email"]', "wrong@example.com");
    await page.fill('[name="password"]', "wrong");
    await page.click('button[type="submit"]');

    await expect(page.getByText("Invalid")).toBeVisible();
  });
});
```

## What to Test

### Unit Tests (`tests/unit/`)
- Utility functions
- Hooks (custom logic)
- Schema validation
- Pure functions

### Component Tests (`tests/component/`)
- User interactions
- Conditional rendering
- Form validation display
- State changes

### E2E Tests (`tests/e2e/`)
- Auth flows (login, signup, logout)
- Critical user journeys
- Payment flows
- Multi-page flows

### Skip
- Third-party libraries
- Styling
- Simple wrappers
- Framework internals
