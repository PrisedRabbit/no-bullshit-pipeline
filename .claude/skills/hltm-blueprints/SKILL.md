---
name: hltm-blueprints
description: Project blueprint templates for HLTM
version: 2.0.0
model: sonnet
---

# HLTM Blueprints

Comprehensive project templates with tech stack, architecture, patterns.

## Available Blueprints

| Blueprint | Stack |
|-----------|-------|
| `web-app/` | Next.js / React / Bun / Tailwind |
| `desktop-app/` | Tauri / Rust / Vanilla JS |

## Structure

Each blueprint contains:

```
{blueprint}/
├── index.md          # Overview
├── tech-stack.md     # Dependencies, tools
├── architecture.md   # Project structure
├── ux.md             # UX patterns
├── auth/             # Auth patterns
├── forms/            # Form patterns
├── data/             # Data layer patterns
├── testing/          # Test patterns
├── deploy/           # Deployment
└── .env.example      # Environment template
```

## Usage

Autopilot reads blueprints from `~/.claude/skills/hltm-blueprints/` directly.
Blueprint is auto-selected based on project type from brief.
