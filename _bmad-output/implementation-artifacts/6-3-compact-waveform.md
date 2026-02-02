# Story 6.3: Compact Waveform Visualization

Status: done

## Story

(Implicit from FR63: Waveform visualization is compact and non-intrusive)

As a user,
I want the waveform visualization to be compact and non-intrusive,
So that it doesn't distract from the main recording interface.

## Acceptance Criteria

1. **Given** I am recording **When** I view the waveform **Then** it is small and positioned near the record button
2. **Given** the waveform is visible **When** I look at the interface **Then** it doesn't obscure other UI elements

## Tasks / Subtasks

- [x] CSS: Compact waveform dimensions (already implemented)
  - [x] 40x20 pixels canvas size
  - [x] Positioned next to timer in capture section
  - [x] 4px border radius for soft appearance

## Dev Notes

### Already Implemented

**Dimensions (index.html:33):**
```html
<canvas ... width="40" height="20">
```

**Styling (styles.css:1475-1480):**
```css
.recording-waveform-canvas {
  width: 40px;
  height: 20px;
  border-radius: 4px;
  background: var(--bg-input);
}
```

**Position:**
- Inside `.capture-section` next to timer
- Flexbox layout with small gap (4px)

### Why This Story is Pre-Done

The compact design was part of the original waveform implementation.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Completion Notes List

1. ✅ Verified compact 40x20px canvas size
2. ✅ Verified non-intrusive positioning near record button
3. ✅ No code changes required

### File List

**Reviewed (no changes needed):**
- `src/index.html` - Canvas dimensions
- `src/styles.css` - Compact styling
