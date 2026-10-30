//! M2 - IoT Communication, Edge Processing, and User Management.
//!
//! Implementasi awal: direktori perangkat statis dan `Gateway` yang memvalidasi pembacaan
//! lalu meneruskannya ke `ReadingSink`.
//! TODO kelompok 2: MQTT/HTTP, buffering offline, provisioning, autentikasi dan RBAC.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use soil_contracts::{
    ContractError, DeviceDirectory, DeviceId, PlotId, RawReading, ReadingSink, ReadingValidator,
    Result, SoilReading, UserId,
};

/// Direktori perangkat di memori: perangkat -> (pemilik, petak).
#[derive(Debug, Clone, Default)]
pub struct StaticDirectory {
    devices: HashMap<DeviceId, (UserId, PlotId)>,
}

impl StaticDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, device: DeviceId, owner: UserId, plot: PlotId) {
        self.devices.insert(device, (owner, plot));
    }
}

#[async_trait]
impl DeviceDirectory for StaticDirectory {
    async fn owner_of(&self, device: &DeviceId) -> Result<Option<UserId>> {
        Ok(self.devices.get(device).map(|(owner, _)| *owner))
    }

    async fn plot_of(&self, device: &DeviceId) -> Result<Option<PlotId>> {
        Ok(self.devices.get(device).map(|(_, plot)| *plot))
    }
}

/// Pintu masuk data: identifikasi perangkat -> validasi (M1) -> simpan (M3).
pub struct Gateway {
    validator: Arc<dyn ReadingValidator>,
    directory: Arc<dyn DeviceDirectory>,
    sink: Arc<dyn ReadingSink>,
}

impl Gateway {
    pub fn new(
        validator: Arc<dyn ReadingValidator>,
        directory: Arc<dyn DeviceDirectory>,
        sink: Arc<dyn ReadingSink>,
    ) -> Self {
        Self {
            validator,
            directory,
            sink,
        }
    }

    /// Menangani satu pembacaan mentah dan mengembalikan versi tervalidasi yang disimpan.
    pub async fn handle(&self, raw: RawReading) -> Result<SoilReading> {
        let plot = self
            .directory
            .plot_of(&raw.device_id)
            .await?
            .ok_or(ContractError::Unauthorized)?;
        let reading = self.validator.validate(&raw, plot)?;
        self.sink.ingest(reading.clone()).await?;
        Ok(reading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use soil_contracts::mock::InMemoryReadingStore;
    use soil_contracts::QualityFlag;

    struct PassThrough;

    impl ReadingValidator for PassThrough {
        fn validate(&self, raw: &RawReading, plot_id: PlotId) -> Result<SoilReading> {
            Ok(SoilReading {
                device_id: raw.device_id,
                plot_id,
                ts: raw.ts,
                moisture_pct: raw.moisture_raw,
                temperature_c: raw.temperature_raw,
                ph: raw.ph_raw,
                ec_ms_cm: raw.ec_raw,
                quality: QualityFlag::Good,
            })
        }
    }

    fn raw(device: DeviceId) -> RawReading {
        RawReading {
            device_id: device,
            ts: Utc::now(),
            moisture_raw: 60.0,
            temperature_raw: 27.0,
            ph_raw: 4.0,
            ec_raw: 0.5,
        }
    }

    #[tokio::test]
    async fn known_device_is_stored() {
        let (device, plot) = (DeviceId::new(), PlotId::new());
        let mut dir = StaticDirectory::new();
        dir.register(device, UserId::new(), plot);
        let store = Arc::new(InMemoryReadingStore::new());
        let gw = Gateway::new(Arc::new(PassThrough), Arc::new(dir), store.clone());

        let stored = gw.handle(raw(device)).await.unwrap();
        assert_eq!(stored.plot_id, plot);
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn unknown_device_is_rejected() {
        let store = Arc::new(InMemoryReadingStore::new());
        let gw = Gateway::new(
            Arc::new(PassThrough),
            Arc::new(StaticDirectory::new()),
            store.clone(),
        );

        let err = gw.handle(raw(DeviceId::new())).await.unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        assert!(store.is_empty());
    }
}
