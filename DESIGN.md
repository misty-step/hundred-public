# Hundred — pigment, not polish

## Intent
A small instrument for accumulating effort. Open it, put down a set, return to your day. The hundred-rep constraint gives the interface its shape. Static ASCII pigment gives the tracker its visual identity.

## Structure
A single 980×850 native window, minimum 760×640. Top: Hundred, local date, honest sync state. Main: three exercise rows at left, a quiet character-field at right. Each row has exercise name, exact total / 100, remaining reps and a ten-cell progress trace. Selected exercise has a coloured left edge and a focus outline. A single logging strip below provides exercise, numeric rep entry, presets 10/20/25 and Log set. Bottom: 28-day trace, current/best streak and recent sets with explicit Undo. History replaces the main content with a scrollable day ledger; Escape returns. At shorter window heights, the lower history and keyboard hints scroll; the logging strip remains within the initial 640px view.

## Visual language
- Background `#181c24`, raised surface `#232935`, text `#e1e6ef`, muted `#9daac0`, borders `#46536a`.
- Push-ups mineral rose `#eca6b1`, Crunches ochre `#e9c27b`, Squats cool periwinkle `#b4b9f0`. These three colours coexist; no single neon accent.
- Monospaced system typography: JetBrains Mono with monospace fallback. Body 14px, controls 16px, exercise labels 20px. No giant metric hero, all-caps labels or card grid.
- One expressive element: a 48×20 character pigment field. Its three static overlapping clouds gain density with the actual three totals. Characters ` .:-=+*#%@` carry density; colour carries exercise. It never moves independently or masquerades as measured progress.
- Gentle 6px control corners; no shadows, blur, gradients, decorative icons or surprise movement. The hundred-dot trace is the small brand mark.

## Interaction and copy
Enter logs the selected exercise. Tab / Shift+Tab change exercise. Ctrl+1/2/3 selects directly. Up/Down adjusts reps; typing digits replaces the initial number. Ctrl+Z undoes the latest active set; Ctrl+R syncs; Ctrl+H opens history; Escape returns; Ctrl+Q quits. Hints remain visible in the footer. Controls are at least 44px high. Selection and completion also have words, not just colour.

Confirm “Saved 20 squats · 80 left” only after disk commit. Network failures say “Saved here · 1 waiting to sync”. Authentication failures say “Sync credential rejected. Local sets are safe.” A blank ledger says “Your first set starts here.” A completed day says “Today's hundred, done.” Over-goal reps remain visible. No invented encouraging streaks.

## Accessibility and states
Exact counts, visible selected state, full keyboard route, high-contrast text, static art, scrollable history. GPUI 0.2.2's screen-reader exposure is not claimed; assistive-technology acceptance is a remaining framework limitation. Test empty, partial, complete, over-goal, offline, pending, rejected credential, undo, reopened, long history, narrow and restored states on the real executable. Synthetic data only in captures.
