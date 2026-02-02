# UX Patterns

## Layout

Standard three-column layout with app bar, sidebar, main content, and optional detail panel.

## View State

Use body classes to toggle view states:
- `detail-open` - Show detail panel
- `settings-open` - Show settings view

## Components

- Modals - Centered overlay dialogs
- Buttons - Primary, secondary, danger variants
- Inputs - Text, select, toggle switches
- Empty states - Icon + message + action

## Theming

Use CSS variables for colors. Support light/dark via `data-theme` attribute on root.
