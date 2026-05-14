# UI Redesign — Minimal Pro Design Spec

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the basic battery-card layout with a polished, full-featured Minimal Pro UI — dark, refined, single-window app with ANC control, battery sparklines, find-earbuds, and settings.

**Architecture:** All new UI lives in `apps/baseus-app/src/` (SolidJS + Tailwind). The Rust backend gains two new Tauri commands (`set_anc_mode`, `find_earbud`) and a persistent settings store. The frontend gains a `BatteryHistory` store that buffers the last 60 readings per component for sparklines.

**Tech Stack:** SolidJS, Tailwind CSS v4, Tauri v2, `tauri-plugin-autostart`, `tauri-plugin-notification`

---

## Visual Design

**Style:** Minimal Pro — dark (`#0d0d0f` background), refined rings, clean typography, no decorative chrome.

**Window size:** 380 × 620px, not resizable, not always-on-top.

**Colour tokens:**
- Background: `#0d0d0f`
- Surface: `#111113`
- Border: `#1a1a1e`
- Divider: `#131315`
- Text primary: `#ffffff`
- Text secondary: `#a3a3a3`
- Text muted: `#525252`
- Green (buds healthy): `#22c55e`
- Red (low battery): `#ef4444`
- Indigo (case / ANC active): `#6366f1`

**Low-battery threshold:** < 20% → ring stroke switches to red.

---

## Layout (top to bottom)

### Title bar
- macOS-style traffic-light dots (decorative, no function on Windows — just visual polish)
- Centred device name: `Bass BP1 Pro ANC` (static, dimmed)
- Right-aligned connection status: green dot + `Connected` / pulsing yellow + `Connecting…` / dimmed `Disconnected`

### Earbuds section
Label: `EARBUDS`

Two `BudCard` components side by side:
- Circular SVG ring (80px, stroke-width 6, animated on value change, green → red below 20%)
- Percentage number centred in ring
- `SparkLine` component below ring (100×24px SVG, last 60 readings, 1-minute cadence, green/red matching ring)
- Label: `LEFT` / `RIGHT`

### Case section
Label: `CASE`

Single `CaseCard`:
- 52px ring using indigo stroke, same ring rules
- `SparkLine` to the right (wider, same data rules)

### Noise Control section
Label: `NOISE CONTROL`

Three `AncButton` components in a row:
- `Off` (🔇), `ANC` (🎧), `Transparent` (🌬️)
- Active state: indigo tint background + border + label colour
- Clicking an inactive button sends `set_anc_mode` Tauri command
- While the command is in-flight the clicked button shows a subtle pulse; on `anc_mode_update` event the UI confirms

ANC mode commands (from APK bytecode analysis):
- Off: write `[0xAA, 0x30, 0x00]` to GATT write char (reflected by `AA 30 00` notification)
- ANC: write `[0xAA, 0x33, 0x01, 0x68]`
- Transparent: write `[0xAA, 0x32, 0x02, 0xFF]`

### Find Earbuds section
Label: `FIND EARBUDS`

Two `FindButton` components side by side:
- `Play Left` / `Play Right`
- Sends `find_earbud` Tauri command with `side: "left" | "right"`
- Button shows loading state for 3 s then resets (device plays a tone, no acknowledgement frame expected)

Find command bytes (from APK `BA10XXYY` analysis):
- Left: `[0xBA, 0x10, 0x01, 0x00]`
- Right: `[0xBA, 0x10, 0x01, 0x01]`

### Settings section
Label: `SETTINGS`

Two toggle rows:

| Label | Description | Default | Implementation |
|---|---|---|---|
| Launch at login | Start automatically with Windows | on | `tauri-plugin-autostart` |
| Low battery alerts | Notify when a bud drops below 20% | on | `tauri-plugin-notification` + threshold check in event loop |

Toggles persist to `AppData/Local/baseus-desktop/settings.json` via a hand-rolled `settings.rs` module (no extra plugin). Settings load on startup before the window appears.

---

## New Tauri Commands

### `set_anc_mode`
```typescript
invoke('set_anc_mode', { mode: 'off' | 'anc' | 'transparency' })
```
Writes the appropriate command frame to the GATT write characteristic via the existing `GattTransport`. Returns `Ok(())` or an error string. Errors surface as a brief shake animation on the active button.

### `find_earbud`
```typescript
invoke('find_earbud', { side: 'left' | 'right' })
```
Writes the `BA10` find command. Fire-and-forget — no response expected from device.

### `get_settings` / `set_settings`
```typescript
invoke('get_settings') → Settings
invoke('set_settings', { settings: Settings })
```
```typescript
interface Settings {
  launch_at_login: boolean;
  low_battery_alerts: boolean;
}
```

---

## Frontend State

**`BatteryHistory` store** (`src/stores/batteryHistory.ts`):
- Holds up to 60 timestamped readings per component (left, right, case)
- Appended on every `battery_update` / `case_update` event
- Exposed as a SolidJS store: `batteryHistory.left`, `.right`, `.case`
- `SparkLine` reads the array directly

**`Settings` store** (`src/stores/settings.ts`):
- Loaded from backend on mount via `get_settings`
- Writes back via `set_settings` on every toggle change

---

## New Files

| Path | Purpose |
|---|---|
| `src/components/BudCard.tsx` | Ring + sparkline + label for one bud |
| `src/components/CaseCard.tsx` | Ring + sparkline for the case |
| `src/components/AncButton.tsx` | Single ANC mode button with active/loading states |
| `src/components/FindButton.tsx` | Find-earbud button with 3s loading reset |
| `src/components/SettingRow.tsx` | Label + description + toggle row |
| `src/components/SparkLine.tsx` | SVG sparkline from array of numbers |
| `src/stores/batteryHistory.ts` | SolidJS store buffering last 60 readings |
| `src/stores/settings.ts` | SolidJS store backed by Tauri settings command |
| `src/lib/tauri.ts` | Add `setAncMode`, `findEarbud`, `getSettings`, `setSettings` |
| `src-tauri/src/commands.rs` | Implement the four new Tauri commands |
| `src-tauri/src/settings.rs` | Settings load/save to AppData JSON |

**Modified files:**
- `src/App.tsx` — replace current layout with new sections
- `src-tauri/src/device.rs` — thread-safe handle to send commands from Tauri commands
- `src-tauri/src/lib.rs` — register new commands, init autostart plugin
- `src-tauri/Cargo.toml` — add `tauri-plugin-autostart`, `tauri-plugin-notification`
- `apps/baseus-app/package.json` — add `@tauri-apps/plugin-autostart`, `@tauri-apps/plugin-notification`
- `src-tauri/tauri.conf.json` — add plugin permissions

---

## Device Command Architecture

Currently `GattTransport` lives exclusively inside the `device::run_loop` task. To send commands from Tauri commands, we expose a `CommandSender`:

```rust
// device.rs
pub type CommandSender = tokio::sync::mpsc::UnboundedSender<DeviceCommand>;

pub enum DeviceCommand {
    SetAncMode(AncMode),
    FindEarbud(Side),
}
```

`run_loop` receives a `CommandReceiver` and drains it after each notification (non-blocking `try_recv`). The `CommandSender` is stored in Tauri `app.manage()` state so commands can reach it from any command handler.

---

## Low Battery Alerts

In `notification_loop`, after decoding a `BatteryUpdate` event:
- If `left_pct < 20` or `right_pct < 20`, and the previous value was ≥ 20 (edge-trigger only), fire a `tauri-plugin-notification` system notification.
- Same edge-trigger logic for case via `CaseUpdate`.
- Gated by the `low_battery_alerts` setting loaded from state.

---

## Out of Scope

- EQ, gesture remap, firmware update (post-v1)
- Bud charging state (protocol byte not yet identified)
- Multi-device support
