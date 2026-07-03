# BP1 Pro Desktop — UX Glow-Up (motion pass)

**Date:** 2026-07-03
**Status:** Approved (interactive mockup reviewed & signed off)
**Reference mockup:** `docs/superpowers/specs/assets/bp1-ux-preview.html` (self-contained; the approved visual/behavioral source of truth)

## Goal

Elevate the app from "clean and static" to "clean and alive" without changing its
visual identity. Add motion, delight, and micro-interactions on top of the existing
dark/indigo design system. Ship alongside the game-mode feature (issue #3) so the
new toggle debuts with a proper animated control instead of the plain `ON/OFF` text.

## Non-goals (explicitly out of scope)

- No new color story, no layout/identity redesign, no glassmorphism/gradients-as-identity.
- No changes to protocol, transport, or Rust backend behavior.
- Settings tab visuals unchanged.
- No new dependencies — motion is CSS + native Web Animations / Solid reactivity only.

## Principles

1. **Honor the existing system.** Ground `#0d0d0f`/`#0a0a0c`, cards `#111113`,
   borders `#1a1a1e`, dividers `#161618`, accent `#6366f1` (+ `#818cf8`/`#a5b4fc`/`#c7d2fe`),
   semantic `#22c55e`/`#ef4444`/`#eab308`. These are fixed; motion never recolors them.
2. **Tasteful, not maximal.** Restraint is the point — subtle glow and spring, not
   constant animation. Intensity is tunable during build.
3. **Accessible by default.** Every looping/ambient animation and every non-trivial
   transition collapses under `prefers-reduced-motion: reduce`.

## Motion system (shared tokens)

Add a small set of shared constants (durations + easings) so motion is consistent:

- `--ease-spring: cubic-bezier(.34, 1.56, .64, 1)` — knobs, checkmarks, tab marker (overshoot)
- `--ease-out-soft: cubic-bezier(.22, .61, .36, 1)` — panel entrance, ring sweep, bars
- Durations: micro 120–160ms (hover/press), state 240–340ms (toggles, tab swap),
  entrance 1.0–1.1s (ring count-up on mount only).

## Per-component behavior

| Component | Enhancement |
|---|---|
| **Game mode toggle** (`AncTab`) | Replace `ON/OFF` text button with a sliding pill switch: knob translates with spring, track fills with accent, `state` label flips ON/OFF. When ON: card gets an ambient accent box-shadow + a slow rotating conic-gradient halo (masked border) and the 🎮 icon nudges scale/rotate. A brief scale-pulse fires on click. **Tasteful intensity by default.** |
| **Battery rings** (`HomeTab`/`BudRing`) | On mount (and each time the Battery tab is shown), sweep the progress arc from 0 and count the number up from 0 with an ease-out over ~1.1s. Keep the existing green/red semantic stroke; add a soft `drop-shadow` glow. Charging bolt pulses. Case bar fills from 0% on entrance. |
| **Sidebar marker** (`Sidebar`) | Single active-indicator element that glides (`top`, spring) between icons rather than one-per-button hard cut; add a faint accent glow. Icons lift slightly on hover. |
| **Tab panels** (`App`) | Panel entrance = fade + 8px lift with `--ease-out-soft` on tab change. |
| **Cards / buttons** | Hover: `translateY(-1px)` + border lighten + soft shadow. Press: scale 0.97. Find buttons emit a small accent ripple at the click point. |
| **ANC mode cards** | Selected card keeps existing indigo wash; add a checkmark that pops in with spring, and an accent glow on the selected card. Strength slider expands/collapses (height+opacity) only for ANC/Transparency. |
| **Header** | Very subtle indigo glow that breathes behind the title bar; connection status dot gently pulses. |

## New material choice

- **Monospace for numeric readouts** (battery %, strength level, session timer) via a
  system mono stack (`ui-monospace, "SF Mono", "Cascadia Code", "Segoe UI Mono", Menlo, monospace`)
  with `tabular-nums`. Technical-instrument feel; no webfont, no CDN, no fallback risk.

## Reduced-motion contract

Under `prefers-reduced-motion: reduce`: disable the header breathe, status pulse,
charging-bolt flash, and the game-mode rotating halo (static glow remains); rings and
bars jump straight to final value; transition durations collapse to ~0. State is always
reachable without motion.

## Ships with

- Version bump (0.2.1 → **0.3.0**, user-facing feature + UX pass) in
  `apps/baseus-app/src-tauri/Cargo.toml`, `tauri.conf.json`, `package.json`, and `Cargo.lock`.
- Changelog entry covering game/low-latency mode, the firmware-tolerant ANC ack fix
  (issue #3), and the UX motion pass.

## Verification

- `tsc --noEmit` clean; `cargo fmt`/`clippy` unaffected (frontend-only + version files).
- Manual: run the app, confirm each interaction animates and that forcing
  reduced-motion (OS setting) degrades gracefully. Screenshot before/after for the PR.
