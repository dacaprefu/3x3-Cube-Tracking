use crate::ble::clock_sync::{AlignedTimestamp, ClockSync};
use crate::ble::crypto::{extract_mac_from_properties, GanCipher};
use crate::ble::error::BleError;
use crate::ble::protocol::{decode_packet, GanEvent};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

// UUIDs característicos del servicio de datos de cubos GAN
pub const GAN_DATA_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000fff0_0000_1000_8000_00805f9b34fb);
pub const GAN_NOTIFY_CHAR_UUID: Uuid = Uuid::from_u128(0x0000fff2_0000_1000_8000_00805f9b34fb);
pub const GAN_WRITE_CHAR_UUID: Uuid = Uuid::from_u128(0x0000fff1_0000_1000_8000_00805f9b34fb);

#[derive(Debug, Clone)]
pub struct DiscoveredGanCube {
    pub name: String,
    pub mac: [u8; 6],
    pub peripheral: Peripheral,
}

/// Escanea en busca de dispositivos GAN durante un tiempo determinado.
pub async fn scan_for_gan_cubes(timeout: Duration) -> Result<Vec<DiscoveredGanCube>, BleError> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().ok_or(BleError::NoAdapterAvailable)?;

    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(timeout).await;

    let peripherals = adapter.peripherals().await?;
    let mut discovered = Vec::new();

    for p in peripherals {
        if let Ok(Some(props)) = p.properties().await {
            let name = props.local_name.clone().unwrap_or_default();
            let is_gan = name.to_uppercase().contains("GAN")
                || name.to_uppercase().contains("MG")
                || name.to_uppercase().contains("AI")
                || props.services.iter().any(|&s| s == GAN_DATA_SERVICE_UUID);

            if is_gan {
                if let Ok(mac) = extract_mac_from_properties(&props) {
                    discovered.push(DiscoveredGanCube {
                        name,
                        mac,
                        peripheral: p,
                    });
                }
            }
        }
    }

    let _ = adapter.stop_scan().await;
    Ok(discovered)
}

pub struct GanBleClient {
    peripheral: Peripheral,
    cipher: GanCipher,
    clock_sync: ClockSync,
}

impl GanBleClient {
    pub async fn connect(cube: DiscoveredGanCube) -> Result<Self, BleError> {
        let p = cube.peripheral;
        p.connect().await.map_err(|e| BleError::ConnectionFailed(e.to_string()))?;
        p.discover_services().await.map_err(|e| BleError::ServiceDiscoveryFailed(e.to_string()))?;

        let cipher = GanCipher::new(cube.mac);
        let clock_sync = ClockSync::new();

        Ok(Self {
            peripheral: p,
            cipher,
            clock_sync,
        })
    }

    /// Inicia el bucle de recepción de eventos BLE en una tarea Tokio independiente
    /// desacoplada de la lógica de procesamiento.
    pub async fn start_event_stream(
        &self,
        event_sender: mpsc::Sender<(GanEvent, AlignedTimestamp)>,
    ) -> Result<(), BleError> {
        let chars = self.peripheral.characteristics();
        let notify_char = chars
            .into_iter()
            .find(|c| c.uuid == GAN_NOTIFY_CHAR_UUID)
            .ok_or_else(|| {
                BleError::ServiceDiscoveryFailed(
                    "Característica de notificación GAN (0xFFF2) no encontrada".to_string(),
                )
            })?;

        self.peripheral.subscribe(&notify_char).await?;

        let mut notifications = self.peripheral.notifications().await?;
        let cipher = self.cipher.clone();
        let mut clock_sync = self.clock_sync.clone();

        tokio::spawn(async move {
            while let Some(data) = notifications.next().await {
                let receipt_instant = Instant::now();
                // Desencriptar payload AES-128
                if let Ok(decrypted) = cipher.decrypt_cbc(&data.value) {
                    if let Ok(event) = decode_packet(&decrypted) {
                        let aligned = clock_sync.synchronize(event.hardware_timestamp_ms(), receipt_instant);
                        if event_sender.send((event, aligned)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
