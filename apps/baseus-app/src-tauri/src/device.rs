use std::time::Duration;

use baseus_protocol::{framing::Frame, models::bp1_pro_anc::Bp1ProAnc, types::AncMode};
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

pub async fn run_loop(app: AppHandle, mut cmd_rx: CommandReceiver) {
    loop {
        let _ = app.emit("connection-state", "connecting");
        match GattTransport::connect(DEVICE_NAME).await {
            Ok(mut transport) => {
                let _ = app.emit("connection-state", "connected");
                // BA 05 00 = handshake; triggers the device to start pushing notifications.
                // Confirmed from HomeBleDataResolvePresenter$2.b() bytecode analysis.
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}
