# UX Design Specification - NBP

**Author:** sk
**Date:** 2026-02-04
**Version:** v0.4.1 (Code Review Bug Fix Sprint)

---

## Design Philosophy

**Core Principle:** Zero-friction audio capture with instant feedback. Every interaction should feel immediate and confident.

**Design Pillars:**
1. **Clarity** -- State is always obvious (recording, idle, processing)
2. **Focus** -- Hide irrelevant controls contextually
3. **Feedback** -- Real-time visual response to audio input
4. **Accessibility** -- WCAG AA compliant, keyboard-first

---

## User Personas

### Sarah -- The Meeting Pro
- **Role:** Product Manager
- **Context:** Daily standups, cross-team syncs
- **Need:** Capture everything, extract action items
- **Pain:** Missing notes while participating
- **Goal:** Record -> Process -> Paste to Notion in <5 min

### Alex -- The Thinker
- **Role:** Solo Founder
- **Context:** Brainstorms, late-night ideas, walks
- **Need:** Capture stream-of-consciousness, structure later
- **Pain:** Ideas lost before written down
- **Goal:** 45min ramble -> 2 pages of structured thinking

### David -- The Reliable
- **Role:** Client-facing role
- **Context:** Important calls, can't miss anything
- **Need:** Confidence that recording works
- **Pain:** Fear of losing critical conversations
- **Goal:** Know immediately if something goes wrong

---

## Information Architecture

```
NBP
+-- App Bar (fixed top)
|   +-- Logo + Version
|   +-- Permission Warning (conditional)
|   +-- Capture Section
|   |   +-- Status Indicator
|   |   +-- Timer
|   |   +-- Recording Waveform (during recording)
|   |   +-- Record Button
|   +-- Detail Controls (when detail open)
|       +-- Folder Button
|       +-- Delete Button
|
+-- Main Layout (grid)
|   +-- Sidebar (280px)
|   |   +-- Tags Filter
|   |   +-- Pipeline Filter
|   |   +-- Settings Button
|   |
|   +-- Recordings List (center)
|   |   +-- Recording Items
|   |
|   +-- Detail View (full-width when open)
|       +-- Back Button
|       +-- Title Input
|       +-- Tags Section
|       +-- Audio Player
|       +-- Pipeline Assignment
|       +-- Transcript Section
|       +-- Pipeline Status & Outputs
|       +-- AI Processing Section
|
+-- Overlays
    +-- Delete Confirmation Modal
    +-- Onboarding Modal
    +-- Settings View
        +-- General Settings
        +-- Transcription Settings
        +-- Pipelines & Templates
```

---

## Visual Design System

### Typography

| Element | Font | Weight | Size |
|---------|------|--------|------|
| Logo | Outfit | 800 | 1.4rem |
| Headings | Outfit | 700-800 | 1.15-2.2rem |
| Body | Outfit | 400 | 1rem (16px) |
| Mono | JetBrains Mono | 400-500 | 0.85-0.9rem |
| Labels | Outfit | 600-800 | 0.8-0.85rem |

### Color Themes

#### Neon Purple (Default)
| Token | Value | Usage |
|-------|-------|-------|
| --bg-primary | #07050f | Main background |
| --bg-sidebar | #0e0a1f | Sidebar background |
| --bg-card | #15102e | Card surfaces |
| --bg-input | #1e1740 | Input fields |
| --text-primary | #ffffff | Primary text |
| --text-secondary | #c4b5fd | Secondary text |
| --accent | #a855f7 | Interactive elements |
| --recording | #ff2d55 | Recording state |
| --border | rgba(168, 85, 247, 0.3) | Borders |

#### Deep Obsidian
| Token | Value | Usage |
|-------|-------|-------|
| --bg-primary | #0a0a0a | Main background |
| --bg-sidebar | #121212 | Sidebar background |
| --bg-card | #1a1a1a | Card surfaces |
| --text-primary | #e0e0e0 | Primary text |
| --text-secondary | #888888 | Secondary text |
| --accent | #cccccc | Interactive elements |

#### Deep Blue
| Token | Value | Usage |
|-------|-------|-------|
| --bg-primary | #0a1628 | Main background |
| --bg-sidebar | #0f1d32 | Sidebar background |
| --bg-card | #152238 | Card surfaces |
| --text-primary | #e8f0ff | Primary text |
| --text-secondary | #8ba3c7 | Secondary text |
| --accent | #3b82f6 | Interactive elements |
| --recording | #ef4444 | Recording state |

### Spacing Scale

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Tight gaps |
| sm | 8px | Component internal |
| md | 16px | Between elements |
| lg | 24px | Between sections |
| xl | 32px | Major sections |

### Border Radius Scale

| Token | Value | Usage |
|-------|-------|-------|
| --radius-sm | 6px | Buttons, inputs |
| --radius-md | 10px | Cards |
| --radius-lg | 16px | Large cards |
| --radius-xl | 24px | Modals |

---

## Component Specifications

### Record Button

**States:**
- **Idle:** Circle icon, accent color text, transparent background
- **Recording:** White text, recording color background, pulsing glow animation
- **Disabled:** 50% opacity, no hover effects

**Behavior:**
- Click toggles recording on/off
- During recording: 1.5s glow animation cycle
- Keyboard: Space bar (when focused)

### Recording Waveform

**Location:** App bar, between timer and record button

**Specification:**
- Canvas: 40x20px
- 8 vertical bars, 3px wide, 2px gap
- Color: --accent
- Animation: Bars pulse 8-24px height, 0.5s cycle
- Staggered animation delays (0s-0.7s)

**States:**
- **Recording:** Animated bars responding to audio level
- **Paused:** Flat line (8px height)
- **Not Recording:** Hidden (display: none)

### Status Indicator

**States:**
- **Idle:** 8x8px circle, bg-card fill, border
- **Recording:** 8x8px circle, recording color, pulsing glow, 1.2s animation

### Audio Player (Detail View)

**Specification:**
- Play button: 44x44px circle, accent background
- Time display: JetBrains Mono, current/total format
- Waveform canvas: Full width, 80px height, clickable for seek

**Hidden when:** `body.is-recording-active`

### Microphone Selector

**Location:** Settings -> Transcription section

**Specification:**
- Dropdown select with device names
- Shows: Device name (type indicator)
- Example: "MacBook Pro Microphone (Built-in)"
- Example: "AirPods Pro (Bluetooth)"

**Constraints:**
- Disabled during active recording
- Updates on device connect/disconnect
- Persists selection in settings

---

## Interaction Patterns

### State-Based UI Hiding

```
Recording Active (body.is-recording-active):
+-- SHOW: Capture section (status, timer, waveform, record btn)
+-- SHOW: Recording waveform visualization
+-- HIDE: Audio player section
+-- HIDE: Detail controls (folder, delete)
+-- DISABLE: Microphone selector

Detail View Open (body.detail-open):
+-- HIDE: Sidebar
+-- HIDE: Recordings list
+-- SHOW: Detail view (full width)
+-- IF NOT recording: SHOW detail controls
```

### Theme Switching

```javascript
// Pattern: Remove all, add new
document.body.classList.remove('deep-obsidian', 'deep-blue');
document.body.classList.add(themeName);
```

### Recording Flow

1. **Start:** Click record -> Immediate state change -> Timer starts
2. **Active:** Waveform animates -> Status indicator pulses
3. **Stop:** Click record -> Timer stops -> Recording saved -> Refresh list

### Error States

| State | Visual | Action |
|-------|--------|--------|
| Permission missing | Yellow warning banner | "Fix" button opens settings |
| Recording failed | Red indicator | Error message in detail |
| API failure | Toast notification | Retry option |

---

## v0.4.1 Bug Fix Sprint - UX Changes

### Overview

This sprint has **minimal UX impact**. The changes are primarily backend fixes and security hardening. The following user-visible changes are documented:

### Change 1: Recording Duration Display Fix (FR-CR7)

**Before:** Recordings saved with `save_mix_only` mode (the default) showed duration `0:00` in the recordings list.

**After:** Recordings correctly display the mix duration.

**Affected Component:** Recordings list item, duration display.

**Visual Change:** None -- the same element now shows the correct value instead of zero.

### Change 2: XSS Prevention in User Content (FR-CR2)

**Before:** Tag names and recording titles were rendered without HTML escaping.

**After:** All user-provided content is escaped via `escapeHtml()` before innerHTML insertion.

**Visual Change:** None for normal usage. Tags or titles containing HTML-like characters (`<`, `>`, `&`, `"`) will display as literal text instead of being interpreted as HTML.

### Change 3: Faster App Startup (FR-CR10)

**Before:** Permission check on every launch created a real system audio recording for 500ms, adding noticeable startup delay.

**After:** Lightweight permission check without creating actual recordings.

**Visual Change:** None -- the onboarding and permission UI remain identical. The app simply reaches the ready state faster.

### Change 4: No API Keys in Console (FR-CR17)

**Before:** Full settings object (including API keys) logged to WebView console.

**After:** Settings logging either removed or redacted to exclude sensitive fields.

**Visual Change:** None for the application UI. Only affects developer console output.

---

## Accessibility Requirements

### Contrast Ratios (WCAG AA)

| Element | Requirement | Verification |
|---------|-------------|--------------|
| Primary text on bg | 4.5:1 minimum | All themes |
| Secondary text on bg | 4.5:1 minimum | All themes |
| Accent on bg | 3:1 for large text | All themes |
| Focus indicators | 3:1 | 2px outline |

### Keyboard Navigation

| Key | Action |
|-----|--------|
| Tab | Move between focusable elements |
| Space/Enter | Activate buttons |
| Escape | Close modals, detail view |
| Arrow keys | Navigate lists, dropdowns |

### Focus Indicators

```css
:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

button:focus-visible {
  box-shadow: 0 0 0 4px var(--accent-soft);
}
```

---

## Responsive Behavior

### Minimum Viewport
- Width: 320px
- Height: 100vh

### Layout Breakpoints
- Sidebar: Fixed 280px
- Content: Fluid, max-width 900px for readability

### Overflow Handling
- Body: overflow hidden
- Detail scroller: overflow-y auto
- Recordings list: overflow-y auto

---

## Animation Specifications

### Transitions

| Type | Duration | Easing |
|------|----------|--------|
| Fast | 150ms | ease |
| Normal | 200ms | ease |
| Smooth | 300ms | cubic-bezier(0.4, 0, 0.2, 1) |

### Keyframe Animations

**pulse-dot (Status Indicator):**
```css
0% { transform: scale(1); opacity: 1; }
50% { transform: scale(1.4); opacity: 0.5; }
100% { transform: scale(1); opacity: 1; }
```

**waveform-pulse (Recording Bars):**
```css
0%, 100% { height: 8px; opacity: 0.5; }
50% { height: 24px; opacity: 1; }
```

**recording-glow (Record Button):**
```css
0%, 100% { box-shadow: 0 0 20px rgba(255, 45, 85, 0.4); }
50% { box-shadow: 0 0 30px rgba(255, 45, 85, 0.7); }
```

**expandIn (View Transitions):**
```css
from { opacity: 0; transform: scale(0.96) translateY(10px); }
to { opacity: 1; transform: scale(1) translateY(0); }
```

---

## Pipeline Management UI

### Pipeline Status Indicator (Detail View)

**Location:** Recording detail view, below transcript section

**Specification:**
- Pipeline name: Outfit 700, 1rem
- Status badge: Inline colored pill
  - `waiting` -- text-secondary, border
  - `running` -- accent color, subtle pulse animation
  - `done` -- green (#22c55e), solid background
  - `partial` -- amber (#f59e0b), solid background
- Expand/collapse: Click pipeline row to see step details

**Step Output List:**
- Vertical list of steps with status icons
- Step name: Outfit 600, 0.9rem
- Step connector type: JetBrains Mono, 0.8rem, text-secondary
- Status icon: 16x16px (checkmark for done, X for failed, spinner for running, clock for pending)
- Click step name to view output content in a panel below

**Actions:**
- "Re-run" button per failed step (icon + text)
- "Re-run All" button for entire pipeline
- "View Output" link per completed step

### Pipeline Assignment (Detail View)

**Location:** Recording detail view, between tags and audio player

**Specification:**
- Dropdown select: "Assign Pipeline..."
- Shows pipeline name and step count
- Selected pipelines listed as removable chips (similar to tags)
- Remove pipeline: X button on chip

**Behavior:**
- Assigning a pipeline with transcript available auto-triggers execution
- Assigning without transcript sets status to "waiting"
- Cannot assign same pipeline twice

### Pipeline & Template Management (Settings View)

**Location:** Settings view, new "Pipelines" tab/section

**Sections:**

**1. Pipeline Definitions:**
- Card list of defined pipelines
- Each card shows: name, description, step count, connector types used
- "Create Pipeline" button (accent color)
- Click card to edit
- Delete via icon button with confirmation

**Pipeline Editor (Inline):**
- Pipeline name input
- Pipeline description textarea
- Steps list (ordered, numbered)
  - Each step: name, connector dropdown (llm/save/webhook/mcp), input dropdown (transcript or previous step names), config section
  - LLM config: prompt template dropdown, provider dropdown, model input
  - Save config: path input with variable hints
  - Webhook config: URL input, method dropdown, headers JSON
- "Add Step" button
- Drag handle for reordering steps (or up/down arrows)
- "Save Pipeline" button
- Validation errors shown inline in red

**2. Prompt Templates:**
- Card list of templates
- Each card shows: name, description, truncated prompt preview
- "Create Template" button
- Built-in templates marked with "Built-in" badge

**Template Editor (Inline):**
- Template name input
- Template description input
- Prompt textarea (monospace font, auto-resize)
- Variable hint: "Use {transcript} to insert recording transcript"
- "Save Template" button
- Delete with confirmation (warns if template in use by pipelines)

### Pipeline Sidebar Filter

**Location:** Sidebar, below tags filter

**Specification:**
- Section header: "Pipelines" with filter icon
- List of pipeline names as toggleable filter chips
- Filter recordings that have the selected pipeline assigned
- Multiple pipeline filters combine with OR logic
- Active filter highlighted with accent color

### Step Output Viewer

**Location:** Recording detail view, expandable panel below pipeline status

**Specification:**
- Renders step output markdown content
- Monospace font for frontmatter display (collapsed by default)
- Body content rendered as formatted markdown
- Copy button to copy output to clipboard
- Max height with scroll for long outputs

### Pipeline Progress Events

**Real-time Updates:**
- Pipeline status updates via Tauri events
- Step progress shows animated spinner
- Completion triggers status badge color change
- No page refresh needed (live updates)

---

## Design Tokens Summary

```css
:root {
  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;

  /* Radius */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 16px;
  --radius-xl: 24px;

  /* Transitions */
  --transition-fast: 0.15s ease;
  --transition-normal: 0.2s ease;
  --transition-smooth: 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
```

---

## Implementation Notes

1. **No bundler** -- CSS variables for theming, vanilla JS
2. **Body class state** -- `is-recording-active`, `detail-open`, `settings-open`
3. **Tauri IPC** -- `window.__TAURI__.invoke()` for backend calls
4. **Font loading** -- Google Fonts (Outfit, JetBrains Mono)
