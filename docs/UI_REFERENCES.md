# UI References

This document explains which ProcreateViewer prototypes guide the Rizum Silicate
UI, what each reference is allowed to influence, and which inherited Silicate
controls need product review before they appear in the new UI.

## Primary Visual Reference

### `concept18_rizum_glass_perfect.html`

Use this as the main product UI reference.

It defines the target first impression:

- top-left file pill and open affordance
- large centered canvas surface
- bottom dock with Canvas, Layers, Playback, Info, Export, and Settings
- right-side glass panels for layers/info/export/settings
- compact bottom playback surface
- soft neutral Rizum Glass palette
- restrained serif headings with system-sans body text
- icon-led navigation rather than dense technical menus

When in doubt, match this prototype's information hierarchy before borrowing
ideas from older concepts.

## Animation and Motion References

These are motion references, not full layout references.

### `concept22_playback_morph_focus.html`

Use for playback behavior and focus transitions:

- playback control morphs from a compact dock action into an active control
  surface
- frame/timeline controls feel close to the canvas instead of buried in a modal
- animation state should be visually obvious without stealing the whole screen
- the play/pause action should feel immediate and tactile

Do not copy mock data or unfinished layout details blindly. Use it to shape the
Animation Assist HUD and playback panel.

### `concept23_rizum_glass_animated_panels.html`

Use for panel choreography:

- side panels enter/exit smoothly
- dock state, active panel state, and panel position feel connected
- export/settings panels can feel alive without becoming decorative
- panel motion should help orientation, not become spectacle

Use this when implementing egui panel transitions or animated state changes.

### `concept21_rizum_glass_animated.html`

Use as a secondary animation reference only if a motion detail is missing from
`concept22` or `concept23`.

## Design System Reference

### `docs/ux-prototypes/DESIGN.md`

Use `docs/ux-prototypes/DESIGN.md` as the durable style contract, adapted from
React/Tailwind terms to egui primitives.

Important portable rules:

- neutral glass/paper surfaces, not saturated one-note palettes
- restrained accent color, mostly teal/cyan with small warm accents
- serif titles and system-sans body labels
- compact panels, tight but readable rows
- icons for common actions where possible
- segmented controls, chips, and selectable rows instead of heavy default
  toggles
- no visible instructional/design-process text inside the app UI
- no decorative bokeh/orb backgrounds
- avoid card-in-card nesting
- keep text fitting inside controls at all window sizes

React/Tailwind/shadcn-specific implementation notes in
`docs/ux-prototypes/DESIGN.md` are not literal requirements for this egui fork.
Translate the intent into egui widgets and custom painters.

## Inherited Silicate UI Review

Silicate has useful capability, but some controls are more technical than a
normal artist-facing viewer needs. Do not automatically expose every inherited
control in the primary UI.

### Keep in Primary UI

These should appear in the new UI because they are directly useful:

- open file
- document tabs or a clear multi-document switcher
- layer/folder visibility toggles
- nested folder expand/collapse
- mask visibility where a mask exists
- background color visibility
- export current artwork
- basic document info
- animation playback once implemented
- settings/system integration status

### Rebrand and Simplify

These are useful, but should be redesigned into Rizum UI language:

- layer opacity slider
- blend mode picker
- clipped layer toggle
- background color picker
- current-view export button
- layer/group/mask preview thumbnails
- theme preference
- sampling mode if kept
- canvas fit/reset controls
- multi-file tabs

### Advanced or Needs User Decision

These inherited Silicate controls are powerful but technical. Ask before making
them visible in the default UI:

- Grid View
- Extended Crosshair
- Sampling: Nearest/Linear
- free rotation slider
- middle-drag rotation gesture
- horizontal/vertical canvas flip controls
- boxed zoom gesture
- manual blend-mode editing for a viewer-focused workflow
- manual clipped toggle editing
- background color editing, as opposed to background visibility only
- low-level debug/performance indicators
- web demo controls in the native app

Suggested default: keep these behind an `Advanced` or `Debug` section, or hide
them until there is a clear artist workflow.

## Product Questions For Rizum

Before implementing the full new UI, confirm these decisions:

1. Should the primary layer panel allow editing blend mode, opacity, clipped,
   and background color, or should it start as a safer viewer-only panel with
   visibility controls?
2. Should grid/crosshair/sampling/rotation live in Settings, an Advanced canvas
   menu, or be removed from the default UI?
3. Should multi-file tabs remain as visible tabs, or become a quieter file
   switcher/history surface matching the Rizum dock layout?
4. Should current-view export remain, or should export default to full-canvas
   artwork with current-view as an advanced option?
5. Should the web build keep the same UI as native, or be a simpler viewer with
   fewer integration/export controls?

## Implementation Notes For egui

- Prefer one egui app shell. Do not introduce React/Tauri.
- Use custom egui painters for glass panels, dock pills, thumbnails, and small
  icons when default widgets look too technical.
- Keep the WGPU canvas and egui HUD in one compositor.
- Keep all debug controls explicit and discoverable only when requested.
- Treat the prototypes as interaction sketches, not a requirement to recreate
  HTML/CSS implementation details.
