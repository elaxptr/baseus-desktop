# Dashboard Redesign & Feature Expansion — Design Spec

## Goal

Rebuild the Baseus app UI from a single-page widget into a sidebar-navigation dashboard (~480px wide) while adding six new features: EQ presets, touch gesture config, wear detection, session timer, charging state, and ANC level slider. Four of those features require additional BLE protocol reverse-engineering; two are ready to build now.

---

## Architecture Overview

The app stays a single Tauri window (no new windows, no routing library). Navigation is handled by a `activeTab` SolidJS signal in `App.tsx`. The sidebar renders icon buttons; clicking one sets the signal; the matching section renders. No dynamic imports — all five tab components are always mounted, just hidden with `display:none` when inactive. This keeps state (e.g. the session timer, ANC slider position) alive across tab switches.

**Window size change:** `tauri.conf.json` width bumped from 380 → 480px. Height unchanged (620px min).

---

## New Features

### 0. Find Earbuds (already works — just moved)
The existing Find Left / Find Right buttons move into `HomeTab`, below the case bar. No protocol change. The 5-second auto-stop in `device.rs` is unchanged.

### 1. Session Timer (no RE needed)
Track elapsed listening time for the current connection session. Start the timer when `connection-state` becomes `connected`; stop and reset when it becomes `disconnected`. Display as `Xh Ym` on the Home tab. A "show session timer" toggle in Settings controls visibility. Persisted across app restarts with `tauri-plugin-store` (total today's time accumulates until midnight reset).

### 2. Charging State (needs live capture to confirm)
The current battery frame (`AA 02`) has bytes at positions [1] and [3] that we documented as "bud-ID markers" (0x00=left, 0x01=right) based on a single capture. These need a second capture while the buds are actually charging to confirm whether they become non-zero when charging. If confirmed, `BatteryState.left_charging` and `right_charging` in `baseus-protocol` become meaningful and a ⚡ badge renders on the bud ring. Case charging (`AA 27` byte [1]) is already decoded as `case_charging`. Display: small lightning bolt overlay on the battery ring when charging is true.

### 3. ANC Level Slider (likely works now)
The `BA 34 [type] [level]` write already has a `level` byte. Currently hardcoded to `0x68` (ANC) and `0xFF` (Transparency/Off). A slider (1–10 mapped to the byte range) lets the user control strength. The UI sends `SetAncMode` with a new `level: u8` field. If the device ignores the level byte, the slider is hidden until confirmed working.

**Protocol change needed in `types.rs`:**
```rust
pub enum AncMode { Off, Anc { level: u8 }, Transparency { level: u8 } }
```
`execute_command` uses `level` directly as the 4th byte. Default level = `0x68` (7/10 strength). The UI slider maps 1–10 → `0x10`–`0xFF` linearly.

### 4. EQ Presets (needs RE)
Four presets: Balanced, Bass Boost, Voice, Clear. The Baseus Android app's `EQDataModel` / `EQDataResolvePresenter` classes contain the write commands. RE approach: Frida-hook the BLE write characteristic in MuMuPlayer while switching EQ in the app, capturing the `BA` bytes for each preset. Until RE is done, the EQ tab renders with a "Protocol research needed" notice and the buttons are disabled.

### 5. Touch Gesture Config (needs RE)
Six actions per bud (double-tap, triple-tap, long-press × left + right). Actions: Play/Pause, Next Track, Prev Track, ANC Toggle, Voice Assistant, Volume Up/Down. RE approach: same Frida hook as EQ — trigger each gesture remap in the Android app and capture the write bytes. Until done, gesture dropdowns render but are disabled with the same notice.

### 6. Wear Detection (needs RE)
The device likely sends a notification when a bud is removed from or inserted into the ear — similar to how AirPods emit an `AA` opcode on ear detection. RE approach: capture `AA` notifications while inserting/removing buds with the Frida hook running. Once the opcode is found, add it to `bp1_pro_anc.rs` and emit `DeviceEvent::WearUpdate { left_in_ear: bool, right_in_ear: bool }`. UI: small dot on each bud ring (green = in ear, dim = not in ear).

---

## Component & File Map

### Modified files

| File | Change |
|---|---|
| `tauri.conf.json` | width 380 → 480 |
| `src/App.tsx` | Add `activeTab` signal, sidebar layout, handle `WearUpdate` and `connection-state` for timer |
| `src/stores/batteryHistory.ts` | No change |
| `src/stores/settings.ts` | Add `show_session_timer: bool` setting |
| `src-tauri/src/device.rs` | Handle `WearUpdate` event when opcode is found |
| `crates/baseus-protocol/src/types.rs` | Add `DeviceEvent::WearUpdate { left_in_ear: bool, right_in_ear: bool }`, extend `AncMode` with `level`, add `EqPreset { Balanced, BassBoost, Voice, Clear }` enum |
| `crates/baseus-protocol/src/models/bp1_pro_anc.rs` | Add wear detection opcode when captured |

### New files

| File | Responsibility |
|---|---|
| `src/components/Sidebar.tsx` | Icon nav strip, `activeTab` prop + `onSwitch` callback |
| `src/components/HomeTab.tsx` | Battery rings, case bar, wear dots, session timer, find earbud buttons |
| `src/components/AncTab.tsx` | Mode cards + level slider |
| `src/components/EqTab.tsx` | Preset grid, disabled state with RE notice |
| `src/components/GesturesTab.tsx` | Per-bud action dropdowns, disabled state with RE notice |
| `src/components/SettingsTab.tsx` | Existing settings + `show_session_timer` toggle |
| `src/lib/timer.ts` | Session timer logic (start/stop/accumulate) |

### Removed / consolidated

`BudCard.tsx`, `CaseCard.tsx`, `AncButton.tsx`, `FindButton.tsx`, `SettingRow.tsx`, `ConnectionCard.tsx` — all absorbed into their respective tab components. `SparkLine.tsx` is kept as a shared utility.

---

## Data Flow

```
BLE device → GattTransport → notification_loop → app.emit("device-event", event)
                                                ↓
                                     SolidJS onDeviceEvent()
                                                ↓
                          batteryHistory store / ancMode signal / wearState signal
                                                ↓
                              HomeTab / AncTab render reactively
```

Commands flow the other way: tab component calls `invoke("set_anc_mode", { mode, level })` → `commands.rs` → `cmd_tx.send(DeviceCommand::SetAncMode(...))` → `device.rs` execute_command.

The session timer is purely client-side: `connection-state connected` → start `setInterval(1s)` → update signal → `HomeTab` renders elapsed time.

---

## Phased Build Plan

**Phase 1 — Dashboard shell + Home + ANC (buildable now)**
- Sidebar navigation
- HomeTab with battery rings, case bar, wear dots (placeholder — grey until RE)
- AncTab with mode cards + level slider
- Session timer
- Window resize 380 → 480

**Phase 2 — EQ + Gestures stubs**
- EQ tab with disabled preset cards and RE notice
- Gestures tab with disabled dropdowns and RE notice

**Phase 3 — Protocol RE (parallel work)**
- Capture EQ write bytes via Frida
- Capture gesture config bytes via Frida
- Capture wear detection opcode via Frida
- Confirm charging state bytes with a charging-buds capture

**Phase 4 — Wire up Phase 3 findings**
- Enable EQ presets
- Enable gesture dropdowns
- Enable wear dots
- Enable charging badges

---

## Testing

- `cargo test -p baseus-protocol` — add `WearUpdate` decode test once opcode is known; add `AncMode` level encoding test
- Manual: open each tab, verify layout matches spec; connect buds, verify battery / ANC / wear dots update live
- TypeScript: `pnpm typecheck` passes with new component props
- Window: measure rendered width at 480px, height unchanged

---

## Out of Scope

- Per-band custom EQ (only presets)
- Volume control (no BLE command found)
- Firmware update
- Multi-device support
- macOS / Linux
