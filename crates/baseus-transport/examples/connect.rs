// Manual integration test: connect to a Baseus earbud over BLE GATT and print decoded frames.
//
//   cargo run -p baseus-transport --example connect
//   cargo run -p baseus-transport --example connect -- anc:on anc:off eq:bass game:on
//
// Works on any btleplug backend (Linux/BlueZ, Windows/WinRT, macOS/CoreBluetooth).
// The device is matched by BLE advertising name from the protocol registry, so no
// hard-coded address is needed.
use std::time::Duration;

use baseus_protocol::{framing::Frame, BaseusModel};
use baseus_transport::{win::ble::GattTransport, DeviceMatch};

fn command_bytes(arg: &str) -> Option<Vec<u8>> {
    Some(match arg {
        "anc:off" => vec![0xBA, 0x34, 0x00, 0xFF],
        "anc:on" => vec![0xBA, 0x34, 0x01, 0x68],
        "anc:transparency" => vec![0xBA, 0x34, 0x02, 0xFF],
        "eq:balanced" => vec![0xBA, 0x43, 0x00],
        "eq:bass" => vec![0xBA, 0x43, 0x01],
        "game:on" => vec![0xBA, 0x24, 0x01],
        "game:off" => vec![0xBA, 0x24, 0x00],
        _ => return None,
    })
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
    println!("connected to {}", entries[idx].name);

    transport
        .send(&[0xBA, 0x05, 0x00])
        .await
        .expect("handshake write failed");
    println!("TX handshake: ba 05 00");

    for arg in std::env::args().skip(1) {
        match command_bytes(&arg) {
            Some(bytes) => {
                transport.send(&bytes).await.expect("command write failed");
                println!("TX {arg}: {bytes:02x?}");
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
            None => eprintln!("unknown command '{arg}', skipping"),
        }
    }

    println!("listening for notifications (10s)…");
    let listen = async {
        loop {
            match transport.next_notification().await {
                Ok(data) => match Frame::decode(&data) {
                    Ok(f) => println!("RX cmd={:#04x} payload={:02x?}", f.cmd, f.payload),
                    Err(e) => println!("RX raw={data:02x?} ({e})"),
                },
                Err(e) => {
                    println!("stream ended: {e}");
                    break;
                }
            }
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(10), listen).await;
}
