use crate::{BluetoothTransport, TransportError};

pub struct RfcommTransport {
    _private: (),
}

impl BluetoothTransport for RfcommTransport {
    async fn connect(_addr: u64) -> Result<Self, TransportError> {
        todo!("WinRT RFCOMM — implemented in Task 9")
    }

    async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
        todo!("WinRT RFCOMM — implemented in Task 9")
    }

    async fn recv(&mut self, _buf: &mut [u8]) -> Result<usize, TransportError> {
        todo!("WinRT RFCOMM — implemented in Task 9")
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        todo!("WinRT RFCOMM — implemented in Task 9")
    }
}
