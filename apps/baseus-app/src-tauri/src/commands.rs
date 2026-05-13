use baseus_protocol::types::DeviceEvent;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrontendEvent {
    BatteryUpdate {
        left_pct: u8,
        right_pct: u8,
        case_pct: u8,
        left_charging: bool,
        right_charging: bool,
        case_charging: bool,
    },
    Connected,
    Disconnected,
}

fn device_event_to_frontend(e: DeviceEvent) -> Option<FrontendEvent> {
    match e {
        DeviceEvent::BatteryUpdate(b) => Some(FrontendEvent::BatteryUpdate {
            left_pct: b.left_pct,
            right_pct: b.right_pct,
            case_pct: b.case_pct,
            left_charging: b.left_charging,
            right_charging: b.right_charging,
            case_charging: b.case_charging,
        }),
        DeviceEvent::Connected => Some(FrontendEvent::Connected),
        DeviceEvent::Disconnected => Some(FrontendEvent::Disconnected),
        DeviceEvent::AncModeUpdate(_) => None, // ANC not in v1 UI
    }
}

#[tauri::command]
pub async fn connect(app: AppHandle, addr: u64) -> Result<(), String> {
    use crate::device::Device;
    use baseus_protocol::types::BaseusModel;
    use baseus_transport::win::rfcomm::RfcommTransport;
    use baseus_transport::BluetoothTransport;

    let transport = RfcommTransport::connect(addr)
        .await
        .map_err(|e| e.to_string())?;
    let (device, mut rx) = Device::new(transport, BaseusModel::Bp1ProAnc);

    tokio::spawn(device.run());

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(fe) = device_event_to_frontend(event) {
                        let _ = app.emit("device-event", &fe);
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("event channel lagged by {n}");
                }
            }
        }
    });

    Ok(())
}
