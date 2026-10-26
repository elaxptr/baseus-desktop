// Manual integration test: connect to a Baseus earbud over BLE GATT and print decoded frames.
//
//   cargo run -p baseus-transport --example connect
//   cargo run -p baseus-transport --example connect -- anc:on anc:off eq:bass game:on
//
// Works on any btleplug backend (Linux/BlueZ, Windows/WinRT, macOS/CoreBluetooth).
// The device is matched by service UUID / advertising name from the protocol registry,
// so no hard-coded address is needed. Models sharing a GATT service (BP1 Pro and
// BP1 Ultra) are told apart by the envelope of the first notification, and commands
// are encoded for whichever model answered.
use std::time::Duration;

use baseus_protocol::{
    models::{
        bp1_ultra::{self, Bp1Ultra},
        Bp1ProAnc,
    },
    types::EqPreset,
    AncMode, BaseusModel, DeviceEvent, WireFormat,
};
use baseus_transport::{win::ble::GattTransport, DeviceMatch};

fn command_bytes(model: BaseusModel, arg: &str) -> Option<Vec<u8>> {
    match model {
        BaseusModel::Bp1ProAnc => Some(match arg {
            "anc:off" => vec![0xBA, 0x34, 0x00, 0xFF],
            "anc:on" => vec![0xBA, 0x34, 0x01, 0x68],
            "anc:transparency" => vec![0xBA, 0x34, 0x02, 0xFF],
            "eq:balanced" => vec![0xBA, 0x43, 0x00],
            "eq:bass" => vec![0xBA, 0x43, 0x01],
            "game:on" => vec![0xBA, 0x24, 0x01],
            "game:off" => vec![0xBA, 0x24, 0x00],
            _ => return None,
        }),
        BaseusModel::Bp1Ultra => Some(match arg {
            "anc:off" => Bp1Ultra::encode_anc(AncMode::Off, 0),
            "anc:on" => Bp1Ultra::encode_anc(AncMode::Anc, bp1_ultra::ANC_LEVEL_MAX),
            "anc:transparency" => Bp1Ultra::encode_anc(AncMode::Transparency, 0),
            "anc:?" => Bp1Ultra::encode_query(bp1_ultra::op::ANC_STATE),
            "eq:balanced" => Bp1Ultra::encode_eq(EqPreset::Balanced),
            "eq:bass" => Bp1Ultra::encode_eq(EqPreset::BassBoost),
            "eq:voice" => Bp1Ultra::encode_eq(EqPreset::Voice),
            "eq:clear" => Bp1Ultra::encode_eq(EqPreset::Clear),
            "game:on" => Bp1Ultra::encode_low_latency(true),
            "game:off" => Bp1Ultra::encode_low_latency(false),
            "spatial:on" => Bp1Ultra::encode_spatial_audio(true),
            "spatial:off" => Bp1Ultra::encode_spatial_audio(false),
            "wind:on" => Bp1Ultra::encode_wind_reduction(true),
            "wind:off" => Bp1Ultra::encode_wind_reduction(false),
            "bass:off" => Bp1Ultra::encode_bass_boost(false, 2),
            "bass:1" => Bp1Ultra::encode_bass_boost(true, 1),
            "bass:2" => Bp1Ultra::encode_bass_boost(true, 2),
            "bass:3" => Bp1Ultra::encode_bass_boost(true, 3),
            "fw:?" => Bp1Ultra::encode_query(bp1_ultra::op::FIRMWARE),
            _ => return None,
        }),
    }
}

fn describe(model: BaseusModel, data: &[u8]) -> String {
    match model.wire_format().decode(data) {
        Ok(frame) => {
            let event: Option<DeviceEvent> = match model {
                BaseusModel::Bp1ProAnc => Bp1ProAnc::decode_frame(&frame).ok(),
                BaseusModel::Bp1Ultra => Bp1Ultra::decode_frame(&frame).ok(),
            };
            match event {
                Some(ev) => format!("cmd={:#04x} {ev:?}", frame.cmd),
                None => format!("cmd={:#04x} payload={:02x?}", frame.cmd, frame.payload),
            }
        }
        Err(e) => format!("raw={data:02x?} ({e})"),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let entries: Vec<DeviceMatch> = BaseusModel::all()
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

    let (mut transport, idx) = match GattTransport::connect_any(&entries).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("connection failed: {e}");
            std::process::exit(1);
        }
    };
    println!("connected via registry entry '{}'", entries[idx].name);

    transport
        .send(&[0xBA, 0x05, 0x00])
        .await
        .expect("handshake write failed");
    println!("TX handshake: ba 05 00");

    // The first notification (a battery report on both BP1 models) identifies the model.
    let first = tokio::time::timeout(Duration::from_secs(3), transport.next_notification())
        .await
        .ok()
        .and_then(Result::ok);
    let model = first
        .as_deref()
        .and_then(WireFormat::detect)
        .map(BaseusModel::from_wire_format)
        .unwrap_or(BaseusModel::Bp1ProAnc);
    println!("model: {}", model.display_name());
    if let Some(data) = &first {
        println!("RX {}", describe(model, data));
    }

    for arg in std::env::args().skip(1) {
        match command_bytes(model, &arg) {
            Some(bytes) => {
                transport.send(&bytes).await.expect("command write failed");
                println!("TX {arg}: {bytes:02x?}");
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
            None => eprintln!("unknown command '{arg}' for this model, skipping"),
        }
    }

    println!("listening for notifications (10s)…");
    let listen = async {
        loop {
            match transport.next_notification().await {
                Ok(data) => println!("RX {}", describe(model, &data)),
                Err(e) => {
                    println!("stream ended: {e}");
                    break;
                }
            }
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(10), listen).await;
}
