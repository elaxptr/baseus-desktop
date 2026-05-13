use baseus_protocol::{
    framing::Frame,
    models::{bp1_pro_anc::Bp1ProAnc, DecodeError},
    types::{BaseusModel, DeviceEvent},
};
use baseus_transport::{BluetoothTransport, TransportError};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

const EVENT_CHANNEL_CAP: usize = 64;

// Baseus frames are small (empirically < 64 bytes); 1 KiB is a safe upper bound.
// If decode returns framing errors after a large recv, increase this value.
const RECV_BUF_LEN: usize = 1024;

pub struct Device<T: BluetoothTransport> {
    transport: T,
    model:     BaseusModel,
    event_tx:  broadcast::Sender<DeviceEvent>,
}

impl<T: BluetoothTransport> Device<T> {
    pub fn new(transport: T, model: BaseusModel) -> (Self, broadcast::Receiver<DeviceEvent>) {
        let (tx, rx) = broadcast::channel(EVENT_CHANNEL_CAP);
        (Self { transport, model, event_tx: tx }, rx)
    }

    pub async fn run(mut self) {
        info!("device event loop started");
        let mut buf = vec![0u8; RECV_BUF_LEN];
        loop {
            match self.transport.recv(&mut buf).await {
                Ok(n) => {
                    match Frame::decode(&buf[..n]) {
                        Ok(frame) => {
                            let result = match self.model {
                                BaseusModel::Bp1ProAnc => Bp1ProAnc::decode_frame(&frame),
                            };
                            match result {
                                Ok(event) => { let _ = self.event_tx.send(event); }
                                Err(DecodeError::UnknownOpcode(op)) => {
                                    warn!("unknown opcode {op:#04x} — ignoring");
                                }
                                Err(e) => error!("decode error: {e}"),
                            }
                        }
                        Err(e) => warn!("framing error: {e}"),
                    }
                }
                Err(TransportError::Disconnected) => {
                    info!("device disconnected");
                    let _ = self.event_tx.send(DeviceEvent::Disconnected);
                    break;
                }
                Err(e) => {
                    error!("transport error: {e}");
                    let _ = self.event_tx.send(DeviceEvent::Disconnected);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baseus_protocol::{framing::Frame, types::{BatteryState, DeviceEvent}};
    use baseus_transport::MockTransport;

    fn make_battery_packet(l: u8, r: u8, c: u8) -> Vec<u8> {
        // cmd=0x01 is a placeholder — replace with real OPCODE_BATTERY after Phase 0/Task 7.
        Frame { cmd: 0x01, payload: vec![l, r, c, 0x00] }.encode()
    }

    #[tokio::test]
    #[ignore = "blocked on Phase 0 — Bp1ProAnc::decode_frame is todo!() until captures are analysed"]
    async fn battery_event_forwarded_to_subscriber() {
        let mut mock = MockTransport::new();
        mock.push_rx(make_battery_packet(80, 75, 60));

        let (device, mut rx) = Device::new(mock, baseus_protocol::types::BaseusModel::Bp1ProAnc);
        tokio::spawn(device.run());

        let event = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            async {
                loop {
                    match rx.recv().await {
                        Ok(e) => return e,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            panic!("channel closed before event arrived")
                        }
                    }
                }
            }
        ).await.expect("event within 1s");

        assert!(matches!(
            event,
            DeviceEvent::BatteryUpdate(BatteryState { left_pct: 80, right_pct: 75, case_pct: 60, .. })
        ));
    }

    #[tokio::test]
    async fn disconnect_event_emitted_on_transport_error() {
        let mock = MockTransport::new(); // empty queue → recv returns Disconnected immediately
        let (device, mut rx) = Device::new(mock, baseus_protocol::types::BaseusModel::Bp1ProAnc);
        tokio::spawn(device.run());

        let event = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            async {
                loop {
                    match rx.recv().await {
                        Ok(e) => return e,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            panic!("channel closed before event arrived")
                        }
                    }
                }
            }
        ).await.expect("disconnect event within 1s");

        assert!(matches!(event, DeviceEvent::Disconnected));
    }
}
