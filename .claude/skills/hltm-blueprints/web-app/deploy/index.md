# Deploy

## Firebase Hosting Setup

```bash
# Install CLI
bun add -g firebase-tools

# Login
firebase login

# Init (select Hosting, then Frameworks)
firebase init hosting
```

## firebase.json

```json
{
  "hosting": {
    "source": ".",
    "ignore": ["firebase.json", "**/.*", "**/node_modules/**"],
    "frameworksBackend": {
      "region": "us-central1"
    }
  }
}
```

## Deploy Commands

```bash
# Deploy all
firebase deploy

# Deploy only hosting
firebase deploy --only hosting

# Preview channel (for PRs)
firebase hosting:channel:deploy preview-branch
```

## GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: oven-sh/setup-bun@v1

      - run: bun install

      - run: bun run build

      - uses: FirebaseExtended/action-hosting-deploy@v0
        with:
          repoToken: "${{ secrets.GITHUB_TOKEN }}"
          firebaseServiceAccount: "${{ secrets.FIREBASE_SERVICE_ACCOUNT }}"
          channelId: live
          projectId: your-project-id
```

## Preview on PR

```yaml
# .github/workflows/preview.yml
name: Preview

on:
  pull_request:
    branches: [main]

jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: oven-sh/setup-bun@v1

      - run: bun install

      - run: bun run build

      - uses: FirebaseExtended/action-hosting-deploy@v0
        with:
          repoToken: "${{ secrets.GITHUB_TOKEN }}"
          firebaseServiceAccount: "${{ secrets.FIREBASE_SERVICE_ACCOUNT }}"
          projectId: your-project-id
          # No channelId = creates preview channel
```

## Environment Variables

Firebase Hosting reads from `.env.production`:

```bash
# .env.production
NEXT_PUBLIC_FIREBASE_API_KEY=xxx
NEXT_PUBLIC_FIREBASE_AUTH_DOMAIN=xxx
# ...
```

For secrets (not NEXT_PUBLIC_), use Firebase Functions config or CI secrets.
