use std::time::Duration;

use baseus_protocol::{
    framing::Frame,
    models::{bp1_pro_anc::Bp1ProAnc, inspire_xh1::InspireXh1},
    types::{AncMode, BaseusModel, DeviceEvent, EqPreset, ModelStatus},
};
use baseus_transport::win::ble::GattTransport;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

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
    SetEqPreset(EqPreset),
    SetGameMode(bool),
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

/// Model identity emitted to the frontend after a successful connection.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: &'static str,
    pub status: ModelStatus,
}

pub async fn run_loop(app: AppHandle, mut cmd_rx: CommandReceiver) {
    // Build the device list from the protocol registry — (advertising_name, notify_uuid, write_uuid)
    let device_entries: Vec<(&str, &str, &str)> = BaseusModel::all()
        .iter()
        .flat_map(|m| {
            let (notify, write) = m.gatt_uuids();
            m.advertising_names()
                .iter()
                .map(move |&name| (name, notify, write))
        })
        .collect();

    loop {
        let _ = app.emit("connection-state", "connecting");

        match GattTransport::connect_any(&device_entries).await {
            Ok((mut transport, device_idx)) => {
                // Determine which model this device index corresponds to.
                let mut offset = 0;
                let mut connected_model = BaseusModel::Bp1ProAnc;
                for model in BaseusModel::all() {
                    let name_count = model.advertising_names().len();
                    if device_idx < offset + name_count {
                        connected_model = *model;
                        break;
                    }
                    offset += name_count;
                }

                tracing::info!(
                    "connected to {:?} ({})",
                    connected_model,
                    connected_model.display_name()
                );
                let _ = app.emit("connection-state", "connected");
                let _ = app.emit(
                    "model-info",
                    &ModelInfo {
                        name: connected_model.display_name(),
                        status: connected_model.status(),
                    },
                );

                // Send handshake (works for BP1; harmless no-op if ignored by other models).
                if let Err(e) = transport.send(&[0xBA, 0x05, 0x00]).await {
                    tracing::warn!("handshake send failed: {e}");
                }

                notification_loop(&app, &mut transport, &mut cmd_rx, connected_model).await;
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
    model: BaseusModel,
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
                            let event = match model {
                                // XP1/XC1 share the same earbud protocol as BP1 (same 0x0C ANC
                                // family, same battery framing) — APK-extracted, unverified.
                                BaseusModel::Bp1ProAnc
                                | BaseusModel::InspireXp1
                                | BaseusModel::InspireXc1 => {
                                    if frame.cmd == 0x34 {
                                        // Ack payload semantics vary by firmware (issue #3):
                                        // some units send a flat AA 34 01 for every ANC command,
                                        // including Off — resolve against the last commanded mode.
                                        Some(DeviceEvent::AncModeUpdate(Bp1ProAnc::resolve_anc_ack(
                                            &frame.payload,
                                            last_anc_mode.map(|(m, _)| m),
                                        )))
                                    } else {
                                        match Bp1ProAnc::decode_frame(&frame) {
                                            Ok(ev) => Some(ev),
                                            Err(e) => {
                                                tracing::debug!("unhandled frame cmd={:#04x}: {e}", frame.cmd);
                                                None
                                            }
                                        }
                                    }
                                }
                                BaseusModel::InspireXh1 => {
                                    match InspireXh1::decode_frame(&frame) {
                                        Ok(ev) => Some(ev),
                                        Err(e) => {
                                            tracing::debug!("unhandled frame cmd={:#04x}: {e}", frame.cmd);
                                            None
                                        }
                                    }
                                }
                            };
                            if let Some(ev) = event {
                                maybe_alert_battery(app, &ev, &mut thresholds);
                                let _ = app.emit("device-event", &ev);
                            }
                        } else if data.len() >= 2 && data[0] == 0xAA {
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
                match execute_command(transport, &cmd, model).await {
                    Ok(()) => {
                        tracing::debug!("command sent ok");
                        match &cmd {
                            DeviceCommand::SetAncMode(mode, level) => {
                                // Track Off too — flat-ack firmware (issue #3) answers Off
                                // with AA 34 01, and resolve_anc_ack needs to know Off was
                                // the last commanded mode to not misreport it as Anc.
                                last_anc_mode = Some((*mode, *level));
                                let _ = app.emit("device-event", &DeviceEvent::AncModeUpdate(*mode));
                            }
                            DeviceCommand::SetEqPreset(preset) => {
                                let _ = app.emit("device-event", &DeviceEvent::EqPresetUpdate(*preset));
                            }
                            DeviceCommand::SetGameMode(on) => {
                                let _ = app.emit("device-event", &DeviceEvent::GameModeUpdate(*on));
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
    model: BaseusModel,
) -> Result<(), String> {
    let bytes: Vec<u8> = match (cmd, model) {
        // BP1 / XP1 / XC1 — verified for BP1; XP1/XC1 assumed same protocol (APK-extracted).
        (
            DeviceCommand::SetAncMode(AncMode::Off, _),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x34, 0x00, 0xFF],
        (
            DeviceCommand::SetAncMode(AncMode::Anc, level),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x34, 0x01, *level],
        (
            DeviceCommand::SetAncMode(AncMode::Transparency, level),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x34, 0x02, *level],
        (
            DeviceCommand::SetEqPreset(preset),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x43, preset.to_byte()],
        // Game/low-latency mode — verified for BP1 over both SPP and BLE (issue #3).
        (
            DeviceCommand::SetGameMode(on),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x24, u8::from(*on)],
        (
            DeviceCommand::FindEarbud(Side::Left),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x10, 0x00, 0x01],
        (
            DeviceCommand::FindEarbud(Side::Right),
            BaseusModel::Bp1ProAnc | BaseusModel::InspireXp1 | BaseusModel::InspireXc1,
        ) => vec![0xBA, 0x10, 0x01, 0x01],
        // Inspire XH1 — setting ANC/EQ not yet supported (wire format unverified).
        (DeviceCommand::SetAncMode(mode, _), BaseusModel::InspireXh1) => {
            tracing::info!("XH1 ANC set for {mode:?} not yet supported — wire format unverified");
            return Ok(());
        }
        (DeviceCommand::SetEqPreset(_), BaseusModel::InspireXh1) => {
            tracing::info!("XH1 EQ preset not yet supported — wire format unverified");
            return Ok(());
        }
        (DeviceCommand::SetGameMode(_), BaseusModel::InspireXh1) => {
            tracing::info!("XH1 game mode not yet supported — wire format unverified");
            return Ok(());
        }
        (DeviceCommand::FindEarbud(_), BaseusModel::InspireXh1) => {
            tracing::info!("XH1 find not yet supported");
            return Ok(());
        }
        // Adaptive ANC modes — only used by XH1; unreachable for earbud models.
        (DeviceCommand::SetAncMode(_, _), _) => {
            return Err("ANC mode not supported for this model".to_string());
        }
    };
    transport.send(&bytes).await.map_err(|e| e.to_string())
}

fn maybe_alert_battery(app: &AppHandle, event: &DeviceEvent, thresholds: &mut BatteryThresholds) {
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
                let _ = app
                    .notification()
                    .builder()
                    .title("Baseus — Left bud low")
                    .body(format!("{}% remaining", b.left_pct))
                    .show();
            }
            if thresholds.right_was_ok && !right_now_ok {
                let _ = app
                    .notification()
                    .builder()
                    .title("Baseus — Right bud low")
                    .body(format!("{}% remaining", b.right_pct))
                    .show();
            }
            thresholds.left_was_ok = left_now_ok;
            thresholds.right_was_ok = right_now_ok;
        }
        DeviceEvent::HeadphoneBatteryUpdate(h) => {
            let now_ok = h.pct >= LOW || h.pct == 0;
            if thresholds.left_was_ok && !now_ok {
                let _ = app
                    .notification()
                    .builder()
                    .title("Baseus — Headphone low")
                    .body(format!("{}% remaining", h.pct))
                    .show();
            }
            thresholds.left_was_ok = now_ok;
        }
        DeviceEvent::CaseUpdate(c) => {
            let case_now_ok = c.case_pct >= LOW || c.case_pct == 0;
            if thresholds.case_was_ok && !case_now_ok {
                let _ = app
                    .notification()
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
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
