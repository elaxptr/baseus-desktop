use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::TransportError;

const SCAN_TIMEOUT: Duration = Duration::from_secs(20);
const SCAN_POLL: Duration = Duration::from_millis(500);

pub struct GattTransport {
    peripheral: Peripheral,
    write_char: btleplug::api::Characteristic,
    rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    _notif_task: tokio::task::JoinHandle<()>,
}

impl GattTransport {
    /// Connect to a specific device by name, using the provided GATT characteristic UUIDs.
    pub async fn connect(
        device_name: &str,
        notify_uuid: &str,
        write_uuid: &str,
    ) -> Result<Self, TransportError> {
        let adapter = get_adapter().await?;

        // Check cached/bonded peripherals first — the device may already be connected via
        // Classic BT and not actively advertising BLE, so a fresh scan would time out.
        tracing::info!("checking cached peripherals for {device_name}…");
        let peripheral = match find_in_cache(&adapter, &[device_name]).await {
            Ok(Some((p, _))) => {
                tracing::info!("found {device_name} in adapter cache (already bonded)");
                p
            }
            _ => {
                tracing::info!("not in cache, starting BLE scan for {device_name}…");
                adapter
                    .start_scan(ScanFilter::default())
                    .await
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

                let (p, _) =
                    tokio::time::timeout(SCAN_TIMEOUT, find_any_by_name(&adapter, &[device_name]))
                        .await
                        .map_err(|_| TransportError::DeviceNotFound(device_name.to_string()))?
                        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

                adapter.stop_scan().await.ok();
                p
            }
        };

        connect_with_uuids(peripheral, notify_uuid, write_uuid, device_name).await
    }

    /// Scan for any of the provided (name, notify_uuid, write_uuid) entries and connect to
    /// the first one found. Returns the transport and the index of the matched entry.
    pub async fn connect_any(
        devices: &[(&str, &str, &str)],
    ) -> Result<(Self, usize), TransportError> {
        let adapter = get_adapter().await?;
        let names: Vec<&str> = devices.iter().map(|(n, _, _)| *n).collect();

        // Cache check first
        tracing::info!("checking cached peripherals for any known Baseus device…");
        if let Ok(Some((p, matched_name))) = find_in_cache(&adapter, &names).await {
            let idx = devices
                .iter()
                .position(|(n, _, _)| *n == matched_name)
                .unwrap();
            let (_, notify, write) = devices[idx];
            tracing::info!("found {matched_name} in adapter cache");
            let transport = connect_with_uuids(p, notify, write, matched_name).await?;
            return Ok((transport, idx));
        }

        // Scan
        tracing::info!("starting BLE scan for any of: {names:?}…");
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let result = tokio::time::timeout(SCAN_TIMEOUT, find_any_by_name(&adapter, &names))
            .await
            .map_err(|_| TransportError::DeviceNotFound("any known Baseus device".to_string()))?
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        adapter.stop_scan().await.ok();

        let (p, matched_name) = result;
        let idx = devices
            .iter()
            .position(|(n, _, _)| *n == matched_name)
            .unwrap();
        let (_, notify, write) = devices[idx];
        let transport = connect_with_uuids(p, notify, write, matched_name).await?;
        Ok((transport, idx))
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.peripheral
            .write(&self.write_char, data, WriteType::WithResponse)
            .await
            .map_err(|e| TransportError::Io(e.to_string()))
    }

    pub async fn next_notification(&mut self) -> Result<Vec<u8>, TransportError> {
        self.rx.recv().await.ok_or(TransportError::Disconnected)
    }

    pub async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }
}

async fn get_adapter() -> Result<Adapter, TransportError> {
    let manager = Manager::new()
        .await
        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| TransportError::ConnectionFailed("no Bluetooth adapter".into()))
}

async fn connect_with_uuids(
    peripheral: Peripheral,
    notify_uuid: &str,
    write_uuid: &str,
    device_name: &str,
) -> Result<GattTransport, TransportError> {
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

    let n_uuid = Uuid::parse_str(notify_uuid).unwrap();
    let w_uuid = Uuid::parse_str(write_uuid).unwrap();

    let notify_char = chars
        .iter()
        .find(|c| c.uuid == n_uuid)
        .ok_or_else(|| {
            tracing::error!("notify characteristic {notify_uuid} not found");
            TransportError::ServiceNotFound
        })?
        .clone();

    let write_char = chars
        .iter()
        .find(|c| c.uuid == w_uuid)
        .ok_or_else(|| {
            tracing::error!("write characteristic {write_uuid} not found");
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
    Ok(GattTransport {
        peripheral,
        write_char,
        rx,
        _notif_task: notif_task,
    })
}

async fn find_in_cache<'a>(
    adapter: &Adapter,
    names: &[&'a str],
) -> btleplug::Result<Option<(Peripheral, &'a str)>> {
    for p in adapter.peripherals().await? {
        if let Ok(Some(props)) = p.properties().await {
            tracing::debug!("cached peripheral: {:?}", props.local_name);
            if let Some(local) = props.local_name.as_deref() {
                if let Some(&matched) = names.iter().find(|&&n| n == local) {
                    return Ok(Some((p, matched)));
                }
            }
        }
    }
    Ok(None)
}

async fn find_any_by_name<'a>(
    adapter: &Adapter,
    names: &[&'a str],
) -> btleplug::Result<(Peripheral, &'a str)> {
    loop {
        for p in adapter.peripherals().await? {
            if let Ok(Some(props)) = p.properties().await {
                tracing::debug!("scan saw: {:?}", props.local_name);
                if let Some(local) = props.local_name.as_deref() {
                    if let Some(&matched) = names.iter().find(|&&n| n == local) {
                        return Ok((p, matched));
                    }
                }
            }
        }
        tokio::time::sleep(SCAN_POLL).await;
    }
}
