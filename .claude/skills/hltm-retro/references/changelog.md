# CHANGELOG Format

## Rules

1. **1-3 lines per Epic. MAX.**
2. Can't fit? Epic was too big. Your problem.
3. No implementation details
4. No tech jargon unless necessary
5. User-facing value only

## File Location

```
{PROJECT_ROOT}/changelogs/changelog-YYYY-MM.md
```

Example: `{PROJECT_ROOT}/changelogs/changelog-2025-01.md`

Monthly rotation. Scan all: `glob {PROJECT_ROOT}/changelogs/changelog-*.md`

## Format

```markdown
# Changelog 2025-01

## [Epic 3] Notifications
- Push notifications for new messages
- Email digest (daily/weekly)

## [Epic 2] Messaging
- Real-time chat between users
- File attachments up to 10MB

## [Epic 1] Auth & Profile
- User authentication (email + Google)
- Profile page with settings
```

## What to Write

✅ Good:
- "User authentication (email + Google)"
- "Dashboard with usage stats"
- "Export data to CSV"

❌ Bad:
- "Implemented Firebase Auth with custom claims and session management"
- "Added React Query for data fetching with optimistic updates"
- "Refactored auth context to use Zustand"

## No Archive Needed

Monthly files = automatic rotation.

- New month → new file
- Old months stay (git tracks)
- Agents scan current month, can glob all if needed

## Write Pattern

**Agent NEVER writes file directly.**

```
# Agent returns JSON
agent_output = {
  "epic": "Epic 3",
  "title": "Notifications",
  "items": [
    "Push notifications for new messages",
    "Email digest (daily/weekly)"
  ]
}

# Driver validates (1-3 items max)
if len(items) > 3:
    REJECT("Too many items. Epic too big.")

# Driver appends to changelog-YYYY-MM.md
```

Agent touches changelog = **BUG**.
