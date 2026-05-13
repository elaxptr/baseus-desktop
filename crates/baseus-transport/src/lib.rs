use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("device not found for address {0:#014x}")]
    DeviceNotFound(u64),
    #[error("Bluetooth service not found on device")]
    ServiceNotFound,
    #[error("I/O error: {0}")]
    Io(String),
    #[error("disconnected")]
    Disconnected,
}

/// Abstraction over a bidirectional Bluetooth byte stream.
/// The Windows (WinRT) implementation is in `win::rfcomm`.
/// `MockTransport` is available for unit-testing the device event loop.
pub trait BluetoothTransport: Send + 'static {
    async fn connect(addr: u64) -> Result<Self, TransportError>
    where
        Self: Sized;
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
    /// Read the next packet. Returns the number of bytes written into `buf`.
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
}

/// In-process mock for testing. Pre-load `recv_queue` with raw packet bytes.
/// `send_log` records every outgoing packet for assertion.
pub struct MockTransport {
    pub recv_queue: VecDeque<Vec<u8>>,
    pub send_log:   Vec<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self { recv_queue: VecDeque::new(), send_log: Vec::new() }
    }

    pub fn push_rx(&mut self, packet: Vec<u8>) {
        self.recv_queue.push_back(packet);
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothTransport for MockTransport {
    async fn connect(_addr: u64) -> Result<Self, TransportError> {
        Ok(Self::new())
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.send_log.push(data.to_vec());
        Ok(())
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        if let Some(packet) = self.recv_queue.pop_front() {
            let n = packet.len().min(buf.len());
            buf[..n].copy_from_slice(&packet[..n]);
            Ok(n)
        } else {
            Err(TransportError::Disconnected)
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}

#[cfg(windows)]
pub mod win;
