use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::TransportError;

// Confirmed via nRF Connect on BASS BP1 PRO (4A:01:CE:BA:C8:03).
const NOTIFY_UUID: &str = "654b749c-e37f-ae1f-ebab-40ca133e3690";
const WRITE_UUID: &str = "ee684b1a-1e9b-ed3e-ee55-f894667e92ac";

const SCAN_TIMEOUT: Duration = Duration::from_secs(20);
const SCAN_POLL: Duration = Duration::from_millis(500);

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

        // Check cached/bonded peripherals first — the device may already be connected via
        // Classic BT and not actively advertising BLE, so a fresh scan would time out.
        tracing::info!("checking cached peripherals for {device_name}…");
        let peripheral = match find_in_cache(&adapter, device_name).await {
            Ok(Some(p)) => {
                tracing::info!("found {device_name} in adapter cache (already bonded)");
                p
            }
            _ => {
                tracing::info!("not in cache, starting BLE scan for {device_name}…");
                adapter
                    .start_scan(ScanFilter::default())
                    .await
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

                let p = tokio::time::timeout(SCAN_TIMEOUT, find_by_name(&adapter, device_name))
                    .await
                    .map_err(|_| TransportError::DeviceNotFound(device_name.to_string()))?
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

                adapter.stop_scan().await.ok();
                p
            }
        };

        tracing::info!("found {device_name}, opening GATT connection…");
        peripheral
            .connect()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("connect(): {e}")))?;

        tracing::info!("connected, discovering services…");
        peripheral
            .discover_services()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("discover_services(): {e}")))?;

        let chars = peripheral.characteristics();
        tracing::debug!(
            "discovered {} characteristics: {:?}",
            chars.len(),
            chars.iter().map(|c| c.uuid.to_string()).collect::<Vec<_>>()
        );

        let notify_uuid = Uuid::parse_str(NOTIFY_UUID).unwrap();
        let write_uuid = Uuid::parse_str(WRITE_UUID).unwrap();

        let notify_char = chars
            .iter()
            .find(|c| c.uuid == notify_uuid)
            .ok_or_else(|| {
                tracing::error!("notify characteristic {NOTIFY_UUID} not found");
                TransportError::ServiceNotFound
            })?
            .clone();

        let write_char = chars
            .iter()
            .find(|c| c.uuid == write_uuid)
            .ok_or_else(|| {
                tracing::error!("write characteristic {WRITE_UUID} not found");
                TransportError::ServiceNotFound
            })?
            .clone();

        tracing::info!("subscribing to notify characteristic…");
        peripheral
            .subscribe(&notify_char)
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("subscribe(): {e}")))?;

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

        tracing::info!("GATT fully connected to {device_name}");
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

async fn find_in_cache(adapter: &Adapter, name: &str) -> btleplug::Result<Option<Peripheral>> {
    for p in adapter.peripherals().await? {
        if let Ok(Some(props)) = p.properties().await {
            tracing::debug!("cached peripheral: {:?}", props.local_name);
            if props.local_name.as_deref() == Some(name) {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}

async fn find_by_name(adapter: &Adapter, name: &str) -> btleplug::Result<Peripheral> {
    loop {
        for p in adapter.peripherals().await? {
            if let Ok(Some(props)) = p.properties().await {
                tracing::debug!("scan saw: {:?}", props.local_name);
                if props.local_name.as_deref() == Some(name) {
                    return Ok(p);
                }
            }
        }
        tokio::time::sleep(SCAN_POLL).await;
    }
}
