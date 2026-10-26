use std::time::Duration;

use baseus_protocol::{
    framing::WireFormat,
    models::{
        bp1_pro_anc::Bp1ProAnc,
        bp1_ultra::{self, Bp1Ultra},
    },
    types::{looks_like_baseus, AncMode, BaseusModel, DeviceEvent, EqPreset},
};
use baseus_transport::{win::ble::GattTransport, DeviceMatch, TransportError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

const RETRY_DELAY: Duration = Duration::from_secs(5);
const NOTIF_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for the first notification, which identifies the model when
/// several models share a GATT service (BP1 Pro / BP1 Ultra).
const IDENTIFY_TIMEOUT: Duration = Duration::from_secs(3);
/// The BP1 Ultra can drop a write without acking it, or reject it (`<cmd> 0B` — seen
/// for low-latency mode in some audio states), so after every change the matching
/// state is re-read after this delay and the UI follows the device, not the click.
const ULTRA_STATE_RECHECK: Duration = Duration::from_millis(1500);

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

impl DeviceCommand {
    /// Which control a `command-error` belongs to, so the frontend can revert it.
    fn kind(&self) -> &'static str {
        match self {
            Self::SetAncMode(..) => "anc",
            Self::SetEqPreset(_) => "eq",
            Self::SetGameMode(_) => "game",
            Self::FindEarbud(_) => "find",
        }
    }
}

/// A command the transport failed to write, emitted as `command-error`.
#[derive(Debug, Clone, Serialize)]
struct CommandError {
    command: &'static str,
    message: String,
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

/// Queue a command for the connected earbuds. Refused while disconnected: nothing reads
/// the queue then, so the command would sit there and replay on the next connection.
pub fn send_command(app: &AppHandle, cmd: DeviceCommand) -> Result<(), String> {
    let connected = app
        .state::<SharedSnapshot>()
        .lock()
        .is_ok_and(|s| s.state == "connected");
    if !connected {
        return Err("Earbuds aren't connected".into());
    }
    app.state::<CommandSender>()
        .send(cmd)
        .map_err(|e| e.to_string())
}

/// Model identity and capabilities emitted to the frontend after a successful connection.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: &'static str,
    /// Number of discrete ANC strength levels, or `None` for a continuous 1–10 slider.
    pub anc_levels: Option<u8>,
    /// Noise-control modes that take a strength value.
    pub strength_modes: &'static [AncMode],
    pub find_earbud: bool,
    /// Device-side names of the four EQ presets (Balanced, Bass Boost, Voice, Clear).
    pub eq_labels: Option<[&'static str; 4]>,
    /// Whether `eq_preset_update` events reflect the device's actual preset and should
    /// drive the UI. Only verified for the BP1 Ultra (`30 IDX`, pushed on connect).
    pub eq_state: bool,
}

/// Everything published for the current connection. `connection-state`, `model-info`
/// and most device reports are emitted once, and the webview loads in the background
/// while the BLE link comes up, so a frontend that subscribes a few milliseconds late
/// misses them (seen with a cached peripheral that connects within a second of launch:
/// the window stayed on "Connecting…"). The frontend reads this after subscribing.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSnapshot {
    pub state: &'static str,
    pub model: Option<ModelInfo>,
    /// Latest device event of each kind since the connection started.
    pub events: Vec<DeviceEvent>,
    /// Why the last attempt failed; kept across retries, cleared on connect.
    pub issue: Option<ConnectionIssue>,
    /// Last ANC strength byte commanded or reported, reused by the tray's mode toggles.
    #[serde(skip)]
    pub anc_level: Option<u8>,
}

/// Why the last connection attempt failed. Shown while the app keeps retrying, so a
/// search that never succeeds explains itself instead of sitting on "Connecting…"
/// (issues #8 and #10).
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionIssue {
    /// `adapter` (Bluetooth off or missing), `not_found` (nothing supported nearby) or
    /// `connect_failed` (earbuds found, but the control channel could not be opened).
    pub kind: &'static str,
    pub detail: String,
    /// Nearby devices whose names look like Baseus models this app doesn't support.
    pub unsupported: Vec<String>,
    pub supported: Vec<&'static str>,
}

impl ConnectionIssue {
    fn from_error(e: &TransportError) -> Self {
        let (kind, unsupported) = match e {
            TransportError::AdapterUnavailable(_) => ("adapter", Vec::new()),
            TransportError::DeviceNotFound { nearby } => (
                "not_found",
                nearby
                    .iter()
                    .filter(|name| looks_like_baseus(name))
                    .cloned()
                    .collect(),
            ),
            _ => ("connect_failed", Vec::new()),
        };
        Self {
            kind,
            detail: e.to_string(),
            unsupported,
            supported: BaseusModel::all()
                .iter()
                .map(|m| m.display_name())
                .collect(),
        }
    }
}

impl Default for DeviceSnapshot {
    fn default() -> Self {
        Self {
            state: "connecting",
            model: None,
            events: Vec::new(),
            issue: None,
            anc_level: None,
        }
    }
}

pub type SharedSnapshot = std::sync::Mutex<DeviceSnapshot>;

fn publish_state(app: &AppHandle, state: &'static str) {
    if let Ok(mut snapshot) = app.state::<SharedSnapshot>().lock() {
        snapshot.state = state;
        if state == "connecting" {
            snapshot.model = None;
            snapshot.events.clear();
            snapshot.anc_level = None;
        }
    }
    let _ = app.emit("connection-state", state);
    crate::tray::refresh(app);
}

/// Emit a device event and remember it as the latest of its kind.
fn emit_event(app: &AppHandle, event: &DeviceEvent) {
    if let Ok(mut snapshot) = app.state::<SharedSnapshot>().lock() {
        let kind = std::mem::discriminant(event);
        snapshot
            .events
            .retain(|e| std::mem::discriminant(e) != kind);
        snapshot.events.push(event.clone());
        if let DeviceEvent::AncLevelUpdate(level) = event {
            snapshot.anc_level = Some(*level);
        }
    }
    let _ = app.emit("device-event", event);
    crate::tray::refresh(app);
}

fn publish_issue(app: &AppHandle, issue: Option<ConnectionIssue>) {
    if let Ok(mut snapshot) = app.state::<SharedSnapshot>().lock() {
        snapshot.issue = issue.clone();
    }
    let _ = app.emit("connection-issue", &issue);
}

fn publish_model(app: &AppHandle, info: ModelInfo) {
    if let Ok(mut snapshot) = app.state::<SharedSnapshot>().lock() {
        snapshot.model = Some(info.clone());
    }
    let _ = app.emit("model-info", &info);
    crate::tray::refresh(app);
}

impl ModelInfo {
    fn of(model: BaseusModel) -> Self {
        let (eq_labels, eq_state) = match model {
            BaseusModel::Bp1ProAnc => (None, false),
            BaseusModel::Bp1Ultra => (
                Some(["Baseus Classic", "Powerful Bass", "Podcast", "Clear Treble"]),
                true,
            ),
        };
        Self {
            name: model.display_name(),
            anc_levels: model.anc_levels(),
            strength_modes: model.strength_modes(),
            find_earbud: model.supports_find_earbud(),
            eq_labels,
            eq_state,
        }
    }
}

pub async fn run_loop(app: AppHandle, mut cmd_rx: CommandReceiver) {
    // Build the device list from the protocol registry.
    let device_entries: Vec<DeviceMatch> = BaseusModel::all()
        .iter()
        .flat_map(|m| {
            let (notify_uuid, write_uuid) = m.gatt_uuids();
            let service_uuid = m.service_uuid();
            m.advertising_names().iter().map(move |&name| DeviceMatch {
                name,
                service_uuid,
                notify_uuid,
                write_uuid,
            })
        })
        .collect();

    loop {
        publish_state(&app, "connecting");

        match GattTransport::connect_any(&device_entries).await {
            Ok((mut transport, device_idx)) => {
                // Determine which model this device index corresponds to.
                let mut offset = 0;
                let mut advertised_model = BaseusModel::Bp1ProAnc;
                for model in BaseusModel::all() {
                    let name_count = model.advertising_names().len();
                    if device_idx < offset + name_count {
                        advertised_model = *model;
                        break;
                    }
                    offset += name_count;
                }

                // BP1 handshake. The BP1 Ultra ignores plain `BA` frames, so it is
                // harmless there, and it is needed before the model is known.
                if let Err(e) = transport.send(&[0xBA, 0x05, 0x00]).await {
                    tracing::warn!("handshake send failed: {e}");
                }

                // Models sharing a GATT service are told apart by their wire format.
                let first = identify(&mut transport).await;
                let connected_model = first
                    .as_ref()
                    .and_then(|data| WireFormat::detect(data))
                    .map(BaseusModel::from_wire_format)
                    .unwrap_or(advertised_model);

                tracing::info!(
                    "connected to {:?} ({})",
                    connected_model,
                    connected_model.display_name()
                );
                // Drop anything queued as the previous connection went down, so an old
                // click doesn't replay on these earbuds.
                while cmd_rx.try_recv().is_ok() {}
                // Model first: the UI must know the model's capabilities before it
                // starts sending commands to a "connected" device.
                publish_model(&app, ModelInfo::of(connected_model));
                publish_issue(&app, None);
                publish_state(&app, "connected");

                if connected_model == BaseusModel::Bp1Ultra {
                    for query in Bp1Ultra::initial_queries() {
                        if let Err(e) = transport.send(&query).await {
                            tracing::warn!("initial query failed: {e}");
                        }
                    }
                }

                notification_loop(&app, &mut transport, &mut cmd_rx, connected_model, first).await;
                publish_state(&app, "disconnected");
            }
            Err(e) => {
                tracing::warn!("connect failed: {e}");
                publish_issue(&app, Some(ConnectionIssue::from_error(&e)));
                publish_state(&app, "disconnected");
            }
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Wait briefly for the first notification. Both BP1 models push a battery report
/// right after the notify subscription; its envelope tells the models apart.
async fn identify(transport: &mut GattTransport) -> Option<Vec<u8>> {
    match tokio::time::timeout(IDENTIFY_TIMEOUT, transport.next_notification()).await {
        Ok(Ok(data)) => Some(data),
        _ => None,
    }
}

/// Decode one raw notification into the events it carries for `model`.
fn decode_notification(
    model: BaseusModel,
    data: &[u8],
    last_anc_mode: Option<AncMode>,
) -> Vec<DeviceEvent> {
    let frame = match model.wire_format().decode(data) {
        Ok(frame) => frame,
        Err(e) => {
            tracing::debug!("rejected notification {}: {e}", hex(data));
            return Vec::new();
        }
    };
    match model {
        // Ack payload semantics vary by firmware (issue #3): some units send a flat
        // AA 34 01 for every ANC command, including Off — resolve against the last
        // commanded mode.
        BaseusModel::Bp1ProAnc if frame.cmd == 0x34 => vec![DeviceEvent::AncModeUpdate(
            Bp1ProAnc::resolve_anc_ack(&frame.payload, last_anc_mode),
        )],
        BaseusModel::Bp1ProAnc => match Bp1ProAnc::decode_frame(&frame) {
            Ok(ev) => vec![ev],
            Err(e) => {
                tracing::debug!("unhandled frame cmd={:#04x}: {e}", frame.cmd);
                Vec::new()
            }
        },
        BaseusModel::Bp1Ultra => {
            let mut events = Vec::with_capacity(2);
            match Bp1Ultra::decode_frame(&frame) {
                Ok(ev) => events.push(ev),
                Err(e) => tracing::debug!("unhandled frame cmd={:#04x}: {e}", frame.cmd),
            }
            if frame.cmd == bp1_ultra::op::ANC_STATE {
                if let Some(level) = Bp1Ultra::anc_level(&frame.payload) {
                    events.push(DeviceEvent::AncLevelUpdate(level));
                }
            }
            events
        }
    }
}

fn emit_notification(
    app: &AppHandle,
    model: BaseusModel,
    data: &[u8],
    last_anc_mode: Option<AncMode>,
    thresholds: &mut BatteryThresholds,
) {
    tracing::debug!("raw notification: {}", hex(data));
    for ev in decode_notification(model, data, last_anc_mode) {
        maybe_alert_battery(app, &ev, thresholds);
        emit_event(app, &ev);
    }
}

async fn notification_loop(
    app: &AppHandle,
    transport: &mut GattTransport,
    cmd_rx: &mut CommandReceiver,
    model: BaseusModel,
    first: Option<Vec<u8>>,
) {
    let mut thresholds = BatteryThresholds {
        left_was_ok: true,
        right_was_ok: true,
        case_was_ok: true,
    };
    let mut last_anc_mode: Option<(AncMode, u8)> = None;
    let (find_stop_tx, mut find_stop_rx) = tokio::sync::mpsc::unbounded_channel::<Side>();
    // State opcodes to re-query on the BP1 Ultra after a write (see ULTRA_STATE_RECHECK).
    let (recheck_tx, mut recheck_rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let schedule_recheck = |opcode: u8| {
        if model == BaseusModel::Bp1Ultra {
            let tx = recheck_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(ULTRA_STATE_RECHECK).await;
                let _ = tx.send(opcode);
            });
        }
    };

    // The frame used to identify the model is a real report — don't drop it.
    if let Some(data) = first {
        emit_notification(app, model, &data, None, &mut thresholds);
    }

    loop {
        tokio::select! {
            result = tokio::time::timeout(NOTIF_TIMEOUT, transport.next_notification()) => {
                match result {
                    Ok(Ok(data)) => emit_notification(
                        app,
                        model,
                        &data,
                        last_anc_mode.map(|(m, _)| m),
                        &mut thresholds,
                    ),
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
                                if *mode != AncMode::Off {
                                    if let Ok(mut snapshot) = app.state::<SharedSnapshot>().lock() {
                                        snapshot.anc_level = Some(*level);
                                    }
                                }
                                emit_event(app, &DeviceEvent::AncModeUpdate(*mode));
                                schedule_recheck(bp1_ultra::op::ANC_STATE);
                            }
                            DeviceCommand::SetEqPreset(preset) => {
                                emit_event(app, &DeviceEvent::EqPresetUpdate(*preset));
                                schedule_recheck(bp1_ultra::op::EQ_STATE);
                            }
                            DeviceCommand::SetGameMode(on) => {
                                emit_event(app, &DeviceEvent::GameModeUpdate(*on));
                                schedule_recheck(bp1_ultra::op::LOW_LATENCY_STATE);
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
                    Err(e) => {
                        tracing::warn!("command error: {e}");
                        let _ = app.emit(
                            "command-error",
                            CommandError { command: cmd.kind(), message: e },
                        );
                    }
                }
            }
            Some(opcode) = recheck_rx.recv() => {
                // The answer arrives as a regular state notification and updates the UI.
                if let Err(e) = transport.send(&Bp1Ultra::encode_query(opcode)).await {
                    tracing::warn!("state re-check {opcode:#04x} failed: {e}");
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
        // BP1 Pro ANC — verified on hardware.
        (DeviceCommand::SetAncMode(AncMode::Off, _), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x34, 0x00, 0xFF]
        }
        (DeviceCommand::SetAncMode(AncMode::Anc, level), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x34, 0x01, *level]
        }
        (DeviceCommand::SetAncMode(AncMode::Transparency, level), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x34, 0x02, *level]
        }
        (DeviceCommand::SetEqPreset(preset), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x43, preset.to_byte()]
        }
        // Game/low-latency mode — verified for BP1 over both SPP and BLE (issue #3).
        (DeviceCommand::SetGameMode(on), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x24, u8::from(*on)]
        }
        (DeviceCommand::FindEarbud(Side::Left), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x10, 0x00, 0x01]
        }
        (DeviceCommand::FindEarbud(Side::Right), BaseusModel::Bp1ProAnc) => {
            vec![0xBA, 0x10, 0x01, 0x01]
        }
        // BP1 Ultra — captured from the official app and replayed over BLE on hardware.
        (DeviceCommand::SetAncMode(mode, level), BaseusModel::Bp1Ultra) => {
            Bp1Ultra::encode_anc(*mode, *level)
        }
        (DeviceCommand::SetEqPreset(preset), BaseusModel::Bp1Ultra) => Bp1Ultra::encode_eq(*preset),
        (DeviceCommand::SetGameMode(on), BaseusModel::Bp1Ultra) => {
            Bp1Ultra::encode_low_latency(*on)
        }
        (DeviceCommand::FindEarbud(_), BaseusModel::Bp1Ultra) => {
            return Err("find-my-earbuds is not supported on the BP1 Ultra yet".into());
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
