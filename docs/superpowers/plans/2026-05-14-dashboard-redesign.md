# Dashboard Redesign & Feature Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the Baseus app UI from a single-page widget into a sidebar-navigation dashboard at 480px wide, adding a session timer, ANC level slider, wear-detection placeholders, and stub tabs for EQ and Gestures (to be wired up after protocol RE).

**Architecture:** Five tab components (`HomeTab`, `AncTab`, `EqTab`, `GesturesTab`, `SettingsTab`) are always mounted behind a `Sidebar` icon strip; the active tab is controlled by an `activeTab` SolidJS signal in `App.tsx`. All tabs stay mounted (CSS `display:none`) so state like the session timer survives tab switches.

**Tech Stack:** Rust (Tauri v2, baseus-protocol crate), SolidJS + TypeScript, Tailwind-free inline styles (existing pattern).

---

## File Map

**Create:**
- `apps/baseus-app/src/components/Sidebar.tsx`
- `apps/baseus-app/src/components/HomeTab.tsx`
- `apps/baseus-app/src/components/AncTab.tsx`
- `apps/baseus-app/src/components/EqTab.tsx`
- `apps/baseus-app/src/components/GesturesTab.tsx`
- `apps/baseus-app/src/components/SettingsTab.tsx`
- `apps/baseus-app/src/lib/timer.ts`

**Modify:**
- `apps/baseus-app/src-tauri/tauri.conf.json` — width 380→480
- `apps/baseus-app/src-tauri/src/settings.rs` — add `show_session_timer`
- `apps/baseus-app/src-tauri/src/device.rs` — `DeviceCommand::SetAncMode(AncMode, u8)`, `last_anc_mode: Option<(AncMode, u8)>`
- `apps/baseus-app/src-tauri/src/commands.rs` — `set_anc_mode` accepts `level: Option<u8>`
- `crates/baseus-protocol/src/types.rs` — add `DeviceEvent::WearUpdate`
- `apps/baseus-app/src/lib/tauri.ts` — new types, updated `setAncMode` signature
- `apps/baseus-app/src/stores/settings.ts` — add `show_session_timer` default
- `apps/baseus-app/src/App.tsx` — full rewrite using new layout

**Delete (after App.tsx rewrite):**
- `apps/baseus-app/src/components/BudCard.tsx`
- `apps/baseus-app/src/components/CaseCard.tsx`
- `apps/baseus-app/src/components/AncButton.tsx`
- `apps/baseus-app/src/components/FindButton.tsx`
- `apps/baseus-app/src/components/SettingRow.tsx`
- `apps/baseus-app/src/components/ConnectionCard.tsx`
- `apps/baseus-app/src/components/BatteryCard.tsx`

---

## Task 1: Extend Rust backend types and commands

**Files:**
- Modify: `crates/baseus-protocol/src/types.rs`
- Modify: `apps/baseus-app/src-tauri/src/settings.rs`
- Modify: `apps/baseus-app/src-tauri/src/device.rs`
- Modify: `apps/baseus-app/src-tauri/src/commands.rs`

- [ ] **Step 1: Add `WearUpdate` to `DeviceEvent` in `types.rs`**

Replace the entire file `crates/baseus-protocol/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};

pub mod ble_uuids {
    pub const SERVICE: &str = "53527aa4-29f7-ae11-4e74-997334782568";
    pub const WRITE:   &str = "ee684b1a-1e9b-ed3e-ee55-f894667e92ac";
    pub const NOTIFY:  &str = "654b749c-e37f-ae1f-ebab-40ca133e3690";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub left_pct: u8,
    pub right_pct: u8,
    pub left_charging: bool,
    pub right_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AncMode {
    Off,
    Anc,
    Transparency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WearState {
    pub left_in_ear: bool,
    pub right_in_ear: bool,
}

/// Events emitted from the device to the app (via Tauri `device-event`).
/// Serialised as `{ "type": "<variant>", "data": <payload> }` for TypeScript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum DeviceEvent {
    BatteryUpdate(BatteryState),
    CaseUpdate(CaseState),
    AncModeUpdate(AncMode),
    WearUpdate(WearState),
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseState {
    pub case_pct: u8,
    pub case_charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseusModel {
    Bp1ProAnc,
}
```

- [ ] **Step 2: Run protocol tests to confirm nothing is broken**

```powershell
cd apps/baseus-app; cargo test -p baseus-protocol
```

Expected: all existing tests pass (WearUpdate is additive, nothing removed).

- [ ] **Step 3: Add `show_session_timer` to `settings.rs`**

Replace entire `apps/baseus-app/src-tauri/src/settings.rs`:

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub launch_at_login: bool,
    pub low_battery_alerts: bool,
    pub show_session_timer: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            low_battery_alerts: true,
            show_session_timer: true,
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

- [ ] **Step 4: Add `level` to `DeviceCommand::SetAncMode` in `device.rs`**

Replace entire `apps/baseus-app/src-tauri/src/device.rs`:

```rust
use std::time::Duration;

use baseus_protocol::{framing::Frame, models::bp1_pro_anc::Bp1ProAnc, types::{AncMode, DeviceEvent}};
use baseus_transport::win::ble::GattTransport;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

const DEVICE_NAME: &str = "Bass BP1 Pro";
const RETRY_DELAY: Duration = Duration::from_secs(5);
const NOTIF_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
struct BatteryThresholds {
    left_was_ok: bool,
    right_was_ok: bool,
    case_was_ok: bool,
}

#[derive(Debug)]
pub enum DeviceCommand {
    SetAncMode(AncMode, u8),
    FindEarbud(Side),
}

#[derive(Debug, Clone)]
pub enum Side {
    Left,
    Right,
}

pub type CommandSender = mpsc::UnboundedSender<DeviceCommand>;
type CommandReceiver = mpsc::UnboundedReceiver<DeviceCommand>;

pub fn command_channel() -> (CommandSender, CommandReceiver) {
    mpsc::unbounded_channel()
}

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
    let mut last_anc_mode: Option<(AncMode, u8)> = None;
    let (find_stop_tx, mut find_stop_rx) = tokio::sync::mpsc::unbounded_channel::<Side>();

    loop {
        tokio::select! {
            result = tokio::time::timeout(NOTIF_TIMEOUT, transport.next_notification()) => {
                match result {
                    Ok(Ok(data)) => {
                        tracing::debug!("raw notification: {}", hex(&data));
                        if let Ok(frame) = Frame::decode(&data) {
                            if frame.cmd == 0x34 {
                                let ev = if frame.payload.first().copied().unwrap_or(0) == 0 {
                                    last_anc_mode = None;
                                    DeviceEvent::AncModeUpdate(AncMode::Off)
                                } else {
                                    DeviceEvent::AncModeUpdate(
                                        last_anc_mode.as_ref().map(|(m, _)| m.clone()).unwrap_or(AncMode::Anc)
                                    )
                                };
                                let _ = app.emit("device-event", &ev);
                            } else {
                                match Bp1ProAnc::decode_frame(&frame) {
                                    Ok(event) => {
                                        maybe_alert_battery(app, &event, &mut thresholds);
                                        let _ = app.emit("device-event", &event);
                                    }
                                    Err(e) => tracing::debug!("unhandled frame cmd={:#04x}: {e}", frame.cmd),
                                }
                            }
                        } else {
                            tracing::debug!("rejected notification (unknown magic {:#04x})", data.first().copied().unwrap_or(0));
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
            Some(cmd) = cmd_rx.recv() => {
                tracing::debug!("executing command: {cmd:?}");
                match execute_command(transport, &cmd).await {
                    Ok(()) => {
                        tracing::debug!("command sent ok");
                        match &cmd {
                            DeviceCommand::SetAncMode(mode, level) => {
                                last_anc_mode = Some((mode.clone(), *level));
                                let _ = app.emit("device-event", &DeviceEvent::AncModeUpdate(mode.clone()));
                            }
                            DeviceCommand::FindEarbud(side) => {
                                let tx = find_stop_tx.clone();
                                let side = side.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                    let _ = tx.send(side);
                                });
                            }
                        }
                    }
                    Err(e) => tracing::warn!("command error: {e}"),
                }
            }
            Some(side) = find_stop_rx.recv() => {
                let stop_bytes: &[u8] = match side {
                    Side::Left  => &[0xBA, 0x10, 0x00, 0x00],
                    Side::Right => &[0xBA, 0x10, 0x01, 0x00],
                };
                if let Err(e) = transport.send(stop_bytes).await {
                    tracing::warn!("find auto-stop failed: {e}");
                } else {
                    tracing::debug!("find auto-stop sent");
                }
            }
        }
    }
}

async fn execute_command(
    transport: &mut GattTransport,
    cmd: &DeviceCommand,
) -> Result<(), String> {
    let bytes: Vec<u8> = match cmd {
        DeviceCommand::SetAncMode(AncMode::Off, _) => vec![0xBA, 0x34, 0x00, 0xFF],
        DeviceCommand::SetAncMode(AncMode::Anc, level) => vec![0xBA, 0x34, 0x01, *level],
        DeviceCommand::SetAncMode(AncMode::Transparency, level) => vec![0xBA, 0x34, 0x02, *level],
        DeviceCommand::FindEarbud(Side::Left) => vec![0xBA, 0x10, 0x00, 0x01],
        DeviceCommand::FindEarbud(Side::Right) => vec![0xBA, 0x10, 0x01, 0x01],
    };
    transport.send(&bytes).await.map_err(|e| e.to_string())
}

fn maybe_alert_battery(
    app: &AppHandle,
    event: &DeviceEvent,
    thresholds: &mut BatteryThresholds,
) {
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}
```

- [ ] **Step 5: Update `commands.rs` to accept `level`**

Replace entire `apps/baseus-app/src-tauri/src/commands.rs`:

```rust
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;
use crate::device::{CommandSender, DeviceCommand, Side};
use crate::settings::{self, Settings};
use baseus_protocol::types::AncMode;

#[tauri::command]
pub fn set_anc_mode(
    mode: String,
    level: Option<u8>,
    cmd_tx: State<CommandSender>,
) -> Result<(), String> {
    let anc_mode = match mode.as_str() {
        "off" => AncMode::Off,
        "anc" => AncMode::Anc,
        "transparency" => AncMode::Transparency,
        other => return Err(format!("unknown mode: {other}")),
    };
    let byte = level.unwrap_or(0x68);
    cmd_tx.send(DeviceCommand::SetAncMode(anc_mode, byte)).map_err(|e| e.to_string())
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
pub fn set_settings<R: Runtime>(app: AppHandle<R>, settings: Settings) -> Result<(), String> {
    settings::save(&settings)?;
    if settings.launch_at_login {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 6: Build and confirm Rust compiles**

```powershell
cd apps/baseus-app; cargo build 2>&1 | Select-String -Pattern "error"
```

Expected: no `error` lines. Warnings are OK.

- [ ] **Step 7: Commit**

```powershell
git add crates/baseus-protocol/src/types.rs apps/baseus-app/src-tauri/src/
git commit -m "feat(backend): add WearUpdate event, anc level byte, show_session_timer setting"
```

---

## Task 2: Widen window and update TypeScript types

**Files:**
- Modify: `apps/baseus-app/src-tauri/tauri.conf.json`
- Modify: `apps/baseus-app/src/lib/tauri.ts`
- Modify: `apps/baseus-app/src/stores/settings.ts`

- [ ] **Step 1: Set window width to 480px in `tauri.conf.json`**

Change `"width": 380` to `"width": 480` in the `windows` array.

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "baseus-app",
  "version": "0.1.0",
  "identifier": "com.baseus.desktop",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Baseus Desktop",
        "width": 480,
        "height": 620,
        "resizable": false,
        "alwaysOnTop": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 2: Update `lib/tauri.ts` with new types**

Replace entire `apps/baseus-app/src/lib/tauri.ts`:

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface BatteryState {
  left_pct: number;
  right_pct: number;
  left_charging: boolean;
  right_charging: boolean;
}

export interface CaseState {
  case_pct: number;
  case_charging: boolean;
}

export interface WearState {
  left_in_ear: boolean;
  right_in_ear: boolean;
}

export type AncMode = 'off' | 'anc' | 'transparency';

export type DeviceEvent =
  | { type: 'battery_update'; data: BatteryState }
  | { type: 'case_update'; data: CaseState }
  | { type: 'anc_mode_update'; data: AncMode }
  | { type: 'wear_update'; data: WearState }
  | { type: 'connected' }
  | { type: 'disconnected' };

export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export function onDeviceEvent(cb: (e: DeviceEvent) => void): Promise<UnlistenFn> {
  return listen<DeviceEvent>('device-event', (event) => cb(event.payload));
}

export function onConnectionState(cb: (s: ConnectionState) => void): Promise<UnlistenFn> {
  return listen<ConnectionState>('connection-state', (event) => cb(event.payload));
}

export interface Settings {
  launch_at_login: boolean;
  low_battery_alerts: boolean;
  show_session_timer: boolean;
}

export function setAncMode(mode: AncMode, level?: number): Promise<void> {
  return invoke('set_anc_mode', { mode, level });
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

- [ ] **Step 3: Update `stores/settings.ts` default**

Replace entire `apps/baseus-app/src/stores/settings.ts`:

```typescript
import { createSignal } from 'solid-js';
import { getSettings, setSettings, type Settings } from '../lib/tauri';

const [settings, setSettingsSignal] = createSignal<Settings>({
  launch_at_login: true,
  low_battery_alerts: true,
  show_session_timer: true,
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

- [ ] **Step 4: Commit**

```powershell
git add apps/baseus-app/src-tauri/tauri.conf.json apps/baseus-app/src/lib/tauri.ts apps/baseus-app/src/stores/settings.ts
git commit -m "feat(config): widen window to 480px, sync TS types with Rust"
```

---

## Task 3: Session timer

**Files:**
- Create: `apps/baseus-app/src/lib/timer.ts`

- [ ] **Step 1: Create `timer.ts`**

Create `apps/baseus-app/src/lib/timer.ts`:

```typescript
import { createSignal } from 'solid-js';

const [elapsed, setElapsed] = createSignal(0); // seconds since session start
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startTimer() {
  setElapsed(0);
  if (intervalId !== null) clearInterval(intervalId);
  intervalId = setInterval(() => setElapsed((s) => s + 1), 1000);
}

export function stopTimer() {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
  setElapsed(0);
}

// Return the signal accessor directly so callers can use it reactively in JSX:
//   elapsed={useElapsed()()}  — reactive; re-reads when timer ticks
export const useElapsed = () => elapsed;

export function formatElapsed(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}
```

- [ ] **Step 2: Commit**

```powershell
git add apps/baseus-app/src/lib/timer.ts
git commit -m "feat(timer): session elapsed time signal"
```

---

## Task 4: `Sidebar.tsx`

**Files:**
- Create: `apps/baseus-app/src/components/Sidebar.tsx`

- [ ] **Step 1: Create `Sidebar.tsx`**

```typescript
export type Tab = 'home' | 'anc' | 'eq' | 'gestures' | 'settings';

interface Props {
  active: Tab;
  onSwitch: (tab: Tab) => void;
}

const NAV: Array<{ tab: Tab; icon: string; label: string }> = [
  { tab: 'home',     icon: '⊙',  label: 'Battery' },
  { tab: 'anc',      icon: '◎',  label: 'Noise Control' },
  { tab: 'eq',       icon: '≋',  label: 'EQ' },
  { tab: 'gestures', icon: '⊡',  label: 'Gestures' },
];

export default function Sidebar(props: Props) {
  return (
    <div
      style={{
        width: '52px',
        background: '#0a0a0c',
        'border-right': '1px solid #161618',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        padding: '10px 0',
        gap: '2px',
        'flex-shrink': '0',
      }}
    >
      {NAV.map(({ tab, icon, label }) => (
        <button
          title={label}
          onClick={() => props.onSwitch(tab)}
          style={{
            width: '36px',
            height: '36px',
            'border-radius': '9px',
            border: 'none',
            background: props.active === tab ? 'rgba(99,102,241,0.18)' : 'transparent',
            color: props.active === tab ? '#a5b4fc' : '#444',
            'font-size': '18px',
            cursor: 'pointer',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            position: 'relative',
            transition: 'background 0.12s, color 0.12s',
          }}
        >
          {props.active === tab && (
            <div
              style={{
                position: 'absolute',
                left: '-8px',
                width: '3px',
                height: '18px',
                background: '#6366f1',
                'border-radius': '0 3px 3px 0',
              }}
            />
          )}
          {icon}
        </button>
      ))}

      {/* Spacer + settings at bottom */}
      <div style={{ flex: '1' }} />
      <button
        title="Settings"
        onClick={() => props.onSwitch('settings')}
        style={{
          width: '36px',
          height: '36px',
          'border-radius': '9px',
          border: 'none',
          background: props.active === 'settings' ? 'rgba(99,102,241,0.18)' : 'transparent',
          color: props.active === 'settings' ? '#a5b4fc' : '#444',
          'font-size': '18px',
          cursor: 'pointer',
          display: 'flex',
          'align-items': 'center',
          'justify-content': 'center',
          position: 'relative',
          transition: 'background 0.12s, color 0.12s',
        }}
      >
        {props.active === 'settings' && (
          <div
            style={{
              position: 'absolute',
              left: '-8px',
              width: '3px',
              height: '18px',
              background: '#6366f1',
              'border-radius': '0 3px 3px 0',
            }}
          />
        )}
        ⚙
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```powershell
git add apps/baseus-app/src/components/Sidebar.tsx
git commit -m "feat(ui): Sidebar navigation component"
```

---

## Task 5: `HomeTab.tsx`

**Files:**
- Create: `apps/baseus-app/src/components/HomeTab.tsx`

The existing `SparkLine.tsx` is reused as-is. The ring geometry: `r=28`, `circumference = 2*π*28 ≈ 175.9`.

- [ ] **Step 1: Create `HomeTab.tsx`**

```typescript
import SparkLine from './SparkLine';
import { findEarbud } from '../lib/tauri';
import { formatElapsed } from '../lib/timer';

interface Props {
  leftPct: number;
  rightPct: number;
  casePct: number;
  leftCharging: boolean;
  rightCharging: boolean;
  caseCharging: boolean;
  leftInEar: boolean;
  rightInEar: boolean;
  wearKnown: boolean; // false until a wear_update event arrives; greys out dots
  leftHistory: number[];
  rightHistory: number[];
  elapsed: number; // seconds
  showTimer: boolean;
}

const CIRC = 2 * Math.PI * 28;

function BudRing(p: {
  pct: number;
  charging: boolean;
  inEar: boolean;
  wearKnown: boolean;
  label: string;
  history: number[];
}) {
  const isLow = () => p.pct > 0 && p.pct < 20;
  const color = () => (isLow() ? '#ef4444' : '#22c55e');
  const offset = () => CIRC * (1 - p.pct / 100);
  const wearColor = () =>
    !p.wearKnown ? '#2a2a2a' : p.inEar ? '#22c55e' : '#3f3f3f';

  return (
    <div
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '14px 10px 10px',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        gap: '8px',
        position: 'relative',
      }}
    >
      {/* Wear detection dot */}
      <div
        title={!p.wearKnown ? 'Wear detection not yet available' : p.inEar ? 'In ear' : 'Not in ear'}
        style={{
          position: 'absolute',
          top: '10px',
          right: '10px',
          width: '7px',
          height: '7px',
          'border-radius': '50%',
          background: wearColor(),
          'box-shadow': p.wearKnown && p.inEar ? `0 0 6px ${wearColor()}` : 'none',
        }}
      />

      {/* Battery ring */}
      <div style={{ position: 'relative', width: '72px', height: '72px' }}>
        <svg width="72" height="72" viewBox="0 0 72 72" style={{ transform: 'rotate(-90deg)' }}>
          <circle cx="36" cy="36" r="28" fill="none" stroke="#1a1a1e" stroke-width="5" />
          <circle
            cx="36" cy="36" r="28" fill="none"
            stroke={color()} stroke-width="5"
            stroke-dasharray={String(CIRC)}
            stroke-dashoffset={String(offset())}
            stroke-linecap="round"
            style={{ transition: 'stroke-dashoffset 0.5s ease, stroke 0.3s ease' }}
          />
        </svg>
        <div
          style={{
            position: 'absolute', inset: '0',
            display: 'flex', 'flex-direction': 'column',
            'align-items': 'center', 'justify-content': 'center',
          }}
        >
          <span style={{ 'font-size': '20px', 'font-weight': '800', color: '#fff', 'line-height': '1' }}>
            {p.pct}
          </span>
          <span style={{ 'font-size': '9px', color: '#404040', 'font-weight': '500' }}>%</span>
        </div>
      </div>

      <SparkLine data={p.history} color={color()} width={80} height={22} />

      <div style={{ display: 'flex', 'align-items': 'center', gap: '4px' }}>
        <span style={{ 'font-size': '10px', color: '#444', 'font-weight': '600', 'letter-spacing': '0.05em' }}>
          {p.label}
        </span>
        {p.charging && (
          <span style={{ 'font-size': '10px', color: '#eab308' }}>⚡</span>
        )}
      </div>
    </div>
  );
}

export default function HomeTab(props: Props) {
  const caseColor = () =>
    props.casePct > 0 && props.casePct < 20 ? '#ef4444' : '#eab308';

  return (
    <div>
      {/* Battery */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Earbuds <Divider /></div>
        <div style={{ display: 'flex', gap: '10px' }}>
          <BudRing
            pct={props.leftPct} charging={props.leftCharging}
            inEar={props.leftInEar} wearKnown={props.wearKnown}
            label="LEFT" history={props.leftHistory}
          />
          <BudRing
            pct={props.rightPct} charging={props.rightCharging}
            inEar={props.rightInEar} wearKnown={props.wearKnown}
            label="RIGHT" history={props.rightHistory}
          />
        </div>
      </div>

      {/* Case */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Case <Divider /></div>
        <div
          style={{
            background: '#111113',
            border: '1px solid #1a1a1e',
            'border-radius': '12px',
            padding: '12px 14px',
            display: 'flex',
            'align-items': 'center',
            gap: '12px',
          }}
        >
          <span style={{ 'font-size': '20px', opacity: '0.5' }}>🗃️</span>
          <div style={{ flex: '1' }}>
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-bottom': '6px' }}>
              <span style={{ 'font-size': '11px', color: '#555' }}>Case</span>
              <span style={{ 'font-size': '13px', 'font-weight': '800', color: caseColor() }}>
                {props.casePct}%
              </span>
            </div>
            <div style={{ height: '4px', background: '#1a1a1e', 'border-radius': '99px', overflow: 'hidden' }}>
              <div
                style={{
                  height: '100%',
                  width: `${props.casePct}%`,
                  background: `linear-gradient(90deg, ${caseColor()}, ${caseColor()}aa)`,
                  'border-radius': '99px',
                  transition: 'width 0.5s ease',
                }}
              />
            </div>
          </div>
          {props.caseCharging && (
            <span style={{ 'font-size': '16px', color: '#eab308' }}>⚡</span>
          )}
        </div>
      </div>

      {/* Find Earbuds */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Find Earbuds <Divider /></div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <FindBtn label="Play Left"  onClick={() => findEarbud('left').catch(() => {})} />
          <FindBtn label="Play Right" onClick={() => findEarbud('right').catch(() => {})} />
        </div>
      </div>

      {/* Session timer */}
      {props.showTimer && props.elapsed > 0 && (
        <div style={{ 'margin-bottom': '4px' }}>
          <div style={labelStyle}>Today <Divider /></div>
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '12px 14px',
              display: 'flex',
              'align-items': 'center',
              gap: '10px',
            }}
          >
            <span style={{ 'font-size': '16px', opacity: '0.4' }}>⏱</span>
            <div>
              <div style={{ 'font-size': '17px', 'font-weight': '800', color: '#fff', 'font-variant-numeric': 'tabular-nums' }}>
                {formatElapsed(props.elapsed)}
              </div>
              <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '1px' }}>Current session</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

function FindBtn(p: { label: string; onClick: () => void }) {
  return (
    <button
      onClick={p.onClick}
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '10px',
        padding: '10px',
        color: '#888',
        'font-size': '12px',
        'font-weight': '500',
        cursor: 'pointer',
        transition: 'background 0.12s, border-color 0.12s',
      }}
    >
      {p.label}
    </button>
  );
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
```

- [ ] **Step 2: Commit**

```powershell
git add apps/baseus-app/src/components/HomeTab.tsx
git commit -m "feat(ui): HomeTab — battery rings, case bar, find buttons, session timer, wear dots"
```

---

## Task 6: `AncTab.tsx`

**Files:**
- Create: `apps/baseus-app/src/components/AncTab.tsx`

- [ ] **Step 1: Create `AncTab.tsx`**

```typescript
import type { AncMode } from '../lib/tauri';

interface Props {
  mode: AncMode;
  loading: AncMode | null;
  level: number; // 1–10
  onMode: (mode: AncMode) => void;
  onLevel: (level: number) => void;
}

const MODES: Array<{ mode: AncMode; icon: string; name: string; desc: string }> = [
  { mode: 'off',          icon: '🔇', name: 'Off',                     desc: 'Passthrough — no processing' },
  { mode: 'anc',          icon: '🎧', name: 'Active Noise Cancellation', desc: 'Blocks ambient sound' },
  { mode: 'transparency', icon: '🌬️', name: 'Transparency',             desc: 'Lets ambient sound in' },
];

export default function AncTab(props: Props) {
  const levelByte = (v: number) => Math.round(((v - 1) / 9) * (0xff - 0x10) + 0x10);

  function handleSlider(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    props.onLevel(v);
  }

  return (
    <div>
      <div style={labelStyle}>Noise Control <Divider /></div>

      <div style={{ display: 'flex', 'flex-direction': 'column', gap: '8px', 'margin-bottom': '16px' }}>
        {MODES.map(({ mode, icon, name, desc }) => {
          const isActive = () => props.mode === mode;
          const isLoading = () => props.loading === mode;
          return (
            <button
              onClick={() => props.onMode(mode)}
              style={{
                background: isActive() ? 'rgba(99,102,241,0.08)' : '#111113',
                border: `1px solid ${isActive() ? 'rgba(99,102,241,0.4)' : '#1a1a1e'}`,
                'border-radius': '12px',
                padding: '14px 16px',
                display: 'flex',
                'align-items': 'center',
                gap: '12px',
                cursor: 'pointer',
                transition: 'border-color 0.12s, background 0.12s',
                animation: isLoading() ? 'pulse 0.8s ease-in-out infinite' : 'none',
                width: '100%',
                'text-align': 'left',
              }}
            >
              <span style={{ 'font-size': '20px', width: '28px', 'text-align': 'center' }}>{icon}</span>
              <div style={{ flex: '1' }}>
                <div style={{ 'font-size': '13px', 'font-weight': '600', color: isActive() ? '#c7d2fe' : '#aaa' }}>
                  {name}
                </div>
                <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{desc}</div>
              </div>
              {isActive() && (
                <div
                  style={{
                    width: '16px', height: '16px', 'border-radius': '50%',
                    background: '#6366f1', display: 'flex',
                    'align-items': 'center', 'justify-content': 'center',
                    'font-size': '9px', color: '#fff', 'flex-shrink': '0',
                  }}
                >
                  ✓
                </div>
              )}
            </button>
          );
        })}
      </div>

      {/* Level slider — only meaningful for ANC and Transparency */}
      {props.mode !== 'off' && (
        <>
          <div style={labelStyle}>Strength <Divider /></div>
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '14px 16px',
            }}
          >
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-bottom': '12px' }}>
              <span style={{ 'font-size': '12px', color: '#888' }}>Level</span>
              <span style={{ 'font-size': '12px', 'font-weight': '700', color: '#818cf8' }}>
                {props.level} / 10
              </span>
            </div>
            <input
              type="range" min="1" max="10" value={props.level}
              onInput={handleSlider}
              style={{
                width: '100%', height: '4px',
                'accent-color': '#6366f1',
                cursor: 'pointer',
              }}
            />
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-top': '6px', 'font-size': '9px', color: '#333' }}>
              <span>Low</span><span>High</span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
```

- [ ] **Step 2: Commit**

```powershell
git add apps/baseus-app/src/components/AncTab.tsx
git commit -m "feat(ui): AncTab — mode cards and level slider"
```

---

## Task 7: `EqTab.tsx` and `GesturesTab.tsx` stubs

**Files:**
- Create: `apps/baseus-app/src/components/EqTab.tsx`
- Create: `apps/baseus-app/src/components/GesturesTab.tsx`

- [ ] **Step 1: Create `EqTab.tsx`**

```typescript
const PRESETS = [
  { id: 'balanced',  name: 'Balanced',   bars: [50, 55, 60, 55, 50] },
  { id: 'bass',      name: 'Bass Boost', bars: [100, 85, 55, 40, 35] },
  { id: 'voice',     name: 'Voice',      bars: [30, 60, 100, 80, 40] },
  { id: 'clear',     name: 'Clear',      bars: [35, 45, 55, 75, 100] },
];

export default function EqTab() {
  return (
    <div>
      <div style={labelStyle}>Sound Presets <Divider /></div>
      <div style={{ display: 'grid', 'grid-template-columns': '1fr 1fr', gap: '8px', 'margin-bottom': '14px' }}>
        {PRESETS.map(({ id, name, bars }) => (
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '12px',
              opacity: '0.45',
              cursor: 'not-allowed',
            }}
          >
            <div style={{ 'font-size': '12px', 'font-weight': '600', color: '#888', 'margin-bottom': '8px' }}>
              {name}
            </div>
            <div style={{ display: 'flex', 'align-items': 'flex-end', gap: '3px', height: '28px' }}>
              {bars.map((h) => (
                <div
                  style={{
                    flex: '1',
                    height: `${h}%`,
                    background: '#333',
                    'border-radius': '2px',
                  }}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
      <ReNotice feature="EQ presets" />
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

function ReNotice(p: { feature: string }) {
  return (
    <div
      style={{
        background: 'rgba(99,102,241,0.06)',
        border: '1px solid rgba(99,102,241,0.15)',
        'border-radius': '10px',
        padding: '12px 14px',
        'font-size': '11px',
        color: '#555',
        'line-height': '1.5',
      }}
    >
      <span style={{ color: '#818cf8', 'font-weight': '600' }}>Protocol RE needed — </span>
      {p.feature} require capturing the BLE write bytes from the Android app via Frida. Once captured, this tab enables automatically.
    </div>
  );
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
```

- [ ] **Step 2: Create `GesturesTab.tsx`**

```typescript
const TAPS = ['Double Tap', 'Triple Tap', 'Long Press'] as const;
const ACTIONS = ['Play / Pause', 'Next Track', 'Prev Track', 'ANC Toggle', 'Voice Assistant', 'Volume Up', 'Volume Down'];

const DEFAULTS: Record<typeof TAPS[number], [string, string]> = {
  'Double Tap': ['Play / Pause', 'Play / Pause'],
  'Triple Tap': ['Next Track',   'Prev Track'],
  'Long Press': ['ANC Toggle',   'Voice Assistant'],
};

export default function GesturesTab() {
  return (
    <div>
      {(['Left', 'Right'] as const).map((side, si) => (
        <div style={{ 'margin-bottom': '10px' }}>
          <div style={labelStyle}>{side} Bud <Divider /></div>
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '4px 14px',
              opacity: '0.5',
            }}
          >
            {TAPS.map((tap) => (
              <div
                style={{
                  display: 'flex',
                  'align-items': 'center',
                  'justify-content': 'space-between',
                  padding: '10px 0',
                  'border-bottom': tap !== 'Long Press' ? '1px solid #161618' : 'none',
                }}
              >
                <span style={{ 'font-size': '10px', color: '#444', 'font-weight': '600', 'letter-spacing': '0.04em', 'text-transform': 'uppercase' }}>
                  {tap}
                </span>
                <div
                  style={{
                    'font-size': '11px',
                    color: '#777',
                    background: '#161618',
                    border: '1px solid #222',
                    'border-radius': '6px',
                    padding: '4px 10px',
                    cursor: 'not-allowed',
                  }}
                >
                  {DEFAULTS[tap][si]} ▾
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
      <ReNotice />
    </div>
  );
}

function ReNotice() {
  return (
    <div
      style={{
        background: 'rgba(99,102,241,0.06)',
        border: '1px solid rgba(99,102,241,0.15)',
        'border-radius': '10px',
        padding: '12px 14px',
        'font-size': '11px',
        color: '#555',
        'line-height': '1.5',
      }}
    >
      <span style={{ color: '#818cf8', 'font-weight': '600' }}>Protocol RE needed — </span>
      Gesture remapping requires capturing the BLE write bytes from the Android app via Frida.
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
```

- [ ] **Step 3: Commit**

```powershell
git add apps/baseus-app/src/components/EqTab.tsx apps/baseus-app/src/components/GesturesTab.tsx
git commit -m "feat(ui): EqTab and GesturesTab stubs with RE notice"
```

---

## Task 8: `SettingsTab.tsx`

**Files:**
- Create: `apps/baseus-app/src/components/SettingsTab.tsx`

- [ ] **Step 1: Create `SettingsTab.tsx`**

```typescript
import { getSettingsStore, updateSetting } from '../stores/settings';

export default function SettingsTab() {
  return (
    <div>
      <div style={labelStyle}>Preferences <Divider /></div>
      <Toggle
        label="Launch at login"
        desc="Start automatically with Windows"
        value={getSettingsStore().launch_at_login}
        onChange={(v) => updateSetting('launch_at_login', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Low battery alerts"
        desc="Notify when a bud drops below 20%"
        value={getSettingsStore().low_battery_alerts}
        onChange={(v) => updateSetting('low_battery_alerts', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Show session timer"
        desc="Display listening time on the home tab"
        value={getSettingsStore().show_session_timer}
        onChange={(v) => updateSetting('show_session_timer', v)}
      />
    </div>
  );
}

function Toggle(p: { label: string; desc: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div style={{ display: 'flex', 'align-items': 'center', padding: '14px 0' }}>
      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '13px', color: '#ccc', 'font-weight': '500' }}>{p.label}</div>
        <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{p.desc}</div>
      </div>
      <div
        onClick={() => p.onChange(!p.value)}
        style={{
          width: '36px', height: '20px',
          'border-radius': '99px',
          background: p.value ? '#22c55e' : '#222',
          position: 'relative',
          cursor: 'pointer',
          transition: 'background 0.2s',
          'flex-shrink': '0',
        }}
      >
        <div
          style={{
            position: 'absolute',
            width: '16px', height: '16px',
            'border-radius': '50%',
            background: '#fff',
            top: '2px',
            left: p.value ? 'auto' : '2px',
            right: p.value ? '2px' : 'auto',
            transition: 'left 0.2s, right 0.2s',
            'box-shadow': '0 1px 4px rgba(0,0,0,0.4)',
          }}
        />
      </div>
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
```

- [ ] **Step 2: Commit**

```powershell
git add apps/baseus-app/src/components/SettingsTab.tsx
git commit -m "feat(ui): SettingsTab with show_session_timer toggle"
```

---

## Task 9: Rewrite `App.tsx`

**Files:**
- Modify: `apps/baseus-app/src/App.tsx`

This is the wiring task. `App.tsx` owns all signals and passes props down. All five tab components are always rendered; the active one is shown via `display:block`, the others `display:none`.

- [ ] **Step 1: Replace entire `App.tsx`**

```typescript
import { createSignal, onCleanup, onMount } from 'solid-js';
import Sidebar, { type Tab } from './components/Sidebar';
import HomeTab from './components/HomeTab';
import AncTab from './components/AncTab';
import EqTab from './components/EqTab';
import GesturesTab from './components/GesturesTab';
import SettingsTab from './components/SettingsTab';
import { onDeviceEvent, onConnectionState, setAncMode, type AncMode, type WearState } from './lib/tauri';
import { pushLeft, pushRight, pushCase, left, right, caseData } from './stores/batteryHistory';
import { loadSettings, getSettingsStore } from './stores/settings';
import { startTimer, stopTimer, useElapsed } from './lib/timer';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [ancMode, setAncModeSignal] = createSignal<AncMode>('off');
  const [ancLoading, setAncLoading] = createSignal<AncMode | null>(null);
  const [ancLevel, setAncLevel] = createSignal(7);
  const [activeTab, setActiveTab] = createSignal<Tab>('home');
  const [leftCharging, setLeftCharging] = createSignal(false);
  const [rightCharging, setRightCharging] = createSignal(false);
  const [caseCharging, setCaseCharging] = createSignal(false);
  const [wear, setWear] = createSignal<WearState | null>(null);

  onMount(async () => {
    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    await loadSettings();

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        pushLeft(e.data.left_pct);
        pushRight(e.data.right_pct);
        setLeftCharging(e.data.left_charging);
        setRightCharging(e.data.right_charging);
      } else if (e.type === 'case_update') {
        pushCase(e.data.case_pct);
        setCaseCharging(e.data.case_charging);
      } else if (e.type === 'anc_mode_update') {
        setAncModeSignal(e.data);
        setAncLoading(null);
      } else if (e.type === 'wear_update') {
        setWear(e.data);
      }
    }).then((fn) => unlisteners.push(fn));

    onConnectionState((s) => {
      setStatus(s);
      if (s === 'connected') startTimer();
      else stopTimer();
    }).then((fn) => unlisteners.push(fn));
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    const byte = Math.round(((ancLevel() - 1) / 9) * (0xff - 0x10) + 0x10);
    try {
      await setAncMode(mode, mode === 'off' ? undefined : byte);
    } catch {
      setAncLoading(null);
    }
  }

  async function handleLevel(v: number) {
    setAncLevel(v);
    const mode = ancMode();
    if (mode !== 'off') {
      const byte = Math.round(((v - 1) / 9) * (0xff - 0x10) + 0x10);
      await setAncMode(mode, byte).catch(() => {});
    }
  }

  const statusColor = () =>
    status() === 'connected' ? '#22c55e' : status() === 'connecting' ? '#eab308' : '#525252';

  const statusText = () =>
    status() === 'connected' ? 'Connected' : status() === 'connecting' ? 'Connecting…' : 'Disconnected';

  const tab = (id: Tab) => ({ display: activeTab() === id ? 'block' : 'none' });

  return (
    <div
      style={{
        width: '480px',
        'min-height': '620px',
        background: '#0d0d0f',
        color: '#fff',
        'font-family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        'box-sizing': 'border-box',
        display: 'flex',
        'flex-direction': 'column',
      }}
    >
      {/* Title bar */}
      <div
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          padding: '12px 16px 10px',
          'border-bottom': '1px solid #161618',
          'flex-shrink': '0',
        }}
      >
        <div style={{ display: 'flex', gap: '5px' }}>
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
            color: '#444',
            'margin-left': '-40px',
          }}
        >
          Bass BP1 Pro ANC
        </div>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '5px', 'font-size': '11px', color: statusColor(), 'font-weight': '500' }}>
          <div style={{ width: '6px', height: '6px', background: statusColor(), 'border-radius': '50%' }} />
          {statusText()}
        </div>
      </div>

      {/* Body: sidebar + content */}
      <div style={{ display: 'flex', flex: '1' }}>
        <Sidebar active={activeTab()} onSwitch={setActiveTab} />

        <div style={{ flex: '1', padding: '16px', 'overflow-y': 'auto' }}>
          <div style={tab('home')}>
            <HomeTab
              leftPct={left()[left().length - 1]?.pct ?? 0}
              rightPct={right()[right().length - 1]?.pct ?? 0}
              casePct={caseData()[caseData().length - 1]?.pct ?? 0}
              leftCharging={leftCharging()}
              rightCharging={rightCharging()}
              caseCharging={caseCharging()}
              leftInEar={wear()?.left_in_ear ?? false}
              rightInEar={wear()?.right_in_ear ?? false}
              wearKnown={wear() !== null}
              leftHistory={left().map((r) => r.pct)}
              rightHistory={right().map((r) => r.pct)}
              elapsed={useElapsed()()}
              showTimer={getSettingsStore().show_session_timer}
            />
          </div>

          <div style={tab('anc')}>
            <AncTab
              mode={ancMode()}
              loading={ancLoading()}
              level={ancLevel()}
              onMode={handleAnc}
              onLevel={handleLevel}
            />
          </div>

          <div style={tab('eq')}>
            <EqTab />
          </div>

          <div style={tab('gestures')}>
            <GesturesTab />
          </div>

          <div style={tab('settings')}>
            <SettingsTab />
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Run TypeScript type check**

```powershell
cd apps/baseus-app; pnpm exec tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```powershell
git add apps/baseus-app/src/App.tsx
git commit -m "feat(ui): rewrite App.tsx — dashboard layout with sidebar, all tabs wired"
```

---

## Task 10: Delete old components and verify

**Files:**
- Delete: `apps/baseus-app/src/components/BudCard.tsx`
- Delete: `apps/baseus-app/src/components/CaseCard.tsx`
- Delete: `apps/baseus-app/src/components/AncButton.tsx`
- Delete: `apps/baseus-app/src/components/FindButton.tsx`
- Delete: `apps/baseus-app/src/components/SettingRow.tsx`
- Delete: `apps/baseus-app/src/components/ConnectionCard.tsx`
- Delete: `apps/baseus-app/src/components/BatteryCard.tsx`

- [ ] **Step 1: Delete old component files**

```powershell
cd apps/baseus-app
Remove-Item src/components/BudCard.tsx
Remove-Item src/components/CaseCard.tsx
Remove-Item src/components/AncButton.tsx
Remove-Item src/components/FindButton.tsx
Remove-Item src/components/SettingRow.tsx
Remove-Item src/components/ConnectionCard.tsx
Remove-Item src/components/BatteryCard.tsx
```

- [ ] **Step 2: TypeScript check — confirm no lingering imports**

```powershell
pnpm exec tsc --noEmit
```

Expected: no errors. If any "cannot find module" errors appear, a tab component still imports a deleted file — fix the import before proceeding.

- [ ] **Step 3: Run Rust tests**

```powershell
cargo test -p baseus-protocol
```

Expected: all tests pass.

- [ ] **Step 4: Start the app and verify visually**

```powershell
pnpm tauri dev
```

Check the following:
- Window is 480px wide (measure if unsure)
- Sidebar shows 4 icons + settings gear at bottom
- Home tab: battery rings populate when buds connect, case bar shows, find buttons work
- ANC tab: three mode cards, clicking each sends the command, active card highlights; slider appears when ANC or Transparency is active
- EQ tab: greyed out preset cards, RE notice visible
- Gestures tab: greyed out dropdowns, RE notice visible
- Settings tab: three toggles work, "Show session timer" toggle hides/shows timer on home tab

- [ ] **Step 5: Commit**

```powershell
git add -A
git commit -m "chore: remove old widget components superseded by dashboard tabs"
```
