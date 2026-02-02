# Architecture

## Project Structure

```
src/
├── app/                    # Next.js pages
│   ├── (auth)/            # Public routes (login, signup)
│   ├── (dashboard)/       # Protected routes
│   ├── layout.tsx
│   └── page.tsx
├── components/
│   ├── ui/                # shadcn components
│   └── [feature]/         # Feature components
├── lib/
│   ├── firebase/          # Firebase config & helpers
│   ├── hooks/             # Custom hooks
│   └── utils/             # Helpers
└── types/                 # TypeScript types
```

## Firestore Structure

```
users/{userId}
  - email
  - displayName
  - createdAt

users/{userId}/[subcollection]/{docId}
  - ...user's private data

[collection]/{docId}
  - ...shared data
  - ownerId (for access control)
```

## Auth Pattern

```typescript
// lib/firebase/auth-context.tsx
"use client";

import { createContext, useContext, useEffect, useState } from "react";
import { User, onAuthStateChanged } from "firebase/auth";
import { auth } from "./config";

const AuthContext = createContext<{ user: User | null; loading: boolean }>({
  user: null,
  loading: true,
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    return onAuthStateChanged(auth, (user) => {
      setUser(user);
      setLoading(false);
    });
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading }}>
      {children}
    </AuthContext.Provider>
  );
}

export const useAuth = () => useContext(AuthContext);
```

## Data Fetching Pattern

```typescript
// Real-time subscription
function useCollection<T>(path: string) {
  const [data, setData] = useState<T[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const unsubscribe = onSnapshot(collection(db, path), (snapshot) => {
      setData(snapshot.docs.map((doc) => ({ id: doc.id, ...doc.data() } as T)));
      setLoading(false);
    });
    return unsubscribe;
  }, [path]);

  return { data, loading };
}
```

## Security Rules

```javascript
rules_version = '2';
service cloud.firestore {
  match /databases/{database}/documents {
    // User's own data
    match /users/{userId} {
      allow read, write: if request.auth != null && request.auth.uid == userId;
    }

    // User's subcollections
    match /users/{userId}/{collection}/{docId} {
      allow read, write: if request.auth != null && request.auth.uid == userId;
    }
  }
}
```
