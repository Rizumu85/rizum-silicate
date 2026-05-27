---
version: 1.0
name: Rizum Glass
description: Reusable Rizum Glass UI style for React + Tailwind CSS + shadcn/ui primitives, with neutral liquid glass, restrained paper rhythm, serif titles, system-sans body text, lively Apple/Arc-style motion, and tiny pointillist accents.

colors:
  primary: "#18181b"
  secondary: "#3f3f46"
  tertiary: "#2dd4bf"
  ink: "#18181b"
  ink-soft: "#3f3f46"
  ink-muted: "#71717a"
  caption: "#a1a1aa"
  canvas: "#f0f0f0"
  surface: "#ffffff"
  surface-muted: "#f4f4f5"
  surface-line: "#e4e4e7"
  glass-border: "#ffffff"
  accent-teal: "#2dd4bf"
  accent-cyan: "#67e8f9"
  accent-orange: "#f59e0b"
  accent-yellow: "#fbbf24"
  accent-violet: "#a78bfa"
  accent-rose: "#fb7185"
  accent-mint: "#86efac"
  success: "#2dd4bf"
  warning: "#f59e0b"
  danger: "#fb7185"

typography:
  display-serif:
    fontFamily: '"New York", "Noto Serif SC", "Songti SC", Georgia, serif'
    fontSize: 1.1875rem
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: 0
  title-serif:
    fontFamily: '"New York", "Noto Serif SC", "Songti SC", Georgia, serif'
    fontSize: 1rem
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: 0
  subtitle-serif:
    fontFamily: '"New York", "Noto Serif SC", "Songti SC", Georgia, serif'
    fontSize: 0.71875rem
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: 0
  body-ui:
    fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", system-ui, sans-serif'
    fontSize: 0.8125rem
    fontWeight: 400
    lineHeight: 1.7
    letterSpacing: 0
  dialogue-body:
    fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", system-ui, sans-serif'
    fontSize: 0.84375rem
    fontWeight: 400
    lineHeight: 1.8
    letterSpacing: 0
  label-ui:
    fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", system-ui, sans-serif'
    fontSize: 0.75rem
    fontWeight: 400
    lineHeight: 1.35
    letterSpacing: 0
  caption-caps:
    fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", system-ui, sans-serif'
    fontSize: 0.6875rem
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: 0.08em
  mono-note:
    fontFamily: '"SF Mono", "Fira Code", Menlo, monospace'
    fontSize: 0.75rem
    fontWeight: 400
    lineHeight: 1.8
    letterSpacing: 0

rounded:
  xs: 6px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 20px
  bubble: 18px
  full: 999px

spacing:
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 28px
  xxl: 32px

components:
  glass-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.xl}"
    padding: "{spacing.xl}"
  glass-panel-compact:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.lg}"
    padding: "{spacing.md}"
  button-neutral:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.label-ui}"
    rounded: "{rounded.sm}"
    padding: 6px 13px
  button-special:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.label-ui}"
    rounded: "{rounded.sm}"
    padding: 6px 13px
  button-muted:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.caption}"
    typography: "{typography.label-ui}"
    rounded: "{rounded.sm}"
    padding: 6px 13px
  input-field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    typography: "{typography.dialogue-body}"
    rounded: "{rounded.sm}"
    padding: 10px 14px
  quick-input-actions:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-muted}"
    typography: "{typography.label-ui}"
    rounded: "{rounded.sm}"
    padding: 0
  dialogue-bubble:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.dialogue-body}"
    rounded: "{rounded.bubble}"
    padding: "{spacing.md}"
  menu-sheet:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.lg}"
    padding: 6px
  menu-item:
    backgroundColor: "transparent"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.xs}"
    padding: 7px 12px
  metric-bar-teal:
    backgroundColor: "{colors.accent-teal}"
    rounded: "{rounded.full}"
  metric-bar-orange:
    backgroundColor: "{colors.accent-yellow}"
    rounded: "{rounded.full}"
  choice-chip:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.sm}"
    padding: 6px 10px
  segmented-track:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.label-ui}"
    rounded: "{rounded.sm}"
    padding: 4px
  creative-toggle-sound-wave:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.xs}"
    padding: 4px 8px
  range-slider:
    backgroundColor: "{colors.surface-line}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.body-ui}"
    rounded: "{rounded.full}"
    padding: "0"
---

## Overview

Rizum Glass is a reusable UI style for small, focused software surfaces that should feel present without taking over the room. The style is **neutral liquid glass**: black, white, and grey carry the interface; color appears as small pointillist accents, icon strokes, stamps, and progress fills.

The visual blend has three ingredients:

- **Liquid glass:** translucent white panels, high backdrop blur, a soft inner top highlight, and a restrained floating shadow.
- **Paper rhythm:** inset dividers, compact rows, editorial serif titles, and newspaper-like asymmetry where content benefits from character.
- **Pointillist warmth:** teal, orange, violet, rose, mint, and yellow are used as tiny dots or single-color SVG strokes. Do not fill big interface regions with accent color.

This document is the source of truth for new pages. When generating new HTML prototypes, use this file first; do not infer missing rules from earlier prototype files.

## Implementation

Use **React + Tailwind CSS + shadcn/ui primitives only** for app UI and generated prototype pages. Compose UI from React components and shadcn-style primitives such as Button, Card, Input, Select, Badge, Tabs, ToggleGroup, Popover, Dialog, ScrollArea, Separator, and Tooltip. Do not hand-roll a separate visual framework with unrelated class names when this design is being tested.

Static validation pages may be delivered as a single HTML file, but the UI inside the file should still be authored as React components, styled with Tailwind utilities, and shaped like shadcn/ui primitives. Inline CSS is only for design tokens, glass variables, keyframes, and browser setup that Tailwind cannot express cleanly.

When testing whether this design can transfer, generate a different product domain that uses similar component types. Do not keep reusing the original reference app's domain nouns, feature names, mascot concepts, or content examples. A good transfer test might be a plant lab, recipe planner, music queue, gallery intake, or studio booking tool using panels, buttons, chips, menus, metrics, and confirmation prompts.

## Colors

Use the prototype ambient background exactly unless a product surface has a strong reason not to:

```css
background:
  radial-gradient(ellipse at 20% 30%, rgba(167,139,250,.08) 0%, transparent 50%),
  radial-gradient(ellipse at 80% 70%, rgba(129,230,217,.06) 0%, transparent 50%),
  radial-gradient(ellipse at 50% 50%, rgba(251,191,36,.04) 0%, transparent 60%),
  #f0f0f0;
```

The page base must read as quiet neutral light grey first. Do not replace this with a linear gradient, a blue-green fade, or a warm bottom wash. Orange, yellow, and amber belong only to tiny accent dots, stamps, and small status marks; the canvas itself must not visibly fade into yellow.

Core surfaces are neutral:

- `surface` for white panel interiors, buttons, inputs, card faces, and popovers.
- `surface-muted` for segmented tracks, quiet chips, and inactive item backgrounds.
- `surface-line` for borders, dividers, and hairline rules.
- `ink`, `ink-soft`, `ink-muted`, and `caption` for readable hierarchy.

Accent colors must stay disciplined and small:

- `accent-teal` for confirm, success, selected state, status bars, event checks, and draw-button star icons.
- `accent-orange` or `accent-yellow` for editorial digest stamps, streaks, warm highlights, and newspaper stars.
- `accent-violet` for search panels, thought-related accents, and soft secondary dots.
- `accent-rose` for save icons, destructive-adjacent emphasis, and rare warm contrast.
- `accent-mint` for growth and fresh-state dots.

Do not create gradients inside metric or status bars. Bars use one solid accent color with slight opacity. Star shapes are not a generic panel decoration. Use stars only where the component is inherently editorial or creative, such as editorial digest stamps, card-draw buttons, or reward/stamp animations. Ordinary settings, queue, menu, status, and utility panels should use no top-right star.

## Motion

Motion should feel like Apple UI and Arc Browser: alive, spatial, and soft, never loud. Use springy easing, subtle overshoot, staggered reveals, hover lift, scale-on-press, menu items cascading top to bottom, and glass surfaces that float into place. Prefer transforms and opacity over layout shifts.

Motion rules:

- Buttons lift 1px on hover and compress slightly on press.
- Popovers, menus, and floating panels use quick scale/slide/fade entry with a soft spring curve.
- Repeated rows can stagger by 40-80ms.
- Creative controls such as sound waves can animate continuously, but quietly.
- Thinking ellipsis uses exactly three 3px dark dots, 2px gap, 1.25s ease-in-out, delays of 0ms / 160ms / 320ms, moving up 2px at 40% opacity peak.
- Continuation dots in message bubbles use three 3px pointillist dots aligned at the lower-right of the bubble, 2px gap, 1.6s ease-in-out pulse, 200ms stagger. Their animation changes opacity from .25 to .5 and translates horizontally by 2px at the midpoint. They are decorative, not the thinking ellipsis.
- Avoid neon glows, spinning loaders, bouncing icons everywhere, or motion that fights readability.

## Typography

Use sans-serif UI text for controls, dialogue, settings, menus, and dense operational panels. Dialogue text should be dark enough to read comfortably (`ink-soft` or stronger), never pale grey.

Use serif type for true titles, panel headings, menu service labels, editorial digest mastheads, tiny editorial captions, and occasional newspaper-style body copy. Do not let serif leak into button text, input text, menu commands, or ordinary operational metadata unless the component is explicitly editorial.

Service labels in menus should match the size and color of settings subtitles: small, quiet, serif, and `ink-muted`.

Button labels use `label-ui`, regular weight. Do not bold special actions such as Save, Confirm, Walk, or Open in Editor.

Use monospace only for file snippets, command-like previews, or markdown note fragments. Never use monospace for ordinary editorial stats unless the content is explicitly a code/file preview.

Prototype title rhythm:

- Main panel titles use serif, 19px, bold, `ink`, and tight but not negative tracking.
- Section subtitles such as settings groups use serif, 11.5px, semibold, `#71717a`. Do not automatically append a decorative dot after labels such as "Active Notifications", "Display", "Voice", or "Connection"; those subtitles should read cleanly on their own. They are not pale captions.
- Menu service labels use the same 11.5px semibold serif `#71717a` rhythm.
- Utility labels inside rows use sans-serif 13px for the label and 11px `caption` for numeric values or hints.

## Layout & Spacing

Keep layouts compact but breathable. The canvas may have open desktop space, but each glass panel should size itself to its content plus a little comfortable padding. Do not make all panels share one forced page width. Prefer content-aware widths using `fit-content`, `max-width`, `clamp()`, `ch` text measures, or intrinsic grid columns so each panel feels like a small floating window.

Panel sizing guidance:

- Panels that contain sentence-length text need a readable line measure: aim for roughly 28-38 Latin characters or 16-24 CJK characters per line, with no important word or button label wrapping unexpectedly.
- Forms and settings panels should be wide enough for label/value rows, segmented controls, and buttons to sit without collisions, then stop growing.
- Settings-like panels should be governed by their widest control. In the prototype, a 300px slider/control line plus 20-28px side padding gives a panel around 340-356px wide. Do not make a compact panel 420-440px wide unless the content itself needs that measure; empty right-side space feels unlike the prototype.
- Menus are compact, but they are not allowed to become cramped. Start around the original 192px menu width, then widen when a row has right-side metadata, shortcut text, or a long label.
- Search, memory, status, and health panels are compact utility surfaces, but still need enough width for row text and metadata.
- Dialogue, composer, sync, and editorial panels may be wider, but should still feel like compact desktop bubbles rather than full app rows.
- Mixed pages should look like a cluster of differently sized glass sheets, not a stack of uniform dashboard cards.

Use width tokens as semantic ranges, not fixed mandates: `menu` is narrow but has a minimum comfortable width, `utility` is compact, `dialogue` and `form` are medium, and `editorial` chooses its width from text measure. Generated prototypes must check that no text appears clipped, cramped, or colliding.

Use separators sparingly, but follow the prototype rhythm when a panel is settings-like.

Main/settings panel separator rhythm:

- Put one inset separator directly below the panel header block. The header has its own padding; the separator sits in a wrapper with the same side inset as the header (`mx-6` in the prototype).
- Inside the content area, separate major sections with `.sep`: `height: 1px`, `background: #e4e4e7`, `margin: 0 8px`, `opacity: .5`.
- Do not draw a line directly under every section title. Section title to content uses vertical spacing (`mb-1.5`, `mb-3`, or `mb-4`), not a divider.
- Do not put separators between every row inside a section. Rows are grouped with `space-y-4`, `justify-between`, and captions.
- A final small status section may follow a separator and then use `mt-5`, as in the prototype connection block.

Menu separator rhythm:

- Use `menu-sep-inset` only between command groups: `height: 1px`, `background: #e4e4e7`, `margin: 6px 12px`, `opacity: .48`.
- Do not put a separator immediately below the menu service label. In the prototype, a separator appears before the service label, then the service label and its service commands are one group.
- Menu separators must be shorter than the sheet width and never touch the top, bottom, or side edges.
- Menus should never look like tables. Do not add row dividers.

Use asymmetric composition for editorial modules, but keep the module narrow enough that it does not become a full-width article page. Editorial digests should feel like updated newspapers: compact masthead, uneven content blocks, one warm pointillist stamp, and at most one slim divider. Avoid default 1:1 dashboard grids when a story-like layout is more appropriate.

Group related action buttons together. In sync or settings rows, button groups should align left as a unit, with Save as the rightmost button inside that group.

Generated sample pages must be real UI scenes, not annotated design audits. Do not put visible text such as "generated from DESIGN.md", "design check", "this panel demonstrates", or instructions explaining the style. The page should look like a plausible product surface.

## Elevation & Depth

Glass panels use the prototype glass recipe by default: `rgba(255,255,255,.62)`, `blur(40px) saturate(200%)`, a `1px solid rgba(255,255,255,.5)` border, `0 1px 0 rgba(255,255,255,.55) inset`, and `0 20px 60px rgba(0,0,0,.06)`. The result should be more frosted and airy than opaque card UI.

Interactive rows and buttons can lift by 1px on hover and deepen the shadow slightly. Avoid colored glow around every control.

Liquid card moments, especially draw-card surfaces, may use a stronger glass recipe: higher blur, thin white borders, and a white shine sweep. Highlights should be white, not colored.

## Shapes

Default buttons and controls use 8px corners. Panels use 16-20px corners. Compact card rows may use 12px corners.

Dialogue bubbles use asymmetric corners, with one tighter corner and three rounder corners, so the bubble feels conversational without needing a tail. If the companion or speaker sits below the bubble, put the tighter corner at the lower-left (`18px 18px 18px 4px`) so the bubble reads as speech coming upward from the character rather than as a floating alert from above.

Do not overuse full pills. Reserve full rounding for tiny dots, progress tracks, and occasional compact counters.

Avoid default toggle switches whenever possible. Switches are visually heavy and usually less charming than the rest of the system. Prefer segmented controls, status chips with pointillist dots, selectable rows, icon buttons, checkable cards, or a creative domain-specific control such as a tiny animated sound-wave for voice on/off.

## Components

**Buttons:** Standard and special buttons share the exact prototype shell: white background, `surface-line` border, 8px radius, 6px 13px padding, 12px regular label, `#52525b` text, `0 1px 2px rgba(0,0,0,.02)` shadow, 180ms springy hover, 1px hover lift, and .97 active scale. A special confirm button such as "OK, go walk" must look identical to its neighboring neutral option such as "In ten minutes"; the only difference is the semantic leading icon. Hover should only deepen the neutral border to `#d4d4d8`, move the button up by 1px, and change the shadow to `0 3px 8px rgba(0,0,0,.04)`. Do not add alternate hover colors, oversized shadows, background fills, darker text, or different motion curves for adjacent prompt buttons. Do not use taller shadcn default button sizing (`h-9`, `px-4`, 13-14px labels) unless the surrounding component explicitly needs it. Ordinary buttons are usually text-only. Do not add an icon to every button.

Use icons only when they clarify a semantic action or when the button is intentionally special. Icon buttons use a 5px icon/text gap and 11px icons. Specialness comes from the leading icon only; the shell and text stay neutral and exactly match ordinary buttons. Icons must be simple SVG shapes from the accent palette. Filled micro-icons are preferred for stars and checks; thin stroke icons are acceptable for editor/open/save actions when they match the local interface language. Avoid outline-only stars.

Use these icon-color pairings:

- Confirm, walk, or completion: check-style icon in `accent-teal` when the action needs emphasis; low-stakes confirm buttons may be text-only.
- Later, upload, download, reload, test connection, and ordinary navigation: text-only by default with normal `ink-soft` text.
- Dismiss, ignore, no thanks, cancel, and opt-out actions: text-only by default with `caption` grey (`#a1a1aa`) or at most `ink-muted`; never `ink-soft`. They should visibly recede compared with the primary and neutral middle actions while keeping the same neutral white button shell.
- Save: simple save/bookmark/check icon in `accent-rose` when paired with other sync actions; text stays neutral.
- Open in Editor: editor-pencil or external-open icon in the actual pointillist palette, preferably `accent-teal` (`#2dd4bf`) or `accent-cyan` (`#67e8f9`), while the text stays neutral. Do not introduce a custom dark teal for this icon.
- Card draw or reward: filled star icon in teal or orange depending on the surrounding module; use the palette color directly, not a custom mixed fill.

For playful reward, game, inventory, or unlock surfaces, Nieobie's Game Icon Pack can be used as the preferred icon source. Treat these icons as small collectible glyphs, not as the default chrome for every button. Since the pack's SVGs are white, place reward icons on compact pointillist gem tiles from the Rizum palette (`accent-teal`, `accent-yellow`, `accent-violet`, `accent-rose`, or soft green). For mystery card backs, prefer a centered question/dice/help glyph instead of a literal card glyph; card glyphs can read as off-center grey clutter when repeated on side cards. Side-card back marks should be much quieter than the active center card, around 6-8% opacity, so the marks do not fight the face card. For neutral list rows such as recent result lists, keep the same icon source but render the frame and icon in black/white/grey only.

Every design sample should include at least one paired confirm/cancel or accept/later/dismiss group so the visual language for reversible decisions is represented, but only the emphasized action needs an icon.

**Thinking state:** Use the label "Thinking" or localized equivalent plus one animated ellipsis made from three small dark dots. Do not place a second static ellipsis in the text, and do not apply pointillist accent colors to this animation.

**Quick input:** Compact multimodal input should keep the text field as the main surface, with small icon-only actions for microphone input and visual/screenshot context when relevant. Use a plain placeholder such as `Say something...` / `说点什么...`; it gives better intent than a bare ellipsis in this compact composer. These actions sit inside the input glass on the right and should be quiet by default: no visible border, no filled button background, and neutral grey SVG strokes. Use the simplest recognizable glyphs: a minimal viewfinder/crop mark for visual context and a reduced microphone glyph with enough internal breathing room that it does not read denser than the neighboring icon. Do not add a decorative leading icon unless it represents a real state; the input should start directly with the placeholder text to reduce clutter. Do not show a microphone status dot in the resting or hover state; reserve recording dots or accent color for true active recording or an attached visual context. On hover or active states, the icon buttons may reveal a Rizum Glass affordance with translucent white fill, white border, soft shadow, and subtle lift. Do not turn quick input into a toolbar-heavy row; the whole control should remain narrow and calm.

**Asset manager:** Asset management is a practical creation surface, not a gallery wall. The first surface should choose the active asset set, offer edit actions for existing sets, and include a clear new/import entry. Do not split installed assets and new assets into unrelated areas as if they were separate products, and do not embed the whole editor under the manager list. Edit and New/Import should open a separate editor panel. Use and Edit should be visually distinct: Use can be a quiet text-only neutral action, while Edit can use the editor-pencil icon language and open the editor flow. Editors for existing and new asset sets should share the same structure. New assets can show large template thumbnails as guidance; existing assets show the currently imported file, while missing items fall back to the same template thumbnail. Template/current previews should be large enough to inspect the visual state, not tiny row icons. Keep both manager and editor compact and neutral glass. Optional feature-specific assets should use selectable chips rather than toggle switches. Imported optional art may remain in the package even when its display chip is disabled. Avoid large colored marketplace thumbnails or decorative achievement-wall layouts.

**First-use onboarding:** First-use onboarding should be a small non-blocking flow, not a full-screen setup wizard. Prefer more steps with one decision per panel over fewer dense panels: welcome, identity, preference, credentials, engine/model choice, storage, assets/extensions, and a short feature tour can each be their own quiet step when the product needs them. The onboarding container should hug the active step's content with comfortable padding instead of keeping one fixed height for every step; width may subtly adapt to text measure, while transitions remain calm. Always offer a graceful exit: use defaults, skip for now, keep local-only storage, or return later. Identity setup should help users who have no idea what to write by offering default tone chips and explaining that the profile can be edited later. Credential and model configuration can be introduced during onboarding, but they must also exist as permanent settings surfaces. Credential entry should be framed as optional and local/private by default unless the user explicitly opts into syncing or sharing. Model selection should use task-oriented labels such as daily, deeper reasoning, and vision/context rather than forcing users to understand provider internals first. Storage/sync setup must frame cloud or remote sync as optional: users who do not know what it is can postpone, and users who only use one device can explicitly choose local-only. Asset setup should reassure users that defaults are enough and custom assets can be imported later through the manager/editor. The feature tour should teach only the highest-frequency actions. Onboarding step controls should be tiny progress dots or compact glass chips with clear progress; avoid long prose, forced forms, or blocking "complete every field" behavior.

**Menus:** Menus are compact glass sheets with the original rhythm: 6px outer padding, 12px corners, item rows at 7px 12px padding, 13px regular sans text, 6px item radius, and inset separators at 6px vertical / 12px horizontal margin with ~48% opacity. The starting width is about 192px, but this is a minimum, not a maximum. If any row has right-side metadata or a shortcut, use a two-column row (`minmax(0, 1fr)` + `max-content`) with at least 12px column gap and widen the sheet enough that the left label and right metadata never touch. Right-side metadata is 9-10px `caption` grey and never wraps. Service labels are small, quiet serif subtitles in `ink-muted`. Menu commands use sans-serif regular text. Use "Reset Position" for default-position actions.

**Metrics:** Metric and status bars are thin, rounded, and solid-colored. Teal communicates healthy/progress; yellow or orange communicates warm load or streak. No bar gradients.

**Toggle alternatives:** Do not reach for a standard pill toggle first. Use:

- Segmented controls for mutually exclusive modes.
- Chips with tiny colored dots for notification categories or lightweight on/off states. Chips must be interactive: selected chips use `surface-muted`, `surface-line`, `ink-soft`, and a stronger dot; unselected chips are transparent, pale `caption` text, and a lower-opacity dot. Animate the text with the chip shell: the label should softly shift/settle and change opacity as the chip turns on or off, so the border/dot motion does not feel detached from static text. Avoid blur filters or separate hover-only text motion on chips; those create a double-flash when a hovered chip is toggled on.
- Clickable rows with a small status marker for settings lists.
- Icon buttons for immediate binary actions.
- Creative controls when the domain suggests one, such as animated sound-wave bars for voice enabled/disabled.

Segmented controls that behave like binary switches, such as show/hide or on/off display modes, should animate with a sliding white capsule inside the muted track. Do not rely on a simple fade between selected button backgrounds; the active surface should physically travel to the selected side with a short spring curve. Keep the labels static and only change their grey/ink color as the capsule moves underneath.

If a true toggle is unavoidable, make it small, quiet, and secondary; it should never become the visual focus of a settings panel.

**Sound-wave control:** Voice/sound on-off should use five 2px rounded vertical bars inside a 24px-high tap target with the prototype heights `3px / 7px / 10px / 5px / 8px`. The state difference is color and motion: off bars are pale grey `#d4d4d8` and paused; on bars become visibly darker `#a1a1aa` and animate. The motion should preserve its current frame across toggles: when turning off, keep the current bar heights and pause the wave timeline; when turning back on, hold the paused frame briefly, around 180ms, while the color changes, then continue the same timeline instead of restarting or snapping to a base frame. If CSS animation cannot resume cleanly from the paused frame, drive the heights with a tiny JS timeline that stores elapsed time. This control intentionally gets darker when enabled and softer when disabled. Use the prototype wave rhythm only in the on state: 3px baseline, 10px peak, about 1.2s, staggered by 150ms. Do not use accent colors, opacity tricks, or transform scaling for this control; animate height so the top edge feels like a real equalizer.

Place sound-wave controls like the prototype settings row: label text on the left, the sound-wave control aligned to the far right of that row with `justify-between`. It should not sit immediately after the label like an inline icon.

**Range sliders:** Use the prototype slider style for numeric settings such as opacity, speed, volume, intensity, or temperature. Track is 2px high, rounded, `surface-line` grey, with no visible colored fill by default. Thumb is a 12px grey circle (`caption`) with no white border. Its default glow is `0 0 8px 2px rgba(161,161,170,.2)`. On hover, the thumb scales to 1.3 and the glow becomes `0 0 14px 4px rgba(161,161,170,.25)`. Show the current value as muted tabular text on the right of the row. Do not use thick tracks, colored progress fills, or white-ring thumbs unless a specific product state truly requires it.

**Draw-card module:** Card-deck interactions should look liquid, layered, and physical. Preserve the card visual language: translucent white glass/paper cards, subtle white shine, soft shadows, rounded corners, and muted black/grey metadata. The motion language does not have to inherit an older prototype; when an animation feels overworked, redesign the choreography from the card style outward. A good default is a fresh redeal sequence: begin as a compact stacked deck, compress once, deal into five visible side-by-side card slots, let cards trade depth during the deal, then settle near the final selection layout before the center card lifts. During shuffle/deal phases, cards should be upright and visually straight, using clean vertical card lanes with `rotate(0deg)`; reserve angled cards for idle fan layouts or selected-card lift beats. Shuffle timing should feel closer to Apple/Arc motion than arcade motion: roughly 1.6-2.0 seconds total, with a small preparation beat, smooth spring-like easing, staggered lanes, and a gentle settle before the selected card lifts. During the moving shuffle itself, every card should look identical: use matching clean glass card backs with no decorative back marks/icons or hidden placeholder elements. Idle and drawn states should also avoid decorative card-back marks: idle can show only "Click to shuffle", drawn can show only "Click to reveal", and result icon/details should appear only after the card is revealed. If the action prompt is already on the card, do not add a second hint below the deck. Card text must match the state while staying quiet: shuffling should hide card-face text; revealed can say "Click to collect" or the domain equivalent. Do not leave idle shuffle copy visible after the deck has already selected a result. Do not make the future center card look pre-selected during the deal; hide result text and avoid a persistent hero-card shadow. The final third of the motion should already be near the dealt layout, so there is no obvious corrective slide into selection position. The selected lift should be a short arced lift with slight rotation and settle, followed by a separate reveal beat. Keep the surrounding card array fixed while the selected card lifts; do not collapse the deck during the draw transition. Do not swap result content while the deal is still resolving. Stagger card motion by a few milliseconds so the deck feels physical rather than synchronized. Keep timing constants grouped near the draw logic so CSS duration and JS reveal timing do not drift. After the user collects the revealed card, remove it from the visible deck and replenish a new card if the interaction repeats.

**Recent items:** Icon frames inside recent-item lists are grayscale only. Use black, white, and grey; do not put accent color in those frames.

**Editorial digest:** Use an editorial masthead, compact asymmetric blocks, and orange pointillist accents. The masthead should not waste vertical space by stacking every piece of metadata under the title, and it should not solve density by widening the whole panel. For a narrow digest card, borrow the feeling of a newspaper masthead without copying a reference exactly: use a large left-aligned serif title, a small source/caption row above it, plus a short right-side metadata group separated by a dashed vertical line. Avoid putting a dashed rule directly under the masthead title when the header already has a bottom divider; that creates a double-line effect. Keep metadata in the existing Rizum Glass voice: simple date, issue, contextual status, and dots in pale `caption` grey, not a long formal rail with oversized year labels and heavy rules. Horizontal rules in the digest should follow the rest of the system: inset from the left and right, usually by the panel padding plus the small separator margin, instead of spanning edge-to-edge across the glass sheet. Use a refined, lighter masthead serif stack such as `"New York Extra Large", "New York", "Didot", "Bodoni 72", "Iowan Old Style", "Noto Serif SC", "Songti SC", Georgia, serif` at roughly 650-700 weight; avoid a clunky default Georgia-heavy feel. Put quick numeric/data summary before longer narrative summary when the digest reads like a morning paper: the user should see the facts land first, then the story. The narrative column can keep a main-news hierarchy, but it should begin with a light kicker label that visually aligns with the data column label; keep story headlines around 12-13px, semibold, and `black/75-80` so they do not overpower the numeric summary. Do not place a generic star stamp in a cramped data corner; it tends to fight the masthead, vertical divider, and data summary. Use saturated orange only when the decoration has a clear editorial role. Streak or progress badges should feel like a physical editorial stamp: warm filled circle or rounded stamp shape around `#e87950`, white serif text, subtle inner dashed ring, and a quick stamp-in animation. Place stamps near the narrative/summary side of the digest, but hang them toward the outer edge or lower margin so they never block body copy. Stamp angle should be explicit and survive the stamp animation; a subtle right-tilt around 13 degrees is a good default. A stronger "odd-shaped" editorial detail works better as a tilted quote slip or small paper note in the lower digest area, where it can replace a plain quote footer without crowding the data column. Quote slips should stay in the neutral glass/paper palette with white or translucent off-white fill, neutral border and shadow; the quote mark may use a single warm accent. The card itself should stay shorter than the slip: reserve only a small lower ledge and place the slip with a negative top offset so it reads as paper attached on top, not a section the panel intentionally made room for. When a quote slip overlaps a stamp, it should sit above the stamp and can cover a small lower-left slice to create physical layering; keep the stamp label readable and keep the slip clear of body copy. Quote slips and stamps can use different final angles; do not let a stamp angle change accidentally alter the quote slip. Quote-slip stamp motion should be quick and tactile, around 340ms, after the numbers settle. Data summary numbers should create a small expectation moment: count up from zero with quick staggered easing, but do not append a checkmark. Sequence number animation before decorative stamp/slip animations so the facts settle before the editorial flourish appears.

For Chinese editorial stamps, prefer Chinese numerals when they improve the stamp texture, such as `连续 / 三天` instead of `连续 / 3 天`.

**Search panels:** Add pointillist color through tiny dots, subtle accent strokes, and section markers. Violet and rose are appropriate accents, but the panel shell remains neutral glass.

**Event streams:** Keep event-stream components restrained. For stacked completion prompts, use a single-color SVG checkmark based on the palette, preferably `accent-teal`. Do not fake pointillism by filling one shape and sprinkling random dots on top. Align the checkmark in a fixed 15px icon box.

**Creative panel triggers:** Not every creative panel should appear the same way. Reward-like or card-draw surfaces can be triggered from compact status/reward affordances. Editorial digests should feel occasional and contextual, not like a manual dashboard default. Event streams can appear as time-based or event-based fragments. Utility creative panels should be summonable through a unified panel switcher rather than separate persistent buttons in product UI. The switcher should be very small at first: a compact corner popover that slides out from a lower-left or lower-right anchor, with only small preview choices visible. Prefer a simplified vertical vinyl-record selection feeling over a generic grid menu: use compact square album-sleeve cards with a small liquid-glass record peeking from behind, not plain narrow chips or bare circular discs. Avoid a heavy container, bottom instructions, or extra launcher button inside the switcher; the sleeves/records themselves are the control. Album-sleeve glass must stay neutral and match the rest of the Rizum Glass panels; do not tint the glass itself by panel type. Keep the small color dot as the panel identifier, not only as a pseudo-decoration, so square cards preserve the earlier album-cover text rhythm: dot at the top, title in the middle, short code at the bottom. Do not draw an inner border/ring inside the album sleeve; the sleeve should read as one soft glass square, with only a faint highlight wash if needed. The record should be translucent, mostly grayscale/white, with faint rings and a larger transparent center hole indicated by one subtle neutral grey ring; do not fill the center, do not stack multiple center-ring colors, and do not color the record center with the sleeve accent. Layer the record below the album sleeve glass and text, as if it is sliding out from behind the cover rather than floating above it. Only the centered/selected sleeve should pull its record out visibly; neighboring sleeves keep their records tucked mostly inside the sleeve with lower opacity so the selector reads as a stack of albums, not a row of exposed discs. The centered sleeve is the current selection; neighboring sleeves should still read as real album cards, not tiny background shadows: use modest scale contrast, moderate opacity, and restrained blur/depth. When a selection opens, the glass record may spin subtly to imply activation. The selected panel can then unfold from that same corner into the full glass panel, preserving a clear sense of spatial summoning. The switcher should support direct selection, keyboard arrows, vertical trackpad swipes, and mouse/touch dragging. It must be data-driven so future panels can be added as one more switcher item instead of a new bespoke trigger.

**Achievement-like grids:** Avoid achievement walls or badge galleries unless the whole concept is redesigned around calm, editorial restraint. Dense badge grids tend to compete with the softer system style.

## Do's and Don'ts

**Do**

- Keep most UI surfaces black, white, grey, and translucent.
- Use pointillist color sparingly: dots, icon strokes, stamps, and tiny markers.
- Let serif typography appear in titles, service labels, and newspaper-like modules.
- Use white shine and blur for liquid-glass emphasis.
- Keep special buttons visually consistent; only the leading icon changes.
- Include confirm/cancel examples in generated design-sample pages.
- Leave ordinary buttons text-only unless the icon carries semantic value.
- Keep sample pages narrow unless a feature genuinely needs a wide canvas.
- Let each panel hug its own content instead of stretching all panels to the same width.
- Replace ordinary toggles with segmented controls, chips, clickable rows, or creative state controls.
- Make chip/toggle alternatives actually interactive in prototypes.
- Include a range slider sample when documenting numeric settings.
- Generate real product UI, not explanatory style-demo copy.
- Create new prototype files with a new version number instead of overwriting older versions.

**Don't**

- Use large accent blocks, heavy full-page color washes, or purple-dominant gradients.
- Bold Save, Confirm, Walk, or Open in Editor labels.
- Add two ellipses to the thinking state.
- Use gradients inside metric or status bars.
- Stretch a narrow focused interface into a full-width dashboard.
- Force every panel to share the same width.
- Default to pill toggle switches for settings.
- Leave chip-style switches visually static or non-interactive.
- Animate sound-wave controls with transform scaling instead of height changes.
- Use outlined star icons where a simple filled shape would do.
- Add decorative icons to every button.
- Add separators after every small content block.
- Put visible design-process or style-instruction text inside UI samples.
- Put accent color inside recent-item icon frames.
- Use emoji as production chrome for checks, achievements, or event icons.
- Reintroduce the achievement wall unless the whole concept is redesigned.
