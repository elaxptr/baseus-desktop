use std::time::Duration;

use btleplug::api::{
    Central, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::{DeviceMatch, TransportError};

const SCAN_TIMEOUT: Duration = Duration::from_secs(20);
const SCAN_POLL: Duration = Duration::from_millis(500);

pub struct GattTransport {
    peripheral: Peripheral,
    write_char: btleplug::api::Characteristic,
    rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    _notif_task: tokio::task::JoinHandle<()>,
}

impl GattTransport {
    /// Connect to a single known device.
    pub async fn connect(device: DeviceMatch<'_>) -> Result<Self, TransportError> {
        Self::connect_any(std::slice::from_ref(&device))
            .await
            .map(|(t, _)| t)
    }

    /// Scan for any of the provided devices and connect to the first one found.
    /// Returns the transport and the index of the matched entry.
    pub async fn connect_any(devices: &[DeviceMatch<'_>]) -> Result<(Self, usize), TransportError> {
        let adapter = get_adapter().await?;

        // Check cached/bonded peripherals first — the device may already be known to the
        // adapter and not actively advertising, so a fresh scan would time out.
        tracing::info!("checking cached peripherals for any known Baseus device…");
        if let Ok(Some((p, idx))) = find_match(&adapter, devices).await {
            tracing::info!("found {} in adapter cache", devices[idx].name);
            let transport = connect_with_uuids(p, devices[idx]).await?;
            return Ok((transport, idx));
        }

        // Scan unfiltered. A ScanFilter with service UUIDs becomes a BlueZ discovery
        // filter, which matches only the advertisement proper — and these earbuds put
        // their service UUID in the scan response, so filtering hides them entirely.
        // Identification happens in `match_index` instead, once the device is visible.
        tracing::info!("starting BLE scan for any known Baseus device…");
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let found = tokio::time::timeout(SCAN_TIMEOUT, poll_for_match(&adapter, devices)).await;
        adapter.stop_scan().await.ok();

        let (p, idx) = found
            .map_err(|_| TransportError::DeviceNotFound("any known Baseus device".to_string()))?
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let transport = connect_with_uuids(p, devices[idx]).await?;
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
    device: DeviceMatch<'_>,
) -> Result<GattTransport, TransportError> {
    let DeviceMatch {
        name: device_name,
        notify_uuid,
        write_uuid,
        ..
    } = device;
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

/// Return the first cached peripheral matching any entry, with the entry's index.
async fn find_match(
    adapter: &Adapter,
    devices: &[DeviceMatch<'_>],
) -> btleplug::Result<Option<(Peripheral, usize)>> {
    for p in adapter.peripherals().await? {
        let Ok(Some(props)) = p.properties().await else {
            continue;
        };
        tracing::debug!(
            "peripheral {} name={:?} services={:?}",
            p.address(),
            props.local_name,
            props.services
        );
        if let Some(idx) = match_index(devices, &props) {
            return Ok(Some((p, idx)));
        }
    }
    Ok(None)
}

async fn poll_for_match(
    adapter: &Adapter,
    devices: &[DeviceMatch<'_>],
) -> btleplug::Result<(Peripheral, usize)> {
    loop {
        if let Some(hit) = find_match(adapter, devices).await? {
            return Ok(hit);
        }
        tokio::time::sleep(SCAN_POLL).await;
    }
}

/// Match a peripheral against the known-device table.
///
/// The advertised service UUID is checked first: it is part of the advertisement
/// proper, so every backend sees it. The name lives in the scan response, which
/// BlueZ surfaces only intermittently, so it is a fallback rather than the key.
fn match_index(devices: &[DeviceMatch<'_>], props: &PeripheralProperties) -> Option<usize> {
    if let Some(idx) = devices.iter().position(|d| {
        Uuid::parse_str(d.service_uuid)
            .is_ok_and(|want| props.services.contains(&want))
    }) {
        return Some(idx);
    }
    let local = props.local_name.as_deref()?;
    devices
        .iter()
        .position(|d| d.name.eq_ignore_ascii_case(local))
}
