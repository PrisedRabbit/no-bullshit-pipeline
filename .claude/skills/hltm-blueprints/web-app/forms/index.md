# Forms

## Stack

```
react-hook-form   # Form state
zod               # Validation
```

```bash
bun add react-hook-form zod @hookform/resolvers
```

## Basic Pattern

```typescript
"use client";

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const schema = z.object({
  name: z.string().min(1, "Required"),
  email: z.string().email("Invalid email"),
});

type FormData = z.infer<typeof schema>;

export function MyForm() {
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<FormData>({
    resolver: zodResolver(schema),
  });

  const onSubmit = async (data: FormData) => {
    // submit logic
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <div>
        <input {...register("name")} placeholder="Name" />
        {errors.name && <span className="text-red-500">{errors.name.message}</span>}
      </div>
      <div>
        <input {...register("email")} placeholder="Email" />
        {errors.email && <span className="text-red-500">{errors.email.message}</span>}
      </div>
      <button type="submit" disabled={isSubmitting}>
        {isSubmitting ? "Saving..." : "Save"}
      </button>
    </form>
  );
}
```

## Common Schemas

```typescript
// lib/schemas.ts
import { z } from "zod";

export const emailSchema = z.string().email("Invalid email");

export const passwordSchema = z
  .string()
  .min(8, "Min 8 characters")
  .regex(/[A-Z]/, "Need uppercase")
  .regex(/[0-9]/, "Need number");

export const confirmPasswordSchema = z
  .object({
    password: passwordSchema,
    confirmPassword: z.string(),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Passwords don't match",
    path: ["confirmPassword"],
  });
```

## With shadcn/ui

```typescript
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";

export function MyForm() {
  const form = useForm<FormData>({
    resolver: zodResolver(schema),
  });

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)}>
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Name</FormLabel>
              <FormControl>
                <Input {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <Button type="submit" disabled={form.formState.isSubmitting}>
          Save
        </Button>
      </form>
    </Form>
  );
}
```

## File Upload

```typescript
const fileSchema = z
  .instanceof(File)
  .refine((f) => f.size < 5_000_000, "Max 5MB")
  .refine((f) => ["image/jpeg", "image/png"].includes(f.type), "Only JPG/PNG");
```
