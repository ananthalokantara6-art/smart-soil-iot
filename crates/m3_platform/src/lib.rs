//! M3 - Soil Data Platform and Visualization.
//!
//! Implementasi awal: penyimpanan di memori untuk pembacaan dan peringatan.
//! TODO kelompok 3: REST API (Axum), database time-series, agregasi, dashboard, filter alert.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use soil_contracts::{
    Alert, AlertSink, ContractError, PlotId, ReadingSink, ReadingSource, ReadingWindow, Result,
    SoilReading,
};

#[derive(Default)]
pub struct MemoryPlatform {
    readings: Mutex<Vec<SoilReading>>,
    alerts: Mutex<Vec<Alert>>,
}

fn poisoned() -> ContractError {
    ContractError::Storage("lock rusak".into())
}

impl MemoryPlatform {
    pub fn new() -> Self {
        Self::default()
    }

    /// Riwayat peringatan satu petak, terbaru terakhir.
    pub fn alerts_for(&self, plot_id: &PlotId) -> Result<Vec<Alert>> {
        let guard = self.alerts.lock().map_err(|_| poisoned())?;
        Ok(guard
            .iter()
            .filter(|a| a.plot_id == *plot_id)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl ReadingSink for MemoryPlatform {
    async fn ingest(&self, reading: SoilReading) -> Result<()> {
        self.readings.lock().map_err(|_| poisoned())?.push(reading);
        Ok(())
    }
}

#[async_trait]
impl ReadingSource for MemoryPlatform {
    async fn window(
        &self,
        plot_id: &PlotId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<ReadingWindow> {
        let guard = self.readings.lock().map_err(|_| poisoned())?;
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
        let guard = self.readings.lock().map_err(|_| poisoned())?;
        Ok(guard
            .iter()
            .filter(|r| r.plot_id == *plot_id)
            .max_by_key(|r| r.ts)
            .cloned())
    }
}

#[async_trait]
impl AlertSink for MemoryPlatform {
    async fn raise(&self, alert: Alert) -> Result<()> {
        self.alerts.lock().map_err(|_| poisoned())?.push(alert);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use soil_contracts::{AlertLevel, DeviceId, Provenance, QualityFlag};

    fn reading(plot: PlotId, ts: DateTime<Utc>) -> SoilReading {
        SoilReading {
            device_id: DeviceId::new(),
            plot_id: plot,
            ts,
            moisture_pct: 60.0,
            temperature_c: 27.0,
            ph: 4.0,
            ec_ms_cm: 0.5,
            quality: QualityFlag::Good,
        }
    }

    #[tokio::test]
    async fn ingest_then_query_window() {
        let p = MemoryPlatform::new();
        let plot = PlotId::new();
        let t0 = Utc::now();
        for i in 0..6 {
            p.ingest(reading(plot, t0 + Duration::minutes(i)))
                .await
                .unwrap();
        }
        p.ingest(reading(PlotId::new(), t0)).await.unwrap();

        let w = p
            .window(&plot, t0, t0 + Duration::minutes(2))
            .await
            .unwrap();
        assert_eq!(w.readings.len(), 3);
        let latest = p.latest(&plot).await.unwrap().unwrap();
        assert_eq!(latest.ts, t0 + Duration::minutes(5));
    }

    #[tokio::test]
    async fn alerts_are_stored() {
        let p = MemoryPlatform::new();
        let plot = PlotId::new();
        p.raise(Alert {
            plot_id: plot,
            ts: Utc::now(),
            level: AlertLevel::Warning,
            message: "uji".into(),
            provenance: Provenance {
                source: "test".into(),
                version: "0".into(),
                method: "manual".into(),
                inputs: vec![],
                generated_at: Utc::now(),
            },
        })
        .await
        .unwrap();
        assert_eq!(p.alerts_for(&plot).unwrap().len(), 1);
        assert!(p.alerts_for(&PlotId::new()).unwrap().is_empty());
    }
}
