# Auth

## Setup

```typescript
// lib/firebase/auth.ts
import {
  signInWithEmailAndPassword,
  createUserWithEmailAndPassword,
  signInWithPopup,
  GoogleAuthProvider,
  signOut as firebaseSignOut,
  sendPasswordResetEmail,
} from "firebase/auth";
import { auth } from "./config";

export const signIn = (email: string, password: string) =>
  signInWithEmailAndPassword(auth, email, password);

export const signUp = (email: string, password: string) =>
  createUserWithEmailAndPassword(auth, email, password);

export const signInWithGoogle = () =>
  signInWithPopup(auth, new GoogleAuthProvider());

export const signOut = () => firebaseSignOut(auth);

export const resetPassword = (email: string) =>
  sendPasswordResetEmail(auth, email);
```

## Protected Route

```typescript
// components/auth/protected-route.tsx
"use client";

import { useAuth } from "@/lib/firebase/auth-context";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!loading && !user) {
      router.push("/login");
    }
  }, [user, loading, router]);

  if (loading) return <div>Loading...</div>;
  if (!user) return null;

  return <>{children}</>;
}
```

## Login Page

```typescript
// app/(auth)/login/page.tsx
"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { signIn, signInWithGoogle } from "@/lib/firebase/auth";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const router = useRouter();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await signIn(email, password);
      router.push("/dashboard");
    } catch (err) {
      setError("Invalid email or password");
    }
  };

  const handleGoogle = async () => {
    try {
      await signInWithGoogle();
      router.push("/dashboard");
    } catch (err) {
      setError("Google sign in failed");
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      {error && <p className="text-red-500">{error}</p>}
      <input
        type="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder="Email"
      />
      <input
        type="password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        placeholder="Password"
      />
      <button type="submit">Sign In</button>
      <button type="button" onClick={handleGoogle}>
        Sign In with Google
      </button>
    </form>
  );
}
```

## Error Messages

```typescript
const authErrors: Record<string, string> = {
  "auth/user-not-found": "No account with this email",
  "auth/wrong-password": "Incorrect password",
  "auth/email-already-in-use": "Email already registered",
  "auth/weak-password": "Password too weak (min 6 chars)",
  "auth/invalid-email": "Invalid email address",
  "auth/too-many-requests": "Too many attempts. Try later",
};

export const getAuthError = (code: string) =>
  authErrors[code] || "Something went wrong";
```
