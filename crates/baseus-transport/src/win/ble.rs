use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::TransportError;

// Confirmed via nRF Connect on BASS BP1 PRO (4A:01:CE:BA:C8:03).
const NOTIFY_UUID: &str = "654b749c-e37f-ae1f-ebab-40ca133e3690";
const WRITE_UUID: &str = "ee684b1a-1e9b-ed3e-ee55-f894667e92ac";

const SCAN_TIMEOUT: Duration = Duration::from_secs(15);
const SCAN_POLL: Duration = Duration::from_millis(300);

pub struct GattTransport {
    peripheral: Peripheral,
    write_char: btleplug::api::Characteristic,
    rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    _notif_task: tokio::task::JoinHandle<()>,
}

impl GattTransport {
    pub async fn connect(device_name: &str) -> Result<Self, TransportError> {
        let manager = Manager::new()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| TransportError::ConnectionFailed("no Bluetooth adapter".into()))?;

        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let peripheral = tokio::time::timeout(
            SCAN_TIMEOUT,
            find_by_name(&adapter, device_name),
        )
        .await
        .map_err(|_| TransportError::DeviceNotFound(device_name.to_string()))?
        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        adapter.stop_scan().await.ok();
        tracing::info!("found {device_name}, connecting…");

        peripheral
            .connect()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        peripheral
            .discover_services()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let chars = peripheral.characteristics();
        let notify_uuid = Uuid::parse_str(NOTIFY_UUID).unwrap();
        let write_uuid = Uuid::parse_str(WRITE_UUID).unwrap();

        let notify_char = chars
            .iter()
            .find(|c| c.uuid == notify_uuid)
            .ok_or(TransportError::ServiceNotFound)?
            .clone();

        let write_char = chars
            .iter()
            .find(|c| c.uuid == write_uuid)
            .ok_or(TransportError::ServiceNotFound)?
            .clone();

        peripheral
            .subscribe(&notify_char)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let mut notif_stream = peripheral
            .notifications()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let notif_task = tokio::spawn(async move {
            while let Some(n) = notif_stream.next().await {
                if tx.send(n.value).is_err() {
                    break;
                }
            }
        });

        tracing::info!("GATT connected to {device_name}");
        Ok(Self {
            peripheral,
            write_char,
            rx,
            _notif_task: notif_task,
        })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.peripheral
            .write(&self.write_char, data, WriteType::WithoutResponse)
            .await
            .map_err(|e| TransportError::Io(e.to_string()))
    }

    pub async fn next_notification(&mut self) -> Result<Vec<u8>, TransportError> {
        self.rx.recv().await.ok_or(TransportError::Disconnected)
    }
}

async fn find_by_name(
    adapter: &Adapter,
    name: &str,
) -> btleplug::Result<Peripheral> {
    loop {
        for p in adapter.peripherals().await? {
            if let Ok(Some(props)) = p.properties().await {
                if props.local_name.as_deref() == Some(name) {
                    return Ok(p);
                }
            }
        }
        tokio::time::sleep(SCAN_POLL).await;
    }
}
