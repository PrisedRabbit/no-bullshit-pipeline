# Data

## CRUD Operations

```typescript
// lib/firebase/firestore.ts
import {
  collection,
  doc,
  addDoc,
  getDoc,
  getDocs,
  updateDoc,
  deleteDoc,
  query,
  where,
  orderBy,
  limit,
  serverTimestamp,
} from "firebase/firestore";
import { db } from "./config";

// Create
export const create = async <T extends object>(path: string, data: T) => {
  const ref = await addDoc(collection(db, path), {
    ...data,
    createdAt: serverTimestamp(),
  });
  return ref.id;
};

// Read one
export const getOne = async <T>(path: string, id: string) => {
  const snap = await getDoc(doc(db, path, id));
  if (!snap.exists()) return null;
  return { id: snap.id, ...snap.data() } as T;
};

// Read many
export const getMany = async <T>(path: string) => {
  const snap = await getDocs(collection(db, path));
  return snap.docs.map((d) => ({ id: d.id, ...d.data() } as T));
};

// Update
export const update = async <T extends object>(
  path: string,
  id: string,
  data: Partial<T>
) => {
  await updateDoc(doc(db, path, id), {
    ...data,
    updatedAt: serverTimestamp(),
  });
};

// Delete
export const remove = async (path: string, id: string) => {
  await deleteDoc(doc(db, path, id));
};
```

## Real-time Hook

```typescript
// lib/hooks/use-collection.ts
import { useEffect, useState } from "react";
import { collection, onSnapshot, query, QueryConstraint } from "firebase/firestore";
import { db } from "@/lib/firebase/config";

export function useCollection<T>(path: string, constraints: QueryConstraint[] = []) {
  const [data, setData] = useState<T[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const q = query(collection(db, path), ...constraints);
    const unsubscribe = onSnapshot(
      q,
      (snap) => {
        setData(snap.docs.map((d) => ({ id: d.id, ...d.data() } as T)));
        setLoading(false);
      },
      (err) => {
        setError(err);
        setLoading(false);
      }
    );
    return unsubscribe;
  }, [path]);

  return { data, loading, error };
}
```

## Pagination

```typescript
// lib/hooks/use-paginated.ts
import { useState } from "react";
import {
  collection,
  query,
  orderBy,
  limit,
  startAfter,
  getDocs,
  DocumentSnapshot,
} from "firebase/firestore";
import { db } from "@/lib/firebase/config";

const PAGE_SIZE = 20;

export function usePaginated<T>(path: string, orderField: string) {
  const [data, setData] = useState<T[]>([]);
  const [lastDoc, setLastDoc] = useState<DocumentSnapshot | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [loading, setLoading] = useState(false);

  const loadMore = async () => {
    if (loading || !hasMore) return;
    setLoading(true);

    let q = query(
      collection(db, path),
      orderBy(orderField, "desc"),
      limit(PAGE_SIZE)
    );

    if (lastDoc) {
      q = query(q, startAfter(lastDoc));
    }

    const snap = await getDocs(q);
    const newData = snap.docs.map((d) => ({ id: d.id, ...d.data() } as T));

    setData((prev) => [...prev, ...newData]);
    setLastDoc(snap.docs[snap.docs.length - 1] || null);
    setHasMore(snap.docs.length === PAGE_SIZE);
    setLoading(false);
  };

  return { data, loadMore, hasMore, loading };
}
```

## Optimistic Update

```typescript
const [items, setItems] = useState<Item[]>([]);

const deleteItem = async (id: string) => {
  // Optimistic: remove immediately
  const prev = items;
  setItems(items.filter((i) => i.id !== id));

  try {
    await remove("items", id);
  } catch (err) {
    // Rollback on error
    setItems(prev);
    toast.error("Failed to delete");
  }
};
```
