# UX Patterns

## Layouts

### App Layout (dashboard, tools)
```
┌─────────────────────────────────┐
│ Header                          │
├────────┬────────────────────────┤
│ Sidebar│ Content                │
└────────┴────────────────────────┘
```

### Content Layout (marketing, blog)
```
┌─────────────────────────────────┐
│ Header                          │
├─────────────────────────────────┤
│         Content (centered)      │
├─────────────────────────────────┤
│ Footer                          │
└─────────────────────────────────┘
```

## Breakpoints

```
sm: 640px    # phones
md: 768px    # tablets
lg: 1024px   # laptops
xl: 1280px   # desktops
```

## Loading States

- **Skeleton** - for known layouts
- **Spinner** - for buttons, actions
- **Progress bar** - for uploads, long operations

## Empty States

```
[Icon]
Headline (what's missing)
Description (what to do)
[Action Button]
```

## Forms

- Label above input
- Error below input (red)
- Submit button right-aligned
- Loading state on submit

## Modals

Use for:
- Confirmations
- Quick forms (< 5 fields)

Avoid for:
- Complex flows (use page)
- Nested modals

## Mobile

| Desktop | Mobile |
|---------|--------|
| Sidebar | Bottom nav / hamburger |
| Table | Card list |
| Modal | Full-screen sheet |
