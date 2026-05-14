#![allow(async_fn_in_trait)]

use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("device '{0}' not found within scan window")]
    DeviceNotFound(String),
    #[error("Bluetooth service or characteristic not found on device")]
    ServiceNotFound,
    #[error("I/O error: {0}")]
    Io(String),
    #[error("disconnected")]
    Disconnected,
}

/// Abstraction over a BLE GATT notification channel.
/// The real implementation is `win::ble::GattTransport` (btleplug-backed).
/// `MockTransport` is available for unit tests.
pub trait BluetoothTransport: Send {
    /// Connect to the named BLE peripheral and subscribe to its notify characteristic.
    async fn connect(device_name: &str) -> Result<Self, TransportError>
    where
        Self: Sized;

    /// Write a raw command frame to the write characteristic.
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Await the next GATT notification from the device.
    async fn next_notification(&mut self) -> Result<Vec<u8>, TransportError>;
}

/// In-process mock for testing. Pre-load `rx_queue` with raw notification bytes.
/// `tx_log` records every outgoing write for assertion.
pub struct MockTransport {
    pub rx_queue: VecDeque<Vec<u8>>,
    pub tx_log: Vec<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            rx_queue: VecDeque::new(),
            tx_log: Vec::new(),
        }
    }

    pub fn push_rx(&mut self, packet: Vec<u8>) {
        self.rx_queue.push_back(packet);
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothTransport for MockTransport {
    async fn connect(_device_name: &str) -> Result<Self, TransportError> {
        Ok(Self::new())
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.tx_log.push(data.to_vec());
        Ok(())
    }

    async fn next_notification(&mut self) -> Result<Vec<u8>, TransportError> {
        self.rx_queue
            .pop_front()
            .ok_or(TransportError::Disconnected)
    }
}

pub mod win;
