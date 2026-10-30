//! Implementasi tiruan agar tiap kelompok bisa tes tanpa menunggu kelompok lain.
//! Aktifkan dengan feature `mock`:
//! `soil_contracts = { workspace = true, features = ["mock"] }`

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::{ContractError, Result};
use crate::ids::{DeviceId, PlotId};
use crate::ports::{ReadingSink, ReadingSource};
use crate::reading::{QualityFlag, ReadingWindow, SoilReading};

/// Penyimpanan di memori: implementasi tiruan `ReadingSink` + `ReadingSource` (peran M3).
#[derive(Default)]
pub struct InMemoryReadingStore {
    inner: Mutex<Vec<SoilReading>>,
}

impl InMemoryReadingStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl ReadingSink for InMemoryReadingStore {
    async fn ingest(&self, reading: SoilReading) -> Result<()> {
        self.inner
            .lock()
            .map_err(|_| ContractError::Storage("lock rusak".into()))?
            .push(reading);
        Ok(())
    }
}

#[async_trait]
impl ReadingSource for InMemoryReadingStore {
    async fn window(
        &self,
        plot_id: &PlotId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<ReadingWindow> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| ContractError::Storage("lock rusak".into()))?;
        let mut readings: Vec<SoilReading> = guard
            .iter()
            .filter(|r| r.plot_id == *plot_id && r.ts >= from && r.ts <= to)
            .cloned()
            .collect();
        readings.sort_by_key(|r| r.ts);
        Ok(ReadingWindow {
            plot_id: *plot_id,
            from,
            to,
            readings,
        })
    }

    async fn latest(&self, plot_id: &PlotId) -> Result<Option<SoilReading>> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| ContractError::Storage("lock rusak".into()))?;
        Ok(guard
            .iter()
            .filter(|r| r.plot_id == *plot_id)
            .max_by_key(|r| r.ts)
            .cloned())
    }
}

/// Generator data sensor palsu yang deterministik (seed sama = data sama),
/// dengan nilai khas lahan gambut (pH asam, EC rendah).
pub struct FakeSensorGenerator {
    state: u64,
}

impl FakeSensorGenerator {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Angka acak deterministik di [0, 1).
    fn next_unit(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 40) as f32) / (1u64 << 24) as f32
    }

    pub fn reading(
        &mut self,
        device_id: DeviceId,
        plot_id: PlotId,
        ts: DateTime<Utc>,
    ) -> SoilReading {
        let jitter = |u: f32, center: f32, spread: f32| center + (u - 0.5) * 2.0 * spread;
        let (a, b, c, d) = (
            self.next_unit(),
            self.next_unit(),
            self.next_unit(),
            self.next_unit(),
        );
        SoilReading {
            device_id,
            plot_id,
            ts,
            moisture_pct: jitter(a, 60.0, 10.0),
            temperature_c: jitter(b, 27.0, 2.0),
            ph: jitter(c, 4.0, 0.3),
            ec_ms_cm: jitter(d, 0.5, 0.1),
            quality: QualityFlag::Good,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn store_roundtrip_and_window_filter() {
        let store = InMemoryReadingStore::new();
        let (dev, plot) = (DeviceId::new(), PlotId::new());
        let mut gen = FakeSensorGenerator::new(42);
        let t0 = Utc::now();

        for i in 0..10 {
            let r = gen.reading(dev, plot, t0 + Duration::minutes(i));
            store.ingest(r).await.unwrap();
        }

        let w = store
            .window(&plot, t0, t0 + Duration::minutes(4))
            .await
            .unwrap();
        assert_eq!(w.readings.len(), 5);
        assert_eq!(w.series(crate::Parameter::Ph).len(), 5);

        let latest = store.latest(&plot).await.unwrap().unwrap();
        assert_eq!(latest.ts, t0 + Duration::minutes(9));
    }

    #[test]
    fn generator_is_deterministic() {
        let (dev, plot, ts) = (DeviceId::new(), PlotId::new(), Utc::now());
        let a = FakeSensorGenerator::new(7).reading(dev, plot, ts);
        let b = FakeSensorGenerator::new(7).reading(dev, plot, ts);
        assert_eq!(a, b);
    }
}
