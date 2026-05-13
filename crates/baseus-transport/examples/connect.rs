// Manual integration test: connect to an RFCOMM device and print incoming bytes.
// Usage: cargo run -p baseus-transport --example connect -- 0x001122334455
//
// Find your Bluetooth address in Windows Settings → Bluetooth & devices
// → your earbuds → Properties. Convert AA:BB:CC:DD:EE:FF → 0xAABBCCDDEEFF.
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let addr_str = std::env::args().nth(1).expect("pass BT address as 0x<hex>");
    let addr =
        u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).expect("invalid hex address");
    use baseus_transport::{win::rfcomm::RfcommTransport, BluetoothTransport};
    match RfcommTransport::connect(addr).await {
        Ok(mut t) => {
            println!("Connected! Waiting for packets...");
            let mut buf = [0u8; 512];
            for _ in 0..10 {
                match t.recv(&mut buf).await {
                    Ok(n) => println!("RX [{}]: {:02x?}", n, &buf[..n]),
                    Err(e) => {
                        println!("Error: {e}");
                        break;
                    }
                }
            }
        }
        Err(e) => eprintln!("Connection failed: {e}"),
    }
}
