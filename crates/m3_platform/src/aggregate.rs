use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use soil_contracts::{
    ContractError, Parameter, PlotId, QualityFlag, Result, SoilReading, TrendDirection,
};
use std::collections::BTreeMap;

// agregasi
#[derive(Debug, Clone, Serialize)]
pub struct Bucket {
    pub start: DateTime<Utc>,
    pub count: usize,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
}

// mengelompokkan data berdasarkan waktu
pub fn bucketize(series: &[(DateTime<Utc>, f32)], interval_secs: i64) -> Vec<Bucket> {
    let step = interval_secs.max(1);
    let mut groups: BTreeMap<i64, Vec<f32>> = BTreeMap::new();
    for (ts, value) in series {
        if value.is_finite() {
            groups
                .entry(ts.timestamp().div_euclid(step))
                .or_default()
                .push(*value);
        }
    }
    groups
        .into_iter()
        .filter_map(|(slot, values)| {
            let start = Utc.timestamp_opt(slot * step, 0).single()?;
            let count = values.len();
            Some(Bucket {
                start,
                count,
                min: values.iter().copied().fold(f32::INFINITY, f32::min),
                max: values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
                mean: values.iter().sum::<f32>() / count as f32,
            })
        })
        .collect()
}

// menghitung interval otomatis berdasarkan rentang waktu
pub fn auto_interval_secs(span_secs: i64) -> i64 {
    (span_secs / 120).max(60)
}

// membuat data lebih halus dengan rata-rata bergerak
pub fn moving_average(values: &[f32], window: usize) -> Vec<f32> {
    let w = window.max(1);
    (0..values.len())
        .map(|i| {
            let slice = &values[(i + 1).saturating_sub(w)..=i];
            slice.iter().sum::<f32>() / slice.len() as f32
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub first: f32,
    pub last: f32,
    pub delta: f32,
    pub percent: Option<f32>,
    pub direction: TrendDirection,
}

// melihat tren naik atau turun
pub fn period_change(buckets: &[Bucket]) -> Option<Change> {
    let first = buckets.first()?.mean;
    let last = buckets.last()?.mean;
    let delta = last - first;
    let tolerance = first.abs().max(1e-3) * 0.02; // perubahan < 2% dianggap stabil
    let direction = if delta.abs() <= tolerance {
        TrendDirection::Stable
    } else if delta > 0.0 {
        TrendDirection::Rising
    } else {
        TrendDirection::Falling
    };
    Some(Change {
        first,
        last,
        delta,
        percent: (first.abs() > 1e-6).then(|| delta / first.abs() * 100.0),
        direction,
    })
}

// kesehatan sensor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Online,
    Stale,
    Offline,
    NoData,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Health {
    pub status: HealthStatus,
    pub last_seen: Option<DateTime<Utc>>,
    pub sample_count: usize,
    pub unusable_ratio: f32,
    pub suspect_ratio: f32,
    pub gap_count: usize,
    pub score: f32,
}

// memeriksa kesehatan sensor
pub fn sensor_health(readings: &[SoilReading], now: DateTime<Utc>, expected_secs: i64) -> Health {
    let expected = expected_secs.max(1);
    let mut stamps: Vec<DateTime<Utc>> = readings.iter().map(|r| r.ts).collect();
    stamps.sort();
    let Some(&last) = stamps.last() else {
        return Health {
            status: HealthStatus::NoData,
            last_seen: None,
            sample_count: 0,
            unusable_ratio: 0.0,
            suspect_ratio: 0.0,
            gap_count: 0,
            score: 0.0,
        };
    };

    let n = readings.len() as f32;
    let unusable = readings.iter().filter(|r| !r.is_usable()).count() as f32 / n;
    let suspect = readings
        .iter()
        .filter(|r| r.quality == QualityFlag::Suspect)
        .count() as f32
        / n;
    let gap_count = stamps
        .windows(2)
        .filter(|w| (w[1] - w[0]).num_seconds() > 3 * expected)
        .count();

    let age = (now - last).num_seconds();
    let (status, availability) = if age <= 2 * expected {
        (HealthStatus::Online, 1.0)
    } else if age <= 6 * expected {
        (HealthStatus::Stale, 0.5)
    } else {
        (HealthStatus::Offline, 0.0)
    };
    let quality = (1.0 - unusable - 0.5 * suspect).clamp(0.0, 1.0);

    Health {
        status,
        last_seen: Some(last),
        sample_count: readings.len(),
        unusable_ratio: unusable,
        suspect_ratio: suspect,
        gap_count,
        score: quality * availability,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParamSummary {
    pub plot_id: PlotId,
    pub parameter: Parameter,
    pub count: usize,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub std_dev: f32,
}

// membuat ringkasan data
pub fn summarize(
    plot_id: PlotId,
    readings: &[SoilReading],
    parameter: Parameter,
) -> Option<ParamSummary> {
    let values: Vec<f32> = readings
        .iter()
        .filter(|r| r.is_usable())
        .map(|r| r.value(parameter))
        .filter(|v| v.is_finite())
        .collect();
    if values.is_empty() {
        return None;
    }
    let count = values.len();
    let mean = values.iter().sum::<f32>() / count as f32;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / count as f32;
    Some(ParamSummary {
        plot_id,
        parameter,
        count,
        min: values.iter().copied().fold(f32::INFINITY, f32::min),
        max: values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        mean,
        std_dev: variance.sqrt(),
    })
}

pub fn rank_by_mean(mut summaries: Vec<ParamSummary>) -> Vec<ParamSummary> {
    summaries.sort_by(|a, b| b.mean.total_cmp(&a.mean));
    summaries
}

// memeriksa apakah data sensor masuk akal
pub fn validate_reading(r: &SoilReading) -> Result<()> {
    let checks = [
        ("kelembapan", r.moisture_pct, 0.0, 100.0),
        ("suhu", r.temperature_c, -20.0, 80.0),
        ("pH", r.ph, 0.0, 14.0),
        ("EC", r.ec_ms_cm, 0.0, 50.0),
    ];
    for (name, value, lo, hi) in checks {
        if !value.is_finite() || value < lo || value > hi {
            return Err(ContractError::Validation(format!(
                "{name} di luar rentang {lo}..{hi}: {value}"
            )));
        }
    }
    Ok(())
}

// memeriksa integritas sebelum disimpan: nilai harus berhingga dan waktu tidak di masa depan
const MAX_CLOCK_SKEW_SECS: i64 = 300;

pub fn validate_integrity(r: &SoilReading, now: DateTime<Utc>) -> Result<()> {
    let values = [r.moisture_pct, r.temperature_c, r.ph, r.ec_ms_cm];
    if values.iter().any(|v| !v.is_finite()) {
        return Err(ContractError::Validation(
            "nilai sensor NaN atau tak hingga".into(),
        ));
    }
    if (r.ts - now).num_seconds() > MAX_CLOCK_SKEW_SECS {
        return Err(ContractError::Validation(
            "timestamp terlalu jauh di masa depan".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/aggregate.rs"]
mod tests;
