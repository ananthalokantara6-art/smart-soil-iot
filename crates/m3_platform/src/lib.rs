//! M3 - Soil Data Platform and Visualization.
//!
//! Penyimpanan di memori untuk pembacaan, peringatan, dan metadata petak (Progress 2),
//! ditambah REST API (Axum), agregasi time-series, filter alert, dan dashboard.
//! TODO kelompok 3: database time-series sebagai pengganti `MemoryPlatform`.

pub mod aggregate;
pub mod alerts;
pub mod api;
pub mod overview;
pub mod plots;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use aggregate::{validate_integrity, validate_reading};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use plots::PlotMeta;
use soil_contracts::{
    Alert, AlertSink, ContractError, DeviceId, PlotId, ReadingSink, ReadingSource, ReadingWindow,
    Result, SoilReading,
};

pub const DEFAULT_MAX_READINGS: usize = 100_000;
pub const DEFAULT_MAX_ALERTS: usize = 5_000;
const MAX_ALERT_MESSAGE_CHARS: usize = 500;

#[derive(Default)]
struct ReadingStore {
    readings: VecDeque<SoilReading>,
    seen: HashSet<(DeviceId, i64)>,
}

pub struct MemoryPlatform {
    store: Mutex<ReadingStore>,
    alerts: Mutex<VecDeque<Alert>>,
    plots: Mutex<HashMap<PlotId, PlotMeta>>,
    max_readings: usize,
    max_alerts: usize,
}

impl Default for MemoryPlatform {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> ContractError {
    ContractError::Storage("lock rusak".into())
}

impl MemoryPlatform {
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_READINGS, DEFAULT_MAX_ALERTS)
    }

    pub fn with_limits(max_readings: usize, max_alerts: usize) -> Self {
        Self {
            store: Mutex::default(),
            alerts: Mutex::default(),
            plots: Mutex::default(),
            max_readings: max_readings.max(1),
            max_alerts: max_alerts.max(1),
        }
    }

    pub fn ingest_checked(&self, reading: SoilReading) -> Result<bool> {
        validate_reading(&reading)?;
        validate_integrity(&reading, Utc::now())?;

        let key = (reading.device_id, reading.ts.timestamp_millis());
        let mut store = self.store.lock().map_err(|_| poisoned())?;

        if !store.seen.insert(key) {
            return Ok(false);
        }

        store.readings.push_back(reading);
        while store.readings.len() > self.max_readings {
            let Some(old) = store.readings.pop_front() else {
                break;
            };
            store
                .seen
                .remove(&(old.device_id, old.ts.timestamp_millis()));
        }
        Ok(true)
    }

    pub fn register_plot(&self, meta: PlotMeta) -> Result<()> {
        self.plots
            .lock()
            .map_err(|_| poisoned())?
            .insert(meta.plot_id, meta);
        Ok(())
    }

    pub fn list_plots(&self) -> Result<Vec<PlotMeta>> {
        let guard = self.plots.lock().map_err(|_| poisoned())?;
        let mut all: Vec<PlotMeta> = guard.values().cloned().collect();
        all.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(all)
    }

    pub fn latest_usable(&self, plot_id: &PlotId) -> Result<Option<SoilReading>> {
        let guard = self.store.lock().map_err(|_| poisoned())?;
        Ok(guard
            .readings
            .iter()
            .filter(|r| r.plot_id == *plot_id && r.is_usable())
            .max_by_key(|r| r.ts)
            .cloned())
    }

    pub fn all_alerts(&self) -> Result<Vec<Alert>> {
        Ok(self
            .alerts
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .cloned()
            .collect())
    }

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
        self.ingest_checked(reading)?;
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
        let guard = self.store.lock().map_err(|_| poisoned())?;
        let mut readings: Vec<SoilReading> = guard
            .readings
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
        let guard = self.store.lock().map_err(|_| poisoned())?;
        Ok(guard
            .readings
            .iter()
            .filter(|r| r.plot_id == *plot_id)
            .max_by_key(|r| r.ts)
            .cloned())
    }
}

#[async_trait]
impl AlertSink for MemoryPlatform {
    async fn raise(&self, alert: Alert) -> Result<()> {
        let message = alert.message.trim();
        if message.is_empty() {
            return Err(ContractError::Validation("pesan peringatan kosong".into()));
        }
        if message.chars().count() > MAX_ALERT_MESSAGE_CHARS {
            return Err(ContractError::Validation(format!(
                "pesan peringatan lebih dari {MAX_ALERT_MESSAGE_CHARS} karakter"
            )));
        }
        let mut guard = self.alerts.lock().map_err(|_| poisoned())?;
        guard.push_back(alert);
        while guard.len() > self.max_alerts {
            guard.pop_front();
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
