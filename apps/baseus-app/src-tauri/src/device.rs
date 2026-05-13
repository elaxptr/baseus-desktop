use std::time::Duration;

use baseus_protocol::{framing::Frame, models::bp1_pro_anc::Bp1ProAnc};
use baseus_transport::win::ble::GattTransport;
use tauri::{AppHandle, Emitter};

const DEVICE_NAME: &str = "BASS BP1 PRO";
const RETRY_DELAY: Duration = Duration::from_secs(5);

pub async fn run_loop(app: AppHandle) {
    loop {
        let _ = app.emit("connection-state", "connecting");
        match GattTransport::connect(DEVICE_NAME).await {
            Ok(mut transport) => {
                let _ = app.emit("connection-state", "connected");
                notification_loop(&app, &mut transport).await;
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

async fn notification_loop(app: &AppHandle, transport: &mut GattTransport) {
    loop {
        match transport.next_notification().await {
            Ok(data) => {
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
            Err(e) => {
                tracing::warn!("transport error: {e}");
                return;
            }
        }
    }
}
