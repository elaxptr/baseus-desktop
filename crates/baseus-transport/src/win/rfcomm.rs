use crate::{BluetoothTransport, TransportError};
use windows::{
    Devices::Bluetooth::{BluetoothDevice, Rfcomm::RfcommServiceId},
    Networking::Sockets::StreamSocket,
    Storage::Streams::{DataReader, DataWriter, InputStreamOptions},
};

// Standard SPP service ID (UUID 0x1101).
// If Phase 0 RE reveals a custom RFCOMM service UUID for this earbud model,
// replace SerialPort() with:
//   RfcommServiceId::FromUuid(windows::core::GUID { data1: 0x..., data2: 0x..., data3: 0x..., data4: [...] })
fn spp_service_id() -> windows::core::Result<RfcommServiceId> {
    RfcommServiceId::SerialPort()
}

pub struct RfcommTransport {
    // socket must be kept alive; DataWriter/DataReader hold COM references into its streams.
    socket: StreamSocket,
    reader: DataReader,
    writer: DataWriter,
}

impl BluetoothTransport for RfcommTransport {
    async fn connect(addr: u64) -> Result<Self, TransportError> {
        tokio::task::block_in_place(|| {
            let device = BluetoothDevice::FromBluetoothAddressAsync(addr)
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
                .get()
                .map_err(|_| TransportError::DeviceNotFound(addr))?;

            let service_id = spp_service_id()
                .map_err(|_| TransportError::ServiceNotFound)?;

            let rfcomm_result = device
                .GetRfcommServicesForIdAsync(&service_id)
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
                .get()
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            let services = rfcomm_result
                .Services()
                .map_err(|_| TransportError::ServiceNotFound)?;

            if services.Size().unwrap_or(0) == 0 {
                return Err(TransportError::ServiceNotFound);
            }

            let svc = services.GetAt(0).map_err(|_| TransportError::ServiceNotFound)?;

            let socket = StreamSocket::new()
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            socket
                .ConnectAsync(
                    &svc.ConnectionHostName()
                        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
                    &svc.ConnectionServiceName()
                        .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
                )
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?
                .get()
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            let writer = DataWriter::CreateDataWriter(
                &socket
                    .OutputStream()
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
            )
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            let reader = DataReader::CreateDataReader(
                &socket
                    .InputStream()
                    .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?,
            )
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            reader
                .SetInputStreamOptions(InputStreamOptions::Partial)
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

            tracing::info!("RFCOMM connected to {:#014x}", addr);
            Ok(Self { socket, reader, writer })
        })
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.writer
            .WriteBytes(data)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        let stored = tokio::task::block_in_place(|| {
            self.writer
                .StoreAsync()
                .map_err(|e| TransportError::Io(e.to_string()))?
                .get()
                .map_err(|e| TransportError::Io(e.to_string()))
        })?;
        if stored != data.len() as u32 {
            return Err(TransportError::Io(format!(
                "short write: sent {} of {} bytes",
                stored,
                data.len()
            )));
        }
        Ok(())
    }

    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let loaded = tokio::task::block_in_place(|| {
            self.reader
                .LoadAsync(buf.len() as u32)
                .map_err(|e| TransportError::Io(e.to_string()))?
                .get()
                .map_err(|_| TransportError::Disconnected)
        })? as usize;

        if loaded == 0 {
            return Err(TransportError::Disconnected);
        }

        self.reader
            .ReadBytes(&mut buf[..loaded])
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(loaded)
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        tokio::task::block_in_place(|| {
            self.writer
                .FlushAsync()
                .map_err(|e| TransportError::Io(e.to_string()))?
                .get()
                .map_err(|e| TransportError::Io(e.to_string()))
        })?;
        self.socket.Close().map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }
}
