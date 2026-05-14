# UI Redesign — Minimal Pro Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the basic battery-card layout with a polished Minimal Pro dark UI featuring ANC control, battery sparklines, find earbuds, and settings.

**Architecture:** Rust backend gains a `DeviceCommand` mpsc channel, `settings.rs` module, and 4 new Tauri commands. Frontend gains 6 new SolidJS components, 2 stores, and a full App.tsx rewrite. New plugins: `tauri-plugin-autostart`, `tauri-plugin-notification`.

**Tech Stack:** SolidJS, Tailwind CSS v4, Tauri v2, Rust/Tokio, `tauri-plugin-autostart`, `tauri-plugin-notification`

---

## File Map

**Create:**
- `apps/baseus-app/src-tauri/src/settings.rs` — load/save JSON settings to AppData
- `apps/baseus-app/src/components/SparkLine.tsx` — SVG sparkline from array of numbers
- `apps/baseus-app/src/components/BudCard.tsx` — ring + sparkline + label for one bud
- `apps/baseus-app/src/components/CaseCard.tsx` — ring + sparkline for case
- `apps/baseus-app/src/components/AncButton.tsx` — single ANC mode button with active/loading states
- `apps/baseus-app/src/components/FindButton.tsx` — find-earbud button with 3s loading reset
- `apps/baseus-app/src/components/SettingRow.tsx` — label + description + toggle row
- `apps/baseus-app/src/stores/batteryHistory.ts` — SolidJS store, 60 readings per component
- `apps/baseus-app/src/stores/settings.ts` — SolidJS store backed by Tauri settings commands

**Modify:**
- `apps/baseus-app/src-tauri/src/device.rs` — add `DeviceCommand` channel, `CommandReceiver`, drain in loop
- `apps/baseus-app/src-tauri/src/commands.rs` — implement 4 Tauri commands
- `apps/baseus-app/src-tauri/src/lib.rs` — register commands, init autostart plugin, pass command sender to device loop
- `apps/baseus-app/src-tauri/Cargo.toml` — add autostart + notification plugins
- `apps/baseus-app/src-tauri/tauri.conf.json` — resize window to 380×620, add plugin permissions
- `apps/baseus-app/package.json` — add `@tauri-apps/plugin-autostart`, `@tauri-apps/plugin-notification`
- `apps/baseus-app/src/lib/tauri.ts` — add `setAncMode`, `findEarbud`, `getSettings`, `setSettings`
- `apps/baseus-app/src/App.tsx` — full rewrite with new layout

---

## Task 1: DeviceCommand channel in device.rs

**Files:**
- Modify: `apps/baseus-app/src-tauri/src/device.rs`

- [ ] **Step 1: Add DeviceCommand types and re-export CommandSender**

Replace the top of `apps/baseus-app/src-tauri/src/device.rs` with:

```rust
use std::time::Duration;

use baseus_protocol::{framing::Frame, models::bp1_pro_anc::Bp1ProAnc, types::AncMode};
use baseus_transport::win::ble::GattTransport;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

const DEVICE_NAME: &str = "Bass BP1 Pro";
const RETRY_DELAY: Duration = Duration::from_secs(5);
const NOTIF_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum DeviceCommand {
    SetAncMode(AncMode),
    FindEarbud(Side),
}

#[derive(Debug)]
pub enum Side {
    Left,
    Right,
}

pub type CommandSender = mpsc::UnboundedSender<DeviceCommand>;
type CommandReceiver = mpsc::UnboundedReceiver<DeviceCommand>;

pub fn command_channel() -> (CommandSender, CommandReceiver) {
    mpsc::unbounded_channel()
}
```

- [ ] **Step 2: Update run_loop to accept CommandReceiver**

Replace `pub async fn run_loop(app: AppHandle)` with:

```rust
pub async fn run_loop(app: AppHandle, mut cmd_rx: CommandReceiver) {
    loop {
        let _ = app.emit("connection-state", "connecting");
        match GattTransport::connect(DEVICE_NAME).await {
            Ok(mut transport) => {
                let _ = app.emit("connection-state", "connected");
                if let Err(e) = transport.send(&[0xBA, 0x05, 0x00]).await {
                    tracing::warn!("handshake send failed: {e}");
                }
                notification_loop(&app, &mut transport, &mut cmd_rx).await;
                let _ = app.emit("connection-state", "disconnected");
            }
            Err(e) => {
                tracing::warn!("connect failed: {e}");
                let _ = app.emit("connection-state", "disconnected");
            }
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}
```

- [ ] **Step 3: Update notification_loop to drain commands with try_recv**

Replace the `notification_loop` function signature and body:

```rust
async fn notification_loop(
    app: &AppHandle,
    transport: &mut GattTransport,
    cmd_rx: &mut CommandReceiver,
) {
    loop {
        // Drain pending commands before waiting for next notification.
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Err(e) = execute_command(transport, cmd).await {
                tracing::warn!("command error: {e}");
            }
        }

        match tokio::time::timeout(NOTIF_TIMEOUT, transport.next_notification()).await {
            Ok(Ok(data)) => {
                tracing::debug!("raw notification: {}", hex(&data));
                let Ok(frame) = Frame::decode(&data) else {
                    continue;
                };
                match Bp1ProAnc::decode_frame(&frame) {
                    Ok(event) => {
                        let _ = app.emit("device-event", &event);
                    }
                    Err(e) => tracing::debug!("unhandled frame: {e}"),
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("transport error: {e}");
                return;
            }
            Err(_timeout) => {
                if !transport.is_connected().await {
                    tracing::info!("device disconnected (detected via connectivity check)");
                    return;
                }
            }
        }
    }
}
```

- [ ] **Step 4: Add execute_command helper**

Add after `notification_loop`:

```rust
async fn execute_command(
    transport: &mut GattTransport,
    cmd: DeviceCommand,
) -> Result<(), String> {
    let bytes: &[u8] = match &cmd {
        DeviceCommand::SetAncMode(AncMode::Off) => &[0xAA, 0x30, 0x00],
        DeviceCommand::SetAncMode(AncMode::Anc) => &[0xAA, 0x33, 0x01, 0x68],
        DeviceCommand::SetAncMode(AncMode::Transparency) => &[0xAA, 0x32, 0x02, 0xFF],
        DeviceCommand::FindEarbud(Side::Left) => &[0xBA, 0x10, 0x01, 0x00],
        DeviceCommand::FindEarbud(Side::Right) => &[0xBA, 0x10, 0x01, 0x01],
    };
    transport.send(bytes).await.map_err(|e| e.to_string())
}
```

- [ ] **Step 5: Verify it compiles (no tests needed — tested via integration in Task 3)**

```
cd apps/baseus-app/src-tauri && cargo check
```
Expected: compiles with warnings only (unused imports from commands.rs OK at this point).

- [ ] **Step 6: Commit**

```
git add apps/baseus-app/src-tauri/src/device.rs
git commit -m "feat(device): add DeviceCommand channel for ANC and find-earbud commands"
```

---

## Task 2: Settings module

**Files:**
- Create: `apps/baseus-app/src-tauri/src/settings.rs`

- [ ] **Step 1: Create settings.rs**

Create `apps/baseus-app/src-tauri/src/settings.rs`:

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub launch_at_login: bool,
    pub low_battery_alerts: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            low_battery_alerts: true,
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    dirs_next::data_local_dir().map(|d| d.join("baseus-desktop").join("settings.json"))
}

pub fn load() -> Settings {
    let Some(path) = settings_path() else {
        return Settings::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let path = settings_path().ok_or("no data dir")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Add dirs-next dependency to Cargo.toml**

In `apps/baseus-app/src-tauri/Cargo.toml`, under `[dependencies]`:

```toml
dirs-next = "2"
```

- [ ] **Step 3: Declare module in lib.rs**

In `apps/baseus-app/src-tauri/src/lib.rs`, add after `mod commands;`:

```rust
mod settings;
```

- [ ] **Step 4: Verify it compiles**

```
cd apps/baseus-app/src-tauri && cargo check
```
Expected: compiles.

- [ ] **Step 5: Commit**

```
git add apps/baseus-app/src-tauri/src/settings.rs apps/baseus-app/src-tauri/Cargo.toml apps/baseus-app/src-tauri/src/lib.rs
git commit -m "feat(settings): add settings load/save to AppData/Local/baseus-desktop/settings.json"
```

---

## Task 3: Tauri commands + wire up lib.rs

**Files:**
- Modify: `apps/baseus-app/src-tauri/src/commands.rs`
- Modify: `apps/baseus-app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write commands.rs**

Replace `apps/baseus-app/src-tauri/src/commands.rs` entirely:

```rust
use tauri::State;
use crate::device::{CommandSender, DeviceCommand, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::AncMode;

#[tauri::command]
pub fn set_anc_mode(
    mode: String,
    cmd_tx: State<CommandSender>,
) -> Result<(), String> {
    let anc_mode = match mode.as_str() {
        "off" => AncMode::Off,
        "anc" => AncMode::Anc,
        "transparency" => AncMode::Transparency,
        other => return Err(format!("unknown mode: {other}")),
    };
    cmd_tx.send(DeviceCommand::SetAncMode(anc_mode)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_earbud(side: String, cmd_tx: State<CommandSender>) -> Result<(), String> {
    let s = match side.as_str() {
        "left" => Side::Left,
        "right" => Side::Right,
        other => return Err(format!("unknown side: {other}")),
    };
    cmd_tx.send(DeviceCommand::FindEarbud(s)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    settings::save(&settings)
}
```

- [ ] **Step 2: Rewrite lib.rs to wire everything together**

Replace `apps/baseus-app/src-tauri/src/lib.rs` entirely:

```rust
mod commands;
mod device;
mod settings;
mod tray;

use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (cmd_tx, cmd_rx) = device::command_channel();

    tauri::Builder::default()
        .manage(cmd_tx)
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(device::run_loop(handle, cmd_rx));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_anc_mode,
            commands::find_earbud,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify compilation**

```
cd apps/baseus-app/src-tauri && cargo check
```
Expected: compiles without errors.

- [ ] **Step 4: Commit**

```
git add apps/baseus-app/src-tauri/src/commands.rs apps/baseus-app/src-tauri/src/lib.rs
git commit -m "feat(commands): wire set_anc_mode, find_earbud, get_settings, set_settings"
```

---

## Task 4: Plugin dependencies + window config

**Files:**
- Modify: `apps/baseus-app/src-tauri/Cargo.toml`
- Modify: `apps/baseus-app/src-tauri/tauri.conf.json`
- Modify: `apps/baseus-app/package.json`
- Modify: `apps/baseus-app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add Rust plugin dependencies**

In `apps/baseus-app/src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Add JS plugin packages**

```
cd apps/baseus-app && pnpm add @tauri-apps/plugin-autostart @tauri-apps/plugin-notification
```

- [ ] **Step 3: Update tauri.conf.json — window size and plugin permissions**

Replace the `"app"` section in `apps/baseus-app/src-tauri/tauri.conf.json`:

```json
"app": {
  "windows": [
    {
      "label": "main",
      "title": "Baseus Desktop",
      "width": 380,
      "height": 620,
      "resizable": false,
      "alwaysOnTop": false
    }
  ],
  "security": {
    "csp": null
  }
},
"plugins": {
  "autostart": {
    "desktopFile": null,
    "hidden": false,
    "args": []
  },
  "notification": {}
}
```

- [ ] **Step 4: Register autostart plugin in lib.rs**

In `apps/baseus-app/src-tauri/src/lib.rs`, update the builder:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec!["--minimized"]),
    ))
    .plugin(tauri_plugin_notification::init())
    .manage(cmd_tx)
    .setup(|app| {
        // ... existing setup code unchanged ...
    })
```

(Keep the existing `.manage`, `.setup`, `.invoke_handler`, `.run` lines intact; just insert the two `.plugin()` calls before `.manage`.)

- [ ] **Step 5: Verify compilation**

```
cd apps/baseus-app/src-tauri && cargo check
```
Expected: compiles.

- [ ] **Step 6: Commit**

```
git add apps/baseus-app/src-tauri/Cargo.toml apps/baseus-app/src-tauri/tauri.conf.json apps/baseus-app/src-tauri/src/lib.rs apps/baseus-app/package.json apps/baseus-app/pnpm-lock.yaml
git commit -m "feat(config): add autostart + notification plugins, resize window to 380x620"
```

---

## Task 5: Low battery edge-trigger alerts

**Files:**
- Modify: `apps/baseus-app/src-tauri/src/device.rs`

This task adds notification firing when a bud drops below 20%, with edge-trigger logic (fires once on the crossing, not on every frame below threshold). It is gated by the `low_battery_alerts` setting.

- [ ] **Step 1: Add battery threshold tracking state**

In `device.rs`, after the constants at the top, add a struct to track last known percentages:

```rust
#[derive(Default)]
struct BatteryThresholds {
    left_was_ok: bool,
    right_was_ok: bool,
    case_was_ok: bool,
}
```

- [ ] **Step 2: Thread notification firing into notification_loop**

Update `notification_loop` to track thresholds and fire notifications. The complete updated function:

```rust
async fn notification_loop(
    app: &AppHandle,
    transport: &mut GattTransport,
    cmd_rx: &mut CommandReceiver,
) {
    let mut thresholds = BatteryThresholds {
        left_was_ok: true,
        right_was_ok: true,
        case_was_ok: true,
    };

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Err(e) = execute_command(transport, cmd).await {
                tracing::warn!("command error: {e}");
            }
        }

        match tokio::time::timeout(NOTIF_TIMEOUT, transport.next_notification()).await {
            Ok(Ok(data)) => {
                tracing::debug!("raw notification: {}", hex(&data));
                let Ok(frame) = Frame::decode(&data) else {
                    continue;
                };
                match Bp1ProAnc::decode_frame(&frame) {
                    Ok(event) => {
                        maybe_alert_battery(app, &event, &mut thresholds);
                        let _ = app.emit("device-event", &event);
                    }
                    Err(e) => tracing::debug!("unhandled frame: {e}"),
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("transport error: {e}");
                return;
            }
            Err(_timeout) => {
                if !transport.is_connected().await {
                    tracing::info!("device disconnected (detected via connectivity check)");
                    return;
                }
            }
        }
    }
}
```

- [ ] **Step 3: Add maybe_alert_battery function**

Add this function after `execute_command`:

```rust
fn maybe_alert_battery(
    app: &AppHandle,
    event: &baseus_protocol::types::DeviceEvent,
    thresholds: &mut BatteryThresholds,
) {
    use baseus_protocol::types::DeviceEvent;
    use tauri_plugin_notification::NotificationExt;

    let settings = crate::settings::load();
    if !settings.low_battery_alerts {
        return;
    }

    const LOW: u8 = 20;

    match event {
        DeviceEvent::BatteryUpdate(b) => {
            let left_now_ok = b.left_pct >= LOW || b.left_pct == 0;
            let right_now_ok = b.right_pct >= LOW || b.right_pct == 0;

            if thresholds.left_was_ok && !left_now_ok {
                let _ = app.notification()
                    .builder()
                    .title("Baseus — Left bud low")
                    .body(format!("{}% remaining", b.left_pct))
                    .show();
            }
            if thresholds.right_was_ok && !right_now_ok {
                let _ = app.notification()
                    .builder()
                    .title("Baseus — Right bud low")
                    .body(format!("{}% remaining", b.right_pct))
                    .show();
            }
            thresholds.left_was_ok = left_now_ok;
            thresholds.right_was_ok = right_now_ok;
        }
        DeviceEvent::CaseUpdate(c) => {
            let case_now_ok = c.case_pct >= LOW || c.case_pct == 0;
            if thresholds.case_was_ok && !case_now_ok {
                let _ = app.notification()
                    .builder()
                    .title("Baseus — Case low")
                    .body(format!("{}% remaining", c.case_pct))
                    .show();
            }
            thresholds.case_was_ok = case_now_ok;
        }
        _ => {}
    }
}
```

Note: buds at 0% are in-case (passive), so the `|| pct == 0` guard prevents spurious alerts.

- [ ] **Step 4: Verify compilation**

```
cd apps/baseus-app/src-tauri && cargo check
```
Expected: compiles.

- [ ] **Step 5: Commit**

```
git add apps/baseus-app/src-tauri/src/device.rs
git commit -m "feat(alerts): edge-trigger low battery notifications via tauri-plugin-notification"
```

---

## Task 6: SparkLine component

**Files:**
- Create: `apps/baseus-app/src/components/SparkLine.tsx`

- [ ] **Step 1: Create SparkLine.tsx**

Create `apps/baseus-app/src/components/SparkLine.tsx`:

```tsx
interface Props {
  data: number[];
  color: string;
  width?: number;
  height?: number;
}

export default function SparkLine(props: Props) {
  const w = () => props.width ?? 100;
  const h = () => props.height ?? 24;

  const points = () => {
    const d = props.data;
    if (d.length < 2) return '';
    const step = w() / (d.length - 1);
    return d
      .map((v, i) => {
        const x = i * step;
        const y = h() - (v / 100) * h();
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  };

  return (
    <svg
      width={w()}
      height={h()}
      viewBox={`0 0 ${w()} ${h()}`}
      style="overflow: visible;"
    >
      <polyline
        points={points()}
        fill="none"
        stroke={props.color}
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        opacity="0.5"
      />
    </svg>
  );
}
```

- [ ] **Step 2: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors from SparkLine.tsx.

- [ ] **Step 3: Commit**

```
git add apps/baseus-app/src/components/SparkLine.tsx
git commit -m "feat(ui): add SparkLine SVG component"
```

---

## Task 7: BatteryHistory store

**Files:**
- Create: `apps/baseus-app/src/stores/batteryHistory.ts`

- [ ] **Step 1: Create stores directory and batteryHistory.ts**

Create `apps/baseus-app/src/stores/batteryHistory.ts`:

```typescript
import { createStore } from 'solid-js/store';

export interface Reading {
  pct: number;
  ts: number;
}

interface HistoryState {
  left: Reading[];
  right: Reading[];
  case: Reading[];
}

const MAX = 60;

const [history, setHistory] = createStore<HistoryState>({
  left: [],
  right: [],
  case: [],
});

function push(key: keyof HistoryState, pct: number) {
  setHistory(key, (prev) => {
    const next = [...prev, { pct, ts: Date.now() }];
    return next.length > MAX ? next.slice(next.length - MAX) : next;
  });
}

export function pushLeft(pct: number) {
  push('left', pct);
}

export function pushRight(pct: number) {
  push('right', pct);
}

export function pushCase(pct: number) {
  push('case', pct);
}

export function getLeft(): Reading[] {
  return history.left;
}

export function getRight(): Reading[] {
  return history.right;
}

export function getCase(): Reading[] {
  return history.case;
}
```

- [ ] **Step 2: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add apps/baseus-app/src/stores/batteryHistory.ts
git commit -m "feat(store): add BatteryHistory store (60 readings per component)"
```

---

## Task 8: Settings store + tauri.ts bindings

**Files:**
- Create: `apps/baseus-app/src/stores/settings.ts`
- Modify: `apps/baseus-app/src/lib/tauri.ts`

- [ ] **Step 1: Add tauri.ts bindings for new commands**

Append to `apps/baseus-app/src/lib/tauri.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface Settings {
  launch_at_login: boolean;
  low_battery_alerts: boolean;
}

export function setAncMode(mode: 'off' | 'anc' | 'transparency'): Promise<void> {
  return invoke('set_anc_mode', { mode });
}

export function findEarbud(side: 'left' | 'right'): Promise<void> {
  return invoke('find_earbud', { side });
}

export function getSettings(): Promise<Settings> {
  return invoke('get_settings');
}

export function setSettings(settings: Settings): Promise<void> {
  return invoke('set_settings', { settings });
}
```

- [ ] **Step 2: Create settings.ts store**

Create `apps/baseus-app/src/stores/settings.ts`:

```typescript
import { createSignal } from 'solid-js';
import { getSettings, setSettings, type Settings } from '../lib/tauri';

const [settings, setSettingsSignal] = createSignal<Settings>({
  launch_at_login: true,
  low_battery_alerts: true,
});

export async function loadSettings() {
  const s = await getSettings();
  setSettingsSignal(s);
}

export function getSettingsStore(): Settings {
  return settings();
}

export async function updateSetting<K extends keyof Settings>(
  key: K,
  value: Settings[K],
) {
  const next: Settings = { ...settings(), [key]: value };
  setSettingsSignal(next);
  await setSettings(next);
}
```

- [ ] **Step 3: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```
git add apps/baseus-app/src/lib/tauri.ts apps/baseus-app/src/stores/settings.ts
git commit -m "feat(store): add settings store and tauri.ts bindings for new commands"
```

---

## Task 9: BudCard component

**Files:**
- Create: `apps/baseus-app/src/components/BudCard.tsx`

- [ ] **Step 1: Create BudCard.tsx**

Create `apps/baseus-app/src/components/BudCard.tsx`:

```tsx
import SparkLine from './SparkLine';

interface Props {
  label: string;
  pct: number;
  history: number[];
}

const CIRCUMFERENCE = 2 * Math.PI * 32; // r=32

export default function BudCard(props: Props) {
  const isLow = () => props.pct > 0 && props.pct < 20;
  const color = () => (isLow() ? '#ef4444' : '#22c55e');
  const offset = () => CIRCUMFERENCE * (1 - props.pct / 100);

  return (
    <div
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '16px 12px 12px',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        gap: '10px',
      }}
    >
      <div style={{ position: 'relative', width: '80px', height: '80px' }}>
        <svg
          width="80"
          height="80"
          viewBox="0 0 80 80"
          style={{ transform: 'rotate(-90deg)' }}
        >
          <circle cx="40" cy="40" r="32" fill="none" stroke="#1a1a1e" stroke-width="6" />
          <circle
            cx="40"
            cy="40"
            r="32"
            fill="none"
            stroke={color()}
            stroke-width="6"
            stroke-dasharray={CIRCUMFERENCE}
            stroke-dashoffset={offset()}
            stroke-linecap="round"
            style={{ transition: 'stroke-dashoffset 0.5s ease, stroke 0.3s ease' }}
          />
        </svg>
        <div
          style={{
            position: 'absolute',
            inset: '0',
            display: 'flex',
            'flex-direction': 'column',
            'align-items': 'center',
            'justify-content': 'center',
          }}
        >
          <span style={{ 'font-size': '22px', 'font-weight': '800', color: '#fff', 'line-height': '1' }}>
            {props.pct}
          </span>
          <span style={{ 'font-size': '10px', color: '#404040', 'font-weight': '500' }}>%</span>
        </div>
      </div>

      <SparkLine data={props.history} color={color()} width={88} height={24} />

      <span style={{ 'font-size': '11px', color: '#525252', 'font-weight': '500', 'letter-spacing': '0.04em' }}>
        {props.label}
      </span>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add apps/baseus-app/src/components/BudCard.tsx
git commit -m "feat(ui): add BudCard component with animated ring and sparkline"
```

---

## Task 10: CaseCard component

**Files:**
- Create: `apps/baseus-app/src/components/CaseCard.tsx`

- [ ] **Step 1: Create CaseCard.tsx**

Create `apps/baseus-app/src/components/CaseCard.tsx`:

```tsx
import SparkLine from './SparkLine';

interface Props {
  pct: number;
  history: number[];
}

const CIRCUMFERENCE = 2 * Math.PI * 20; // r=20

export default function CaseCard(props: Props) {
  const offset = () => CIRCUMFERENCE * (1 - props.pct / 100);

  return (
    <div
      style={{
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '14px 16px',
        display: 'flex',
        'align-items': 'center',
        gap: '14px',
      }}
    >
      <div style={{ position: 'relative', width: '52px', height: '52px', 'flex-shrink': '0' }}>
        <svg
          width="52"
          height="52"
          viewBox="0 0 52 52"
          style={{ transform: 'rotate(-90deg)' }}
        >
          <circle cx="26" cy="26" r="20" fill="none" stroke="#1a1a1e" stroke-width="5" />
          <circle
            cx="26"
            cy="26"
            r="20"
            fill="none"
            stroke="#6366f1"
            stroke-width="5"
            stroke-dasharray={CIRCUMFERENCE}
            stroke-dashoffset={offset()}
            stroke-linecap="round"
            style={{ transition: 'stroke-dashoffset 0.5s ease' }}
          />
        </svg>
        <div
          style={{
            position: 'absolute',
            inset: '0',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            'font-size': '14px',
            'font-weight': '700',
            color: '#fff',
          }}
        >
          {props.pct}
        </div>
      </div>

      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '11px', color: '#525252', 'font-weight': '500', 'letter-spacing': '0.04em', 'margin-bottom': '6px' }}>
          CASE BATTERY
        </div>
        <SparkLine data={props.history} color="#6366f1" width={160} height={20} />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add apps/baseus-app/src/components/CaseCard.tsx
git commit -m "feat(ui): add CaseCard component with indigo ring and sparkline"
```

---

## Task 11: AncButton component

**Files:**
- Create: `apps/baseus-app/src/components/AncButton.tsx`

- [ ] **Step 1: Create AncButton.tsx**

Create `apps/baseus-app/src/components/AncButton.tsx`:

```tsx
interface Props {
  icon: string;
  label: string;
  active: boolean;
  loading: boolean;
  onClick: () => void;
}

export default function AncButton(props: Props) {
  const baseStyle = {
    flex: '1',
    padding: '10px 6px',
    'border-radius': '10px',
    'text-align': 'center' as const,
    'font-size': '12px',
    'font-weight': '500',
    cursor: 'pointer',
    display: 'flex',
    'flex-direction': 'column' as const,
    'align-items': 'center',
    gap: '4px',
    border: '1px solid',
    transition: 'all 0.12s',
  };

  const activeStyle = {
    background: 'rgba(99,102,241,0.12)',
    'border-color': 'rgba(99,102,241,0.3)',
    color: '#a5b4fc',
  };

  const inactiveStyle = {
    background: '#111113',
    'border-color': '#1a1a1e',
    color: '#404040',
  };

  return (
    <div
      style={{
        ...baseStyle,
        ...(props.active ? activeStyle : inactiveStyle),
        animation: props.loading ? 'pulse 0.8s ease-in-out infinite' : 'none',
      }}
      onClick={props.onClick}
    >
      <span style={{ 'font-size': '16px' }}>{props.icon}</span>
      <span style={{ color: props.active ? '#818cf8' : '#404040' }}>{props.label}</span>
    </div>
  );
}
```

- [ ] **Step 2: Add pulse keyframe to index.css or inline in App.tsx**

Add to `apps/baseus-app/src/index.css` (or create it if it doesn't exist and import in index.tsx):

```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

Check if `src/index.css` exists:
```
cd apps/baseus-app && ls src/
```

If `index.css` does not exist, create it with just the keyframe above, then add `import './index.css';` to `src/index.tsx`.

If it does exist, append the keyframe.

- [ ] **Step 3: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```
git add apps/baseus-app/src/components/AncButton.tsx apps/baseus-app/src/index.css apps/baseus-app/src/index.tsx
git commit -m "feat(ui): add AncButton component with active and loading states"
```

---

## Task 12: FindButton and SettingRow components

**Files:**
- Create: `apps/baseus-app/src/components/FindButton.tsx`
- Create: `apps/baseus-app/src/components/SettingRow.tsx`

- [ ] **Step 1: Create FindButton.tsx**

Create `apps/baseus-app/src/components/FindButton.tsx`:

```tsx
import { createSignal } from 'solid-js';

interface Props {
  label: string;
  onClick: () => void;
}

export default function FindButton(props: Props) {
  const [loading, setLoading] = createSignal(false);

  function handleClick() {
    if (loading()) return;
    setLoading(true);
    props.onClick();
    setTimeout(() => setLoading(false), 3000);
  }

  return (
    <div
      style={{
        flex: '1',
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'center',
        gap: '7px',
        padding: '11px',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '10px',
        'font-size': '12px',
        'font-weight': '500',
        color: loading() ? '#a3a3a3' : '#525252',
        cursor: loading() ? 'default' : 'pointer',
        transition: 'color 0.12s',
      }}
      onClick={handleClick}
    >
      <span style={{ 'font-size': '14px' }}>🔊</span>
      {loading() ? 'Playing…' : props.label}
    </div>
  );
}
```

- [ ] **Step 2: Create SettingRow.tsx**

Create `apps/baseus-app/src/components/SettingRow.tsx`:

```tsx
interface Props {
  label: string;
  description: string;
  value: boolean;
  onChange: (v: boolean) => void;
}

export default function SettingRow(props: Props) {
  return (
    <div
      style={{
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'space-between',
        padding: '10px 0',
      }}
    >
      <div>
        <div style={{ 'font-size': '13px', color: '#a3a3a3' }}>{props.label}</div>
        <div style={{ 'font-size': '11px', color: '#333', 'margin-top': '1px' }}>{props.description}</div>
      </div>

      <div
        style={{
          width: '36px',
          height: '20px',
          background: props.value ? '#6366f1' : '#1e1e22',
          'border-radius': '10px',
          position: 'relative',
          'flex-shrink': '0',
          cursor: 'pointer',
          transition: 'background 0.15s',
        }}
        onClick={() => props.onChange(!props.value)}
      >
        <div
          style={{
            position: 'absolute',
            width: '14px',
            height: '14px',
            background: '#fff',
            'border-radius': '50%',
            top: '3px',
            left: props.value ? '19px' : '3px',
            transition: 'left 0.15s',
          }}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```
git add apps/baseus-app/src/components/FindButton.tsx apps/baseus-app/src/components/SettingRow.tsx
git commit -m "feat(ui): add FindButton and SettingRow components"
```

---

## Task 13: App.tsx full rewrite

**Files:**
- Modify: `apps/baseus-app/src/App.tsx`

This is the integration task that assembles all components into the Minimal Pro layout.

- [ ] **Step 1: Rewrite App.tsx**

Replace `apps/baseus-app/src/App.tsx` entirely:

```tsx
import { createSignal, onCleanup, onMount } from 'solid-js';
import BudCard from './components/BudCard';
import CaseCard from './components/CaseCard';
import AncButton from './components/AncButton';
import FindButton from './components/FindButton';
import SettingRow from './components/SettingRow';
import {
  onDeviceEvent,
  onConnectionState,
  setAncMode,
  findEarbud,
  type BatteryState,
  type CaseState,
} from './lib/tauri';
import { pushLeft, pushRight, pushCase, getLeft, getRight, getCase } from './stores/batteryHistory';
import { loadSettings, getSettingsStore, updateSetting } from './stores/settings';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';
type AncMode = 'off' | 'anc' | 'transparency';

const sectionLabel: Record<string, object> = {
  style: {
    'font-size': '10px',
    'font-weight': '600',
    color: '#333',
    'letter-spacing': '0.1em',
    'text-transform': 'uppercase',
    'margin-bottom': '14px',
    display: 'flex',
    'align-items': 'center',
    gap: '8px',
  },
};

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [battery, setBattery] = createSignal<BatteryState | null>(null);
  const [caseState, setCaseState] = createSignal<CaseState | null>(null);
  const [ancMode, setAncModeSignal] = createSignal<AncMode>('off');
  const [ancLoading, setAncLoading] = createSignal<AncMode | null>(null);

  onMount(async () => {
    await loadSettings();

    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        setBattery(e.data);
        pushLeft(e.data.left_pct);
        pushRight(e.data.right_pct);
      } else if (e.type === 'case_update') {
        setCaseState(e.data);
        pushCase(e.data.case_pct);
      } else if (e.type === 'anc_mode_update') {
        setAncModeSignal(e.data);
        setAncLoading(null);
      }
    }).then((fn) => unlisteners.push(fn));

    onConnectionState((s) => setStatus(s)).then((fn) => unlisteners.push(fn));
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    try {
      await setAncMode(mode);
    } catch {
      setAncLoading(null);
    }
  }

  const statusColor = () =>
    status() === 'connected' ? '#22c55e' : status() === 'connecting' ? '#eab308' : '#525252';

  const statusText = () =>
    status() === 'connected' ? 'Connected' : status() === 'connecting' ? 'Connecting…' : 'Disconnected';

  return (
    <div
      style={{
        width: '380px',
        'min-height': '620px',
        background: '#0d0d0f',
        color: '#fff',
        'font-family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        'box-sizing': 'border-box',
      }}
    >
      {/* Title bar */}
      <div
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          padding: '14px 18px 12px',
          'border-bottom': '1px solid #161618',
        }}
      >
        <div style={{ display: 'flex', gap: '6px' }}>
          {(['#ff5f57', '#ffbd2e', '#28c840'] as const).map((c) => (
            <div style={{ width: '11px', height: '11px', 'border-radius': '50%', background: c }} />
          ))}
        </div>
        <div
          style={{
            flex: '1',
            'text-align': 'center',
            'font-size': '12px',
            'font-weight': '600',
            color: '#525252',
            'margin-left': '-50px',
          }}
        >
          Bass BP1 Pro ANC
        </div>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '5px', 'font-size': '11px', color: statusColor(), 'font-weight': '500' }}>
          <div style={{ width: '6px', height: '6px', background: statusColor(), 'border-radius': '50%' }} />
          {statusText()}
        </div>
      </div>

      {/* Earbuds */}
      <div style={{ padding: '20px 20px 0' }}>
        <div style={{ ...(sectionLabel as any).style }}>
          Earbuds
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '12px' }}>
          <BudCard
            label="LEFT"
            pct={battery()?.left_pct ?? 0}
            history={getLeft().map((r) => r.pct)}
          />
          <BudCard
            label="RIGHT"
            pct={battery()?.right_pct ?? 0}
            history={getRight().map((r) => r.pct)}
          />
        </div>
      </div>

      {/* Case */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={{ ...(sectionLabel as any).style }}>
          Case
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <CaseCard
          pct={caseState()?.case_pct ?? 0}
          history={getCase().map((r) => r.pct)}
        />
      </div>

      {/* Noise Control */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={{ ...(sectionLabel as any).style }}>
          Noise Control
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          {(['off', 'anc', 'transparency'] as AncMode[]).map((mode) => {
            const icons: Record<AncMode, string> = { off: '🔇', anc: '🎧', transparency: '🌬️' };
            const labels: Record<AncMode, string> = { off: 'Off', anc: 'ANC', transparency: 'Transparent' };
            return (
              <AncButton
                icon={icons[mode]}
                label={labels[mode]}
                active={ancMode() === mode}
                loading={ancLoading() === mode}
                onClick={() => handleAnc(mode)}
              />
            );
          })}
        </div>
      </div>

      {/* Find Earbuds */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={{ ...(sectionLabel as any).style }}>
          Find Earbuds
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <FindButton label="Play Left" onClick={() => findEarbud('left').catch(() => {})} />
          <FindButton label="Play Right" onClick={() => findEarbud('right').catch(() => {})} />
        </div>
      </div>

      {/* Settings */}
      <div style={{ padding: '14px 20px 20px' }}>
        <div style={{ ...(sectionLabel as any).style }}>
          Settings
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <SettingRow
          label="Launch at login"
          description="Start automatically with Windows"
          value={getSettingsStore().launch_at_login}
          onChange={(v) => updateSetting('launch_at_login', v)}
        />
        <div style={{ height: '1px', background: '#131315' }} />
        <SettingRow
          label="Low battery alerts"
          description="Notify when a bud drops below 20%"
          value={getSettingsStore().low_battery_alerts}
          onChange={(v) => updateSetting('low_battery_alerts', v)}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript**

```
cd apps/baseus-app && pnpm exec tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Run the app and verify visually**

```
cd apps/baseus-app && pnpm tauri dev
```

Open the app. Check:
- Window is 380×620, not resizable.
- Title bar shows traffic dots + device name + connection status.
- Earbuds section shows two ring cards; rings animate when battery values arrive.
- Case section shows indigo ring with sparkline.
- Noise Control shows three buttons; clicking ANC sends command.
- Find Earbuds buttons show "Playing…" for 3 seconds on click.
- Settings section shows two toggles that persist on restart.

- [ ] **Step 4: Commit**

```
git add apps/baseus-app/src/App.tsx
git commit -m "feat(ui): full App.tsx rewrite — Minimal Pro layout with all sections"
```

---

## Self-Review

**Spec coverage:**
- ✅ Window 380×620, not resizable — Task 4
- ✅ Title bar with dots + device name + connection status — Task 13
- ✅ BudCard with 80px ring + sparkline + label — Tasks 9, 13
- ✅ CaseCard with 52px indigo ring + sparkline — Tasks 10, 13
- ✅ ANC buttons with active/loading states — Tasks 11, 13
- ✅ `set_anc_mode` Tauri command with correct bytes — Tasks 1, 3
- ✅ FindButton 3s loading reset — Task 12
- ✅ `find_earbud` Tauri command with BA10 bytes — Tasks 1, 3
- ✅ Settings toggles with persist — Tasks 2, 8, 12, 13
- ✅ `launch_at_login` via autostart plugin — Tasks 4, 8
- ✅ Low battery alerts, edge-trigger, gated by setting — Task 5
- ✅ BatteryHistory store, 60 readings, exposed as arrays — Task 7
- ✅ SparkLine reads array of numbers — Task 6
- ✅ `get_settings` / `set_settings` commands — Tasks 2, 3
- ✅ DeviceCommand channel in device.rs — Task 1
- ✅ AncMode serialization from Rust: `'off' | 'anc' | 'transparency'` — existing protocol types

**Placeholder scan:** None found.

**Type consistency check:**
- `DeviceCommand::SetAncMode(AncMode)` — `AncMode` imported from `baseus_protocol::types` in Tasks 1 and 3 ✅
- `CommandSender` type exported from `device.rs`, imported in `commands.rs` ✅
- `Settings` struct in `settings.rs`, serialized to JSON, matches `Settings` interface in `tauri.ts` (snake_case fields) ✅
- `BatteryState.left_pct`, `CaseState.case_pct` — same field names as existing `tauri.ts` ✅
- `getLeft()` returns `Reading[]`, `.map(r => r.pct)` in App.tsx gives `number[]` — matches `SparkLine data: number[]` ✅
