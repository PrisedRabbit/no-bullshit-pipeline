# Tech Stack

## Frontend

```
next@14+          # Framework (App Router)
react@18+         # UI library
typescript        # Language
tailwindcss       # Styling
shadcn/ui         # Components
```

## Backend

```
firebase          # Platform
├── firestore     # Database
├── auth          # Authentication
├── storage       # File storage
└── hosting       # Web deployment
```

## Dev Tools

```
bun               # Package manager & runtime (never npm/pnpm)
eslint + prettier # Linting
vitest            # Unit tests
playwright        # E2E tests
```

## Firebase Config

```typescript
// lib/firebase/config.ts
import { initializeApp, getApps } from "firebase/app";
import { getAuth } from "firebase/auth";
import { getFirestore } from "firebase/firestore";
import { getStorage } from "firebase/storage";

const config = {
  apiKey: process.env.NEXT_PUBLIC_FIREBASE_API_KEY,
  authDomain: process.env.NEXT_PUBLIC_FIREBASE_AUTH_DOMAIN,
  projectId: process.env.NEXT_PUBLIC_FIREBASE_PROJECT_ID,
  storageBucket: process.env.NEXT_PUBLIC_FIREBASE_STORAGE_BUCKET,
  messagingSenderId: process.env.NEXT_PUBLIC_FIREBASE_MESSAGING_SENDER_ID,
  appId: process.env.NEXT_PUBLIC_FIREBASE_APP_ID,
};

const app = getApps().length === 0 ? initializeApp(config) : getApps()[0];

export const auth = getAuth(app);
export const db = getFirestore(app);
export const storage = getStorage(app);
```

## Quick Start

```bash
bun create next-app my-app --typescript --tailwind --eslint --app
cd my-app
bun add firebase
bunx shadcn-ui@latest init
```
