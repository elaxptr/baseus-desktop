use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Duration;

use btleplug::api::{
    BDAddr, Central, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::{DeviceMatch, TransportError};

const SCAN_TIMEOUT: Duration = Duration::from_secs(20);
const SCAN_POLL: Duration = Duration::from_millis(500);

/// Devices found to lack the control service, kept for the life of the process so later
/// attempts don't connect them again (each try pages the earbuds' classic audio device).
static NOT_CONTROL: Mutex<BTreeSet<BDAddr>> = Mutex::new(BTreeSet::new());

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
        // Names of non-matching devices seen along the way, reported on failure.
        let mut nearby = BTreeSet::new();
        // Matching devices that turned out not to be the control device.
        let mut rejected = NOT_CONTROL.lock().map(|s| s.clone()).unwrap_or_default();

        // Check cached/bonded peripherals first — the device may already be known to the
        // adapter and not actively advertising, so a fresh scan would time out.
        tracing::info!("checking cached peripherals for any known Baseus device…");
        let cached = find_matches(&adapter, devices, &mut nearby, &rejected)
            .await
            .unwrap_or_default();
        for (p, idx, by_service) in cached {
            tracing::info!("found {} in adapter cache", devices[idx].name);
            if let Some(transport) =
                try_candidate(p, devices[idx], by_service, &mut rejected).await?
            {
                return Ok((transport, idx));
            }
        }

        // Scan unfiltered. A ScanFilter with service UUIDs becomes a BlueZ discovery
        // filter, which matches only the advertisement proper — and these earbuds put
        // their service UUID in the scan response, so filtering hides them entirely.
        // Identification happens in `match_index` instead, once the device is visible.
        tracing::info!("starting BLE scan for any known Baseus device…");
        // A scan that cannot start almost always means the radio is off or missing.
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| TransportError::AdapterUnavailable(e.to_string()))?;

        let found = tokio::time::timeout(SCAN_TIMEOUT, async {
            loop {
                let candidates = find_matches(&adapter, devices, &mut nearby, &rejected)
                    .await
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
                for (p, idx, by_service) in candidates {
                    adapter.stop_scan().await.ok();
                    if let Some(transport) =
                        try_candidate(p, devices[idx], by_service, &mut rejected).await?
                    {
                        return Ok((transport, idx));
                    }
                    adapter.start_scan(ScanFilter::default()).await.ok();
                }
                tokio::time::sleep(SCAN_POLL).await;
            }
        })
        .await;
        adapter.stop_scan().await.ok();

        found.map_err(|_| TransportError::DeviceNotFound {
            nearby: nearby.into_iter().collect(),
        })?
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
        .map_err(|e| TransportError::AdapterUnavailable(e.to_string()))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|e| TransportError::AdapterUnavailable(e.to_string()))?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| TransportError::AdapterUnavailable("no Bluetooth adapter".into()))
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
    #[cfg(target_os = "linux")]
    let was_connected = peripheral.is_connected().await.unwrap_or(false);
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

    // BlueZ keeps the LE link up after the process that opened it exits, and the earbuds
    // send nothing on a link they already had: no battery report, so no model detection
    // either. Start a fresh link, now that this is known to be the control device and
    // not the classic audio one.
    #[cfg(target_os = "linux")]
    if was_connected {
        tracing::info!("link left open by an earlier session, reconnecting…");
        peripheral
            .disconnect()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("disconnect(): {e}")))?;
        peripheral
            .connect()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("connect(): {e}")))?;
        peripheral
            .discover_services()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("discover_services(): {e}")))?;
    }

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

/// Connect to one matching peripheral. `Ok(None)` means move on to the next match: a
/// name match can be the earbuds' classic audio device, which BlueZ lists beside the LE
/// one under the same name. It has no control service, or can't be paged at all when out
/// of range, so any failure of a name-only match skips it instead of ending the search.
/// Skipped devices are remembered in `rejected`, and left as they are: they carry the audio.
async fn try_candidate(
    peripheral: Peripheral,
    device: DeviceMatch<'_>,
    by_service: bool,
    rejected: &mut BTreeSet<BDAddr>,
) -> Result<Option<GattTransport>, TransportError> {
    let address = peripheral.address();
    match connect_with_uuids(peripheral, device).await {
        Ok(transport) => Ok(Some(transport)),
        Err(e) if !by_service || matches!(e, TransportError::ServiceNotFound) => {
            tracing::info!("{address} is not the control device ({e}), trying the next match");
            if matches!(e, TransportError::ServiceNotFound) {
                if let Ok(mut known) = NOT_CONTROL.lock() {
                    known.insert(address);
                }
            }
            rejected.insert(address);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Cached peripherals matching any entry, with the entry's index and whether the service
/// UUID matched: those advertising the control service first, then name-only matches. Names of the peripherals that did not
/// match are added to `nearby`.
async fn find_matches(
    adapter: &Adapter,
    devices: &[DeviceMatch<'_>],
    nearby: &mut BTreeSet<String>,
    rejected: &BTreeSet<BDAddr>,
) -> btleplug::Result<Vec<(Peripheral, usize, bool)>> {
    let mut by_service = Vec::new();
    let mut by_name = Vec::new();
    for p in adapter.peripherals().await? {
        if rejected.contains(&p.address()) {
            continue;
        }
        let Ok(Some(props)) = p.properties().await else {
            continue;
        };
        tracing::debug!(
            "peripheral {} name={:?} services={:?}",
            p.address(),
            props.local_name,
            props.services
        );
        match match_index(devices, &props) {
            Some((idx, true)) => by_service.push((p, idx, true)),
            Some((idx, false)) => by_name.push((p, idx, false)),
            None => {
                if let Some(name) = props.local_name.map(|n| n.trim().to_string()) {
                    if !name.is_empty() {
                        nearby.insert(name);
                    }
                }
            }
        }
    }
    by_service.extend(by_name);
    Ok(by_service)
}

/// Match a peripheral against the known-device table.
///
/// The advertised service UUID is checked first: it is part of the advertisement
/// proper, so every backend sees it. The name lives in the scan response, which
/// BlueZ surfaces only intermittently, so it is a fallback rather than the key.
/// Returns the entry's index and whether the service UUID (rather than the name) matched.
fn match_index(devices: &[DeviceMatch<'_>], props: &PeripheralProperties) -> Option<(usize, bool)> {
    if let Some(idx) = devices.iter().position(|d| {
        Uuid::parse_str(d.service_uuid).is_ok_and(|want| props.services.contains(&want))
    }) {
        return Some((idx, true));
    }
    let local = props.local_name.as_deref()?.trim();
    devices
        .iter()
        .position(|d| d.name.eq_ignore_ascii_case(local))
        .map(|idx| (idx, false))
}
